use ndarray::{Array1, Array2, Array3, ArrayView1, ArrayView2, Axis};
use rayon::prelude::*;
use safetensors::SafeTensors;
use std::fs::File;
use std::sync::Arc;

// Qwen2.5-0.5B パラメータ定数
const VOCAB_SIZE: usize = 151936;
const HIDDEN_SIZE: usize = 896;
const INTERMEDIATE_SIZE: usize = 4864;
const NUM_LAYERS: usize = 24;
const NUM_HEADS: usize = 14;
const NUM_KV_HEADS: usize = 2;
const HEAD_DIM: usize = HIDDEN_SIZE / NUM_HEADS; // 64
const NUM_KV_GROUPS: usize = NUM_HEADS / NUM_KV_HEADS; // 7
const EPS: f32 = 1e-6;

// 行列演算ユーティリティ: C = A @ B^T
fn matmul_transpose_b(a: &Array2<f32>, b: &Array2<f32>) -> Array2<f32> {
    let (m, k1) = (a.nrows(), a.ncols());
    let (n, k2) = (b.nrows(), b.ncols());
    assert_eq!(k1, k2, "Matrix shape mismatch");

    let mut c = Array2::<f32>::zeros((m, n));
    c.axis_iter_mut(Axis(0))
        .into_par_iter()
        .enumerate()
        .for_each(|(i, mut row)| {
            let a_row = a.row(i);
            for j in 0..n {
                let b_row = b.row(j);
                let mut sum = 0.0f32;
                for k in 0..k1 {
                    sum += a_row[k] * b_row[k];
                }
                row[j] = sum;
            }
        });
    c
}

// 1. RMSNorm
pub struct RMSNorm {
    pub weight: Array1<f32>,
    pub eps: f32,
}

impl RMSNorm {
    pub fn forward(&self, x: &Array2<f32>) -> Array2<f32> {
        let mut out = x.clone();
        out.axis_iter_mut(Axis(0)).for_each(|mut row| {
            let var: f32 = row.iter().map(|v| v * v).sum::<f32>() / (row.len() as f32);
            let scale = 1.0 / (var + self.eps).sqrt();
            for (val, &w) in row.iter_mut().zip(self.weight.iter()) {
                *val = (*val * scale) * w;
            }
        });
        out
    }
}

// 2. Rotary Position Embedding (RoPE)
pub struct RotaryEmbedding {
    cos_cached: Array2<f32>,
    sin_cached: Array2<f32>,
}

impl RotaryEmbedding {
    pub fn new(max_seq_len: usize, head_dim: usize, base: f32) -> Self {
        let half_dim = head_dim / 2;
        let mut inv_freq = Vec::with_capacity(half_dim);
        for i in (0..head_dim).step_by(2) {
            inv_freq.push(1.0 / base.powf(i as f32 / head_dim as f32));
        }

        let mut cos_cached = Array2::<f32>::zeros((max_seq_len, head_dim));
        let mut sin_cached = Array2::<f32>::zeros((max_seq_len, head_dim));

        for pos in 0..max_seq_len {
            for i in 0..half_dim {
                let freq = pos as f32 * inv_freq[i];
                let c = freq.cos();
                let s = freq.sin();
                cos_cached[[pos, i]] = c;
                cos_cached[[pos, i + half_dim]] = c;
                sin_cached[[pos, i]] = s;
                sin_cached[[pos, i + half_dim]] = s;
            }
        }
        Self { cos_cached, sin_cached }
    }

    pub fn apply(&self, x: &mut Array3<f32>, pos_offset: usize) {
        // x: [num_heads, seq_len, head_dim]
        let seq_len = x.shape()[1];
        let half_dim = HEAD_DIM / 2;

        for h in 0..x.shape()[0] {
            for t in 0..seq_len {
                let pos = pos_offset + t;
                for i in 0..half_dim {
                    let q1 = x[[h, t, i]];
                    let q2 = x[[h, t, i + half_dim]];
                    let c = self.cos_cached[[pos, i]];
                    let s = self.sin_cached[[pos, i]];

                    x[[h, t, i]] = q1 * c - q2 * s;
                    x[[h, t, i + half_dim]] = q1 * s + q2 * c;
                }
            }
        }
    }
}

// 3. Grouped-Query Attention (GQA) & KV-Cache
pub struct Attention {
    pub q_proj: Array2<f32>,
    pub q_bias: Array1<f32>,
    pub k_proj: Array2<f32>,
    pub k_bias: Array1<f32>,
    pub v_proj: Array2<f32>,
    pub v_bias: Array1<f32>,
    pub o_proj: Array2<f32>,
    // KV Cache [heads, seq_len, head_dim]
    pub k_cache: Array3<f32>,
    pub v_cache: Array3<f32>,
}

