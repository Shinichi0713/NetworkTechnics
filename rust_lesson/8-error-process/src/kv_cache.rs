use candle_core::{DType, Device, IndexOp, Result, Tensor, D};
use candle_nn::{linear_no_bias, Linear, Module, VarBuilder};

pub struct KVCache {
    k: Option<Tensor>,
    v: Option<Tensor>,
}

impl KVCache {
    pub fn new() -> Self {
        Self { k: None, v: None }
    }

    /// 過去の Key/Value テンソルと新規の Key/Value テンソルをシーケンス軸（dim=2）で結合
    pub fn append(&mut self, k: &Tensor, v: &Tensor) -> Result<(Tensor, Tensor)> {
        let (new_k, new_v) = match (&self.k, &self.v) {
            (Some(past_k), Some(past_v)) => {
                let k = Tensor::cat(&[past_k, k], 2)?;
                let v = Tensor::cat(&[past_v, v], 2)?;
                (k, v)
            }
            _ => (k.clone(), v.clone()),
        };
        self.k = Some(new_k.clone());
        self.v = Some(new_v.clone());
        Ok((new_k, new_v))
    }
}

pub struct CausalSelfAttention {
    q_proj: Linear,
    k_proj: Linear,
    v_proj: Linear,
    out_proj: Linear,
    num_heads: usize,
    head_dim: usize,
}

impl CausalSelfAttention {
    pub fn new(hidden_size: usize, num_heads: usize, vb: VarBuilder) -> Result<Self> {
        let head_dim = hidden_size / num_heads;
        let q_proj = linear_no_bias(hidden_size, hidden_size, vb.pp("q_proj"))?;
        let k_proj = linear_no_bias(hidden_size, hidden_size, vb.pp("k_proj"))?;
        let v_proj = linear_no_bias(hidden_size, hidden_size, vb.pp("v_proj"))?;
        let out_proj = linear_no_bias(hidden_size, hidden_size, vb.pp("out_proj"))?;

        Ok(Self {
            q_proj,
            k_proj,
            v_proj,
            out_proj,
            num_heads,
            head_dim,
        })
    }

    pub fn forward(
        &self,
        x: &Tensor,
        cache: &mut KVCache,
    ) -> Result<Tensor> {
        let (b_sz, seq_len, _hidden_size) = x.dims3()?;

        // 1. Q, K, V プロジェクション
        let q = self.q_proj.forward(x)?;
        let k = self.k_proj.forward(x)?;
        let v = self.v_proj.forward(x)?;

        // 2. 形状変換: [batch, seq_len, num_heads, head_dim] -> [batch, num_heads, seq_len, head_dim]
        let q = q.reshape((b_sz, seq_len, self.num_heads, self.head_dim))?.transpose(1, 2)?;
        let k = k.reshape((b_sz, seq_len, self.num_heads, self.head_dim))?.transpose(1, 2)?;
        let v = v.reshape((b_sz, seq_len, self.num_heads, self.head_dim))?.transpose(1, 2)?;

        // 3. KV キャッシュの結合 (過去の K, V と新規の K, V を連結)
        let (k, v) = cache.append(&k, &v)?;

        // 4. Scaled Dot-Product Attention
        let total_seq_len = k.dim(2)?;
        let scale = 1.0 / (self.head_dim as f64).sqrt();
        let mut attn_weights = (q.matmul(&k.t()?)? * scale)?;

        // Causal Mask (Prefill 段階などで seq_len > 1 の場合のみ適用)
        if seq_len > 1 {
            let mask = self.make_causal_mask(seq_len, total_seq_len, x.device())?;
            attn_weights = attn_weights.broadcast_add(&mask)?;
        }

        let attn_weights = candle_nn::ops::softmax_last_dim(&attn_weights)?;
        let attn_output = attn_weights.matmul(&v)?;

        // 5. 元の形状に復元して Output プロジェクション
        let attn_output = attn_output.transpose(1, 2)?.reshape((b_sz, seq_len, () ))?;
        self.out_proj.forward(&attn_output)
    }

    fn make_causal_mask(&self, seq_len: usize, total_seq_len: usize, device: &Device) -> Result<Tensor> {
        let mut mask = vec![0.0f32; seq_len * total_seq_len];
        for i in 0..seq_len {
            for j in 0..total_seq_len {
                if j > i + (total_seq_len - seq_len) {
                    mask[i * total_seq_len + j] = f32::NEG_INFINITY;
                }
            }
        }
        Tensor::from_vec(mask, (1, 1, seq_len, total_seq_len), device)
    }
}