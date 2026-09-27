use tch::{nn, nn::Module, nn::Path, Device, Kind, Tensor};

// ----------------------------------------------------------------------
// 1. Qwen2 RMSNorm
// ----------------------------------------------------------------------
#[derive(Debug)]
struct Qwen2RMSNorm {
    weight: Tensor,
    eps: f64,
}

impl Qwen2RMSNorm {
    fn new(vs: Path, hidden_size: i64, eps: f64) -> Self {
        let weight = vs.var("weight", &[hidden_size], nn::Init::Const(1.0));
        Self { weight, eps }
    }

    fn forward(&self, xs: &Tensor) -> Tensor {
        let input_kind = xs.kind();
        let xs_f32 = xs.to_kind(Kind::Float);
        let variance = xs_f32.pow_tensor_scalar(2).mean_dim(&[-1i64][..], true, Kind::Float);
        let norm_xs = &xs_f32 * (variance + self.eps).rsqrt();
        (&self.weight * norm_xs).to_kind(input_kind)
    }
}

// ----------------------------------------------------------------------
// 2. Rotary Position Embedding (RoPE)
// ----------------------------------------------------------------------
#[derive(Debug)]
struct Qwen2RotaryEmbedding {
    inv_freq: Tensor,
}

impl Qwen2RotaryEmbedding {
    fn new(dim: i64, base: f64, device: Device) -> Self {
        let torch_arange = Tensor::arange_step(0, dim, 2, (Kind::Float, device));
        let inv_freq = 1.0 / Tensor::pow_tensor_scalar(&Tensor::from(base), &(torch_arange / (dim as f64)));
        Self { inv_freq }
    }

    fn forward(&self, seq_len: i64, device: Device) -> (Tensor, Tensor) {
        let t = Tensor::arange(seq_len, (Kind::Float, device));
        let freqs = t.outer(&self.inv_freq);
        let emb = Tensor::cat(&[&freqs, &freqs], -1);
        (emb.cos(), emb.sin())
    }
}

fn rotate_half(x: &Tensor) -> Tensor {
    let half_dim = x.size().last().unwrap() / 2;
    let x1 = x.slice(-1, 0, half_dim, 1);
    let x2 = x.slice(-1, half_dim, x.size().last().unwrap() * 2, 1);
    Tensor::cat(&[&-x2, &x1], -1)
}

fn apply_rotary_pos_emb(q: &Tensor, k: &Tensor, cos: &Tensor, sin: &Tensor) -> (Tensor, Tensor) {
    let cos_exp = cos.unsqueeze(0).unsqueeze(1);
    let sin_exp = sin.unsqueeze(0).unsqueeze(1);
    let q_embed = (q * &cos_exp) + (rotate_half(q) * &sin_exp);
    let k_embed = (k * &cos_exp) + (rotate_half(k) * &sin_exp);
    (q_embed, k_embed)
}

// ----------------------------------------------------------------------
// 3. Grouped-Query Attention (GQA)
// ----------------------------------------------------------------------
#[derive(Debug)]
struct Qwen2Attention {
    q_proj: nn::Linear,
    k_proj: nn::Linear,
    v_proj: nn::Linear,
    o_proj: nn::Linear,
    num_heads: i64,
    num_key_value_heads: i64,
    head_dim: i64,
    num_key_value_groups: i64,
    hidden_size: i64,
}

impl Qwen2Attention {
    fn new(vs: Path, hidden_size: i64, num_heads: i64, num_key_value_heads: i64) -> Self {
        let head_dim = hidden_size / num_heads;
        let num_key_value_groups = num_heads / num_key_value_heads;

        // Qwen 特有の QKV Bias=True 設定
        let c_bias = nn::LinearConfig { bias: true, ..Default::default() };
        let c_no_bias = nn::LinearConfig { bias: false, ..Default::default() };

        let q_proj = nn::linear(&vs / "q_proj", hidden_size, num_heads * head_dim, c_bias);
        let k_proj = nn::linear(&vs / "k_proj", hidden_size, num_key_value_heads * head_dim, c_bias);
        let v_proj = nn::linear(&vs / "v_proj", hidden_size, num_key_value_heads * head_dim, c_bias);
        let o_proj = nn::linear(&vs / "o_proj", num_heads * head_dim, hidden_size, c_no_bias);

        Self {
            q_proj,
            k_proj,
            v_proj,
            o_proj,
            num_heads,
            num_key_value_heads,
            head_dim,
            num_key_value_groups,
            hidden_size,
        }
    }