impl Attention {
    pub fn forward(&mut self, x: &Array2<f32>, rope: &RotaryEmbedding, pos: usize) -> Array2<f32> {
        let seq_len = x.nrows();
        
        // 1. Projection
        let mut q = matmul_transpose_b(x, &self.q_proj);
        let mut k = matmul_transpose_b(x, &self.k_proj);
        let mut v = matmul_transpose_b(x, &self.v_proj);

        // Add Bias
        q.axis_iter_mut(Axis(0)).for_each(|mut r| r += &self.q_bias);
        k.axis_iter_mut(Axis(0)).for_each(|mut r| r += &self.k_bias);
        v.axis_iter_mut(Axis(0)).for_each(|mut r| r += &self.v_bias);

        // 2. Reshape to Multi-Head: [heads, seq_len, head_dim]
        let mut q_heads = Array3::<f32>::zeros((NUM_HEADS, seq_len, HEAD_DIM));
        let mut k_heads = Array3::<f32>::zeros((NUM_KV_HEADS, seq_len, HEAD_DIM));
        let mut v_heads = Array3::<f32>::zeros((NUM_KV_HEADS, seq_len, HEAD_DIM));

        for t in 0..seq_len {
            for h in 0..NUM_HEADS {
                let slice = q.slice(ndarray::s![t, h * HEAD_DIM..(h + 1) * HEAD_DIM]);
                q_heads.slice_mut(ndarray::s![h, t, ..]).assign(&slice);
            }
            for h in 0..NUM_KV_HEADS {
                let slice_k = k.slice(ndarray::s![t, h * HEAD_DIM..(h + 1) * HEAD_DIM]);
                let slice_v = v.slice(ndarray::s![t, h * HEAD_DIM..(h + 1) * HEAD_DIM]);
                k_heads.slice_mut(ndarray::s![h, t, ..]).assign(&slice_k);
                v_heads.slice_mut(ndarray::s![h, t, ..]).assign(&slice_v);
            }
        }

        // 3. Apply RoPE
        rope.apply(&mut q_heads, pos);
        rope.apply(&mut k_heads, pos);

        // 4. Update KV Cache (Simple concatenation simulation)
        // ここでは1ステップ推論の動的計算を実行
        let scale = 1.0 / (HEAD_DIM as f32).sqrt();
        let mut attn_out_heads = Array3::<f32>::zeros((NUM_HEADS, seq_len, HEAD_DIM));

        for h in 0..NUM_HEADS {
            let kv_h = h / NUM_KV_GROUPS; // GQA Mapping
            for t_q in 0..seq_len {
                // Scaled Dot-Product Attention
                let mut scores = Vec::with_capacity(seq_len);
                let mut max_score = f32::NEG_INFINITY;
                for t_k in 0..seq_len {
                    if t_k > t_q {
                        scores.push(f32::NEG_INFINITY); // Causal Mask
                        continue;
                    }
                    let mut dot = 0.0f32;
                    for d in 0..HEAD_DIM {
                        dot += q_heads[[h, t_q, d]] * k_heads[[kv_h, t_k, d]];
                    }
                    dot *= scale;
                    if dot > max_score { max_score = dot; }
                    scores.push(dot);
                }

                // Softmax
                let mut sum_exp = 0.0f32;
                for score in scores.iter_mut() {
                    *score = (*score - max_score).exp();
                    sum_exp += *score;
                }
                for score in scores.iter_mut() {
                    *score /= sum_exp;
                }

                // Weighted Sum of V
                for d in 0..HEAD_DIM {
                    let mut val = 0.0f32;
                    for t_k in 0..seq_len {
                        if scores[t_k] > 0.0 {
                            val += scores[t_k] * v_heads[[kv_h, t_k, d]];
                        }
                    }
                    attn_out_heads[[h, t_q, d]] = val;
                }
            }
        }

        // 5. Concatenate Heads & Output Projection
        let mut concat_attn = Array2::<f32>::zeros((seq_len, HIDDEN_SIZE));
        for t in 0..seq_len {
            for h in 0..NUM_HEADS {
                let slice = attn_out_heads.slice(ndarray::s![h, t, ..]);
                concat_attn.slice_mut(ndarray::s![t, h * HEAD_DIM..(h + 1) * HEAD_DIM]).assign(&slice);
            }
        }

        matmul_transpose_b(&concat_attn, &self.o_proj)
    }
}

// 4. SwiGLU MLP
pub struct SwiGLU {
    pub gate_proj: Array2<f32>,
    pub up_proj: Array2<f32>,
    pub down_proj: Array2<f32>,
}

impl SwiGLU {
    pub fn forward(&self, x: &Array2<f32>) -> Array2<f32> {
        let gate = matmul_transpose_b(x, &self.gate_proj);
        let up = matmul_transpose_b(x, &self.up_proj);

        // SiLU(gate) * up
        let mut intermediate = Array2::<f32>::zeros(gate.dim());
        gate.axis_iter(Axis(0))
            .zip(up.axis_iter(Axis(0)))
            .zip(intermediate.axis_iter_mut(Axis(0)))
            .into_par_iter()
            .for_each(|((g_row, u_row), mut out_row)| {
                for i in 0..g_row.len() {
                    let silu = g_row[i] / (1.0 + (-g_row[i]).exp());
                    out_row[i] = silu * u_row[i];
                }
            });

        matmul_transpose_b(&intermediate, &self.down_proj)
    }
}