    fn forward(&self, xs: &Tensor, rotary_emb: &Qwen2RotaryEmbedding) -> Tensor {
        let (bsz, q_len, _) = xs.size3().unwrap();

        let q = self.q_proj.forward(xs)
            .view([bsz, q_len, self.num_heads, self.head_dim])
            .transpose(1, 2);
        let k = self.k_proj.forward(xs)
            .view([bsz, q_len, self.num_key_value_heads, self.head_dim])
            .transpose(1, 2);
        let v = self.v_proj.forward(xs)
            .view([bsz, q_len, self.num_key_value_heads, self.head_dim])
            .transpose(1, 2);

        let (cos, sin) = rotary_emb.forward(q_len, xs.device());
        let (q_emb, mut k_emb) = apply_rotary_pos_emb(&q, &k, &cos, &sin);
        let mut v_emb = v;

        if self.num_key_value_groups > 1 {
            k_emb = k_emb.repeat_interleave(self.num_key_value_groups, 1);
            v_emb = v_emb.repeat_interleave(self.num_key_value_groups, 1);
        }

        let scale = 1.0 / (self.head_dim as f64).sqrt();
        let scores = q_emb.matmul(&k_emb.transpose(-2, -1)) * scale;

        // Causal Mask
        let causal_mask = Tensor::full(&[q_len, q_len], -1e9, (Kind::Float, xs.device())).triu(1);
        let scores = scores + causal_mask.unsqueeze(0).unsqueeze(0);

        let attn_weights = scores.softmax(-1, Kind::Float);
        let attn_output = attn_weights.matmul(&v_emb);

        let attn_output = attn_output
            .transpose(1, 2)
            .contiguous()
            .view([bsz, q_len, self.hidden_size]);

        self.o_proj.forward(&attn_output)
    }
}

// ----------------------------------------------------------------------
// 4. SwiGLU MLP
// ----------------------------------------------------------------------
#[derive(Debug)]
struct Qwen2MLP {
    gate_proj: nn::Linear,
    up_proj: nn::Linear,
    down_proj: nn::Linear,
}

impl Qwen2MLP {
    fn new(vs: Path, hidden_size: i64, intermediate_size: i64) -> Self {
        let c = nn::LinearConfig { bias: false, ..Default::default() };
        let gate_proj = nn::linear(&vs / "gate_proj", hidden_size, intermediate_size, c);
        let up_proj = nn::linear(&vs / "up_proj", hidden_size, intermediate_size, c);
        let down_proj = nn::linear(&vs / "down_proj", intermediate_size, hidden_size, c);
        Self { gate_proj, up_proj, down_proj }
    }

    fn forward(&self, xs: &Tensor) -> Tensor {
        let lhs = self.gate_proj.forward(xs).silu();
        let rhs = self.up_proj.forward(xs);
        self.down_proj.forward(&(lhs * rhs))
    }
}

// ----------------------------------------------------------------------
// 5. Decoder Layer
// ----------------------------------------------------------------------
#[derive(Debug)]
struct Qwen2DecoderLayer {
    input_layernorm: Qwen2RMSNorm,
    self_attn: Qwen2Attention,
    post_attention_layernorm: Qwen2RMSNorm,
    mlp: Qwen2MLP,
}

impl Qwen2DecoderLayer {
    fn new(vs: Path, hidden_size: i64, num_heads: i64, num_kv_heads: i64, intermediate_size: i64) -> Self {
        let input_layernorm = Qwen2RMSNorm::new(&vs / "input_layernorm", hidden_size, 1e-6);
        let self_attn = Qwen2Attention::new(&vs / "self_attn", hidden_size, num_heads, num_kv_heads);
        let post_attention_layernorm = Qwen2RMSNorm::new(&vs / "post_attention_layernorm", hidden_size, 1e-6);
        let mlp = Qwen2MLP::new(&vs / "mlp", hidden_size, intermediate_size);

        Self {
            input_layernorm,
            self_attn,
            post_attention_layernorm,
            mlp,
        }
    }

    fn forward(&self, xs: &Tensor, rotary_emb: &Qwen2RotaryEmbedding) -> Tensor {
        let residual = xs;
        let hidden = self.input_layernorm.forward(xs);
        let hidden = self.self_attn.forward(&hidden, rotary_emb);
        let hidden = residual + hidden;

        let residual = &hidden;
        let hidden_norm = self.post_attention_layernorm.forward(&hidden);
        let hidden_mlp = self.mlp.forward(&hidden_norm);
        residual + hidden_mlp
    }
}

// ----------------------------------------------------------------------
// 6. Full Qwen2 Model
// ----------------------------------------------------------------------
#[derive(Debug)]
struct Qwen2ForCausalLM {
    embed_tokens: nn::Embedding,
    rotary_emb: Qwen2RotaryEmbedding,
    layers: Vec<Qwen2DecoderLayer>,
    norm: Qwen2RMSNorm,
    lm_head: nn::Linear,
}

impl Qwen2ForCausalLM {
    fn new(
        vs: Path,
        vocab_size: i64,
        hidden_size: i64,
        num_layers: i64,
        num_heads: i64,
        num_kv_heads: i64,
        intermediate_size: i64,
    ) -> Self {
        let embed_tokens = nn::embedding(&vs / "embed_tokens", vocab_size, hidden_size, Default::default());
        let rotary_emb = Qwen2RotaryEmbedding::new(hidden_size / num_heads, 10000.0, vs.device());

        let mut layers = Vec::new();
        let layers_path = &vs / "layers";
        for i in 0..num_layers {
            layers.push(Qwen2DecoderLayer::new(
                &layers_path / i,
                hidden_size,
                num_heads,
                num_kv_heads,
                intermediate_size,
            ));
        }

        let norm = Qwen2RMSNorm::new(&vs / "norm", hidden_size, 1e-6);
        let lm_head = nn::linear(&vs / "lm_head", hidden_size, vocab_size, nn::LinearConfig { bias: false, ..Default::default() });

        Self {
            embed_tokens,
            rotary_emb,
            layers,
            norm,
            lm_head,
        }
    }