// 5. Transformer Decoder Layer
pub struct DecoderLayer {
    pub input_layernorm: RMSNorm,
    pub self_attn: Attention,
    pub post_attention_layernorm: RMSNorm,
    pub mlp: SwiGLU,
}

impl DecoderLayer {
    pub fn forward(&mut self, x: &Array2<f32>, rope: &RotaryEmbedding, pos: usize) -> Array2<f32> {
        // Residual 1
        let norm_x = self.input_layernorm.forward(x);
        let attn_out = self.self_attn.forward(&norm_x, rope, pos);
        let x = x + &attn_out;

        // Residual 2
        let norm_x = self.post_attention_layernorm.forward(&x);
        let mlp_out = self.mlp.forward(&norm_x);
        x + &mlp_out
    }
}

// 6. Qwen2 推論エンジン本体
pub struct Qwen2Engine {
    pub embed_tokens: Array2<f32>,
    pub layers: Vec<DecoderLayer>,
    pub norm: RMSNorm,
    pub lm_head: Array2<f32>,
    pub rope: RotaryEmbedding,
}

impl Qwen2Engine {
    pub fn forward(&mut self, input_ids: &[usize]) -> Array2<f32> {
        let seq_len = input_ids.len();
        let mut hidden_states = Array2::<f32>::zeros((seq_len, HIDDEN_SIZE));

        // Embedding Lookup
        for (t, &id) in input_ids.iter().enumerate() {
            hidden_states.row_mut(t).assign(&self.embed_tokens.row(id));
        }

        // Pass Layer Stack
        for layer in self.layers.iter_mut() {
            hidden_states = layer.forward(&hidden_states, &self.rope, 0);
        }

        // Final Norm
        hidden_states = self.norm.forward(&hidden_states);

        // LM Head Projection (最後のトークンのロジットを計算)
        let last_hidden = hidden_states.slice(ndarray::s![seq_len - 1..seq_len, ..]).to_shared();
        matmul_transpose_b(&last_hidden, &self.lm_head)
    }
}

fn main() {
    println!("=== Qwen2.5-0.5B Rust Inference Engine Initialization ===");

    // ダミーのモデル初期化（本来は safetensors からロード）
    let rope = RotaryEmbedding::new(32768, HEAD_DIM, 1000000.0);
    let embed_tokens = Array2::<f32>::zeros((VOCAB_SIZE, HIDDEN_SIZE));
    let lm_head = Array2::<f32>::zeros((VOCAB_SIZE, HIDDEN_SIZE));
    let norm = RMSNorm { weight: Array1::ones(HIDDEN_SIZE), eps: EPS };

    let mut layers = Vec::with_capacity(NUM_LAYERS);
    for _ in 0..NUM_LAYERS {
        layers.push(DecoderLayer {
            input_layernorm: RMSNorm { weight: Array1::ones(HIDDEN_SIZE), eps: EPS },
            self_attn: Attention {
                q_proj: Array2::zeros((HIDDEN_SIZE, HIDDEN_SIZE)),
                q_bias: Array1::zeros(HIDDEN_SIZE),
                k_proj: Array2::zeros((NUM_KV_HEADS * HEAD_DIM, HIDDEN_SIZE)),
                k_bias: Array1::zeros(NUM_KV_HEADS * HEAD_DIM),
                v_proj: Array2::zeros((NUM_KV_HEADS * HEAD_DIM, HIDDEN_SIZE)),
                v_bias: Array1::zeros(NUM_KV_HEADS * HEAD_DIM),
                o_proj: Array2::zeros((HIDDEN_SIZE, HIDDEN_SIZE)),
                k_cache: Array3::zeros((NUM_KV_HEADS, 0, HEAD_DIM)),
                v_cache: Array3::zeros((NUM_KV_HEADS, 0, HEAD_DIM)),
            },
            post_attention_layernorm: RMSNorm { weight: Array1::ones(HIDDEN_SIZE), eps: EPS },
            mlp: SwiGLU {
                gate_proj: Array2::zeros((INTERMEDIATE_SIZE, HIDDEN_SIZE)),
                up_proj: Array2::zeros((INTERMEDIATE_SIZE, HIDDEN_SIZE)),
                down_proj: Array2::zeros((HIDDEN_SIZE, INTERMEDIATE_SIZE)),
            },
        });
    }

    let mut engine = Qwen2Engine { embed_tokens, layers, norm, lm_head, rope };

    // テストプロンプト（トークンID列の入力）
    let input_ids = vec![151644, 872, 198, 2610, 525, 264, 10950]; // ダミーのトークン系列
    println!("Running forward pass for {} tokens...", input_ids.len());

    let logits = engine.forward(&input_ids);

    println!("Success! Output Logits Shape: ({}, {})", logits.nrows(), logits.ncols());
    println!("Sample Logit [0, 0]: {}", logits[[0, 0]]);
}