    fn forward(&self, input_ids: &Tensor) -> Tensor {
        let mut hidden_states = self.embed_tokens.forward(input_ids);

        for layer in &self.layers {
            hidden_states = layer.forward(&hidden_states, &self.rotary_emb);
        }

        hidden_states = self.norm.forward(&hidden_states);
        self.lm_head.forward(&hidden_states)
    }
}

// ----------------------------------------------------------------------
// 7. エントリポイント
// ----------------------------------------------------------------------
fn main() {
    tch::maybe_init();
    let device = Device::cuda_if_available();
    let vs = nn::VarStore::new(device);

    let vocab_size = 151936;
    let hidden_size = 512;
    let num_layers = 4;
    let num_heads = 8;
    let num_kv_heads = 2; // GQA (Query: 8, KV: 2)
    let intermediate_size = 2048;

    let model = Qwen2ForCausalLM::new(
        vs.root(),
        vocab_size,
        hidden_size,
        num_layers,
        num_heads,
        num_kv_heads,
        intermediate_size,
    );

    // ダミー入力 (Batch: 2, SeqLen: 16)
    let batch_size = 2;
    let seq_len = 16;
    let input_ids = Tensor::randint(1000, &[batch_size, seq_len], (Kind::Int64, device));

    let logits = model.forward(&input_ids);

    println!("Input shape  : {:?}", input_ids.size());
    println!("Logits shape : {:?}", logits.size());
}

use tch::{nn, nn::Module, nn::OptimizerConfig, Device, Kind, Tensor};

// --- (前述の Qwen2RMSNorm, Qwen2RotaryEmbedding, Qwen2Attention, Qwen2MLP, Qwen2DecoderLayer, Qwen2ForCausalLM の定義) ---

fn train() -> anyhow::Result<()> {
    tch::maybe_init();
    let device = Device::cuda_if_available();
    println!("Using device: {:?}", device);

    // 1. ハイパーパラメータ設定
    let vocab_size = 1000;      // 語彙数（ダミー設定）
    let hidden_size = 256;      // 隠れ層の次元数
    let num_layers = 4;         // デコーダ層数
    let num_heads = 8;          // Query ヘッド数
    let num_kv_heads = 2;       // KV ヘッド数 (GQA)
    let intermediate_size = 1024;

    let batch_size = 4;
    let seq_len = 32;
    let epochs = 10;
    let learning_rate = 3e-4;
    let max_grad_norm = 1.0;

    // 2. Variable Store とモデルの構築
    let mut vs = nn::VarStore::new(device);
    let model = Qwen2ForCausalLM::new(
        vs.root(),
        vocab_size,
        hidden_size,
        num_layers,
        num_heads,
        num_kv_heads,
        intermediate_size,
    );

    // 3. オプティマイザの設定 (AdamW)
    let mut opt = nn::AdamW::default().build(&vs, learning_rate)?;

    // 4. 学習ループ
    println!("Starting training...");
    for epoch in 1..=epochs {
        // --- ダミーデータ作成 ---
        // input_ids: [B, S]
        let input_ids = Tensor::randint(vocab_size, &[batch_size, seq_len], (Kind::Int64, device));
        
        // Causal LM ターゲット: 入力を 1 トークンシフト [B, S-1]
        // input_seq : 0 ~ S-2
        // target_seq: 1 ~ S-1
        let input_seq = input_ids.slice(1, 0, seq_len - 1, 1);
        let target_seq = input_ids.slice(1, 1, seq_len, 1);

        // --- 順伝播 ---
        // logits: [B, S-1, Vocab]
        let logits = model.forward(&input_seq);

        // --- Cross Entropy Loss 計算 ---
        // Loss 計算のために形状を平坦化:
        // logits  -> [B * (S-1), Vocab]
        // targets -> [B * (S-1)]
        let logits_flat = logits.view(&[-1, vocab_size]);
        let targets_flat = target_seq.view(&[-1]);

        let loss = logits_flat.cross_entropy_for_logits(&targets_flat);

        // --- 逆伝播と最適化 ---
        opt.zero_grad();
        loss.backward();

        // 勾配クリッピング (Gradient Clipping)
        vs.clip_grad_norm(max_grad_norm);

        opt.step();

        // ロス出力
        let loss_val: f64 = loss.double_value();
        println!("Epoch: {:2}/{} | Loss: {:.4}", epoch, epochs, loss_val);
    }

    println!("Training completed.");
    
    // 5. 学習済みチェックポイントの保存
    vs.save("qwen2_model.ot")?;
    println!("Model saved to qwen2_model.ot");

    Ok(())
}