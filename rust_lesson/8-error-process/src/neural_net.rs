use std::cmp::Ordering;

// ============================================================================
// 0. 外部依存なしの高品質疑似乱数生成器 (Xorshift128+)
// ============================================================================

struct SimpleRng {
    s: [u64; 2],
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        let mut rng = SimpleRng { s: [seed, seed.wrapping_add(0x9E3779B97F4A7C15)] };
        if rng.s[0] == 0 && rng.s[1] == 0 {
            rng.s[0] = 1;
        }
        // 初期状態を撹拌
        for _ in 0..10 {
            rng.next_u64();
        }
        rng
    }

    fn next_u64(&mut mut self) -> u64 {
        let mut x = self.s[0];
        let y = self.s[1];
        self.s[0] = y;
        x ^= x << 23;
        self.s[1] = x ^ y ^ (x >> 17) ^ (y >> 26);
        self.s[1].wrapping_add(y)
    }

    // 0.0 .. 1.0 の一様分布
    fn gen_f32(&mut self) -> f32 {
        (self.next_u64() >> 11) as f32 / (1u64 << 53) as f32
    }

    // Box-Muller 変換による標準正規分布 (平均 0.0, 標準偏差 stddev)
    fn gen_normal(&mut self, mean: f32, stddev: f32) -> f32 {
        let u1 = self.gen_f32().max(1e-10);
        let u2 = self.gen_f32();
        let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos();
        mean + z0 * stddev
    }
}

// ============================================================================
// 1. 高精度数値演算 & 安定化アテンション基本関数
// ============================================================================

// SiLU (Swish) 活性化関数: x * sigmoid(x)
#[inline]
fn silu(x: f32) -> f32 {
    x / (1.0 + (-x).exp())
}

// 数値的に安定した Softmax（log-sum-exp トリックによるアンダーフロー防止）
fn safe_softmax_inplace(x: &mut [f32]) {
    if x.is_empty() { return; }
    let max_val = x.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let mut sum = 0.0f32;
    for val in x.iter_mut() {
        *val = (*val - max_val).exp();
        sum += *val;
    }
    let inv_sum = if sum > 0.0 { 1.0 / sum } else { 0.0 };
    for val in x.iter_mut() {
        *val *= inv_sum;
    }
}

// 高精度 Row-Major 行列積演算 Y = X * W
// X: [N x in_f], W: [in_f x out_f], Y: [N x out_f]
fn matmul_cpu_high_precision(x: &[f32], w: &[f32], y: &mut [f32], n: usize, in_f: usize, out_f: usize) {
    for i in 0..n {
        let x_row = &x[i * in_f..(i + 1) * in_f];
        let y_row = &mut y[i * out_f..(i + 1) * out_f];
        for j in 0..out_f {
            // アキュムレータを f64 にすることで精度劣化を防ぐ
            let mut sum = 0.0f64;
            for k in 0..in_f {
                sum += (x_row[k] as f64) * (w[k * out_f + j] as f64);
            }
            y_row[j] = sum as f32;
        }
    }
}

// ============================================================================
// 2. 高精度ニューラルネットワーク基本層
// ============================================================================

// 線形層 (Linear Layer)
struct Linear {
    in_features: usize,
    out_features: usize,
    weight: Vec<f32>, // [in_features x out_features]
}

impl Linear {
    fn new(in_f: usize, out_f: usize, rng: &mut SimpleRng) -> Self {
        let stddev = (2.0 / in_f as f32).sqrt();
        let mut weight = vec![0.0; in_f * out_f];
        for w in weight.iter_mut() {
            *w = rng.gen_normal(0.0, stddev);
        }
        Linear { in_features: in_f, out_features: out_f, weight }
    }

    fn forward(&self, input: &[f32], output: &mut [f32], n: usize) {
        matmul_cpu_high_precision(input, &self.weight, output, n, self.in_features, self.out_features);
    }
}

// RMSNorm (Root Mean Square Normalization)
struct RMSNorm {
    dim: usize,
    weight: Vec<f32>, // Gamma パラメータ
    eps: f32,
}

impl RMSNorm {
    fn new(dim: usize, eps: f32) -> Self {
        RMSNorm {
            dim,
            weight: vec![1.0; dim],
            eps,
        }
    }

    fn forward(&self, input: &[f32], output: &mut [f32], n: usize) {
        for i in 0..n {
            let in_ptr = &input[i * self.dim..(i + 1) * self.dim];
            let out_ptr = &mut output[i * self.dim..(i + 1) * self.dim];

            let mut square_sum = 0.0f64;
            for &val in in_ptr.iter() {
                square_sum += (val as f64) * (val as f64);
            }
            let rms = ((square_sum / self.dim as f64) + self.eps as f64).sqrt() as f32;
            let inv_rms = 1.0 / rms;

            for j in 0..self.dim {
                out_ptr[j] = (in_ptr[j] * inv_rms) * self.weight[j];
            }
        }
    }
}

// SwiGLU Feed-Forward Network
struct SwiGLUFFN {
    gate_proj: Linear,
    up_proj: Linear,
    down_proj: Linear,
}

impl SwiGLUFFN {
    fn new(d_model: usize, hidden_dim: usize, rng: &mut SimpleRng) -> Self {
        SwiGLUFFN {
            gate_proj: Linear::new(d_model, hidden_dim, rng),
            up_proj: Linear::new(d_model, hidden_dim, rng),
            down_proj: Linear::new(hidden_dim, d_model, rng),
        }
    }

    fn forward(&self, input: &[f32], output: &mut [f32], seq_len: usize) {
        let hidden_dim = self.gate_proj.out_features;
        let mut gate_out = vec![0.0f32; seq_len * hidden_dim];
        let mut up_out = vec![0.0f32; seq_len * hidden_dim];
        let mut act_out = vec![0.0f32; seq_len * hidden_dim];

        self.gate_proj.forward(input, &mut gate_out, seq_len);
        self.up_proj.forward(input, &mut up_out, seq_len);

        for i in 0..gate_out.len() {
            act_out[i] = silu(gate_out[i]) * up_out[i];
        }

        self.down_proj.forward(&act_out, output, seq_len);
    }
}

// ============================================================================
// 3. KV キャッシュ構造体
// ============================================================================

#[derive(Default)]
struct KVCache {
    k_cache: Vec<f32>, // [cached_seq_len x num_kv_heads x head_dim]
    v_cache: Vec<f32>, // [cached_seq_len x num_kv_heads x head_dim]
    cached_seq_len: usize,
}

impl KVCache {
    fn reset(&mut self) {
        self.k_cache.clear();
        self.v_cache.clear();
        self.cached_seq_len = 0;
    }

    fn append(&mut self, new_k: &[f32], new_v: &[f32], num_tokens: usize, num_kv_heads: usize, head_dim: usize) {
        let elements = num_tokens * num_kv_heads * head_dim;
        self.k_cache.extend_from_slice(&new_k[..elements]);
        self.v_cache.extend_from_slice(&new_v[..elements]);
        self.cached_seq_len += num_tokens;
    }
}

// ============================================================================
// 4. アテンション層 (GQA + 高精度 f64 RoPE)
// ============================================================================

struct QwenAttention {
    d_model: usize,
    num_heads: usize,
    num_kv_heads: usize,
    head_dim: usize,
    num_queries_per_kv: usize,
    rope_theta: f32,

    q_proj: Linear,
    k_proj: Linear,
    v_proj: Linear,
    out_proj: Linear,
}

impl QwenAttention {
    fn new(d_model: usize, num_heads: usize, num_kv_heads: usize, rng: &mut SimpleRng, rope_theta: f32) -> Self {
        let head_dim = d_model / num_heads;
        QwenAttention {
            d_model,
            num_heads,
            num_kv_heads,
            head_dim,
            num_queries_per_kv: num_heads / num_kv_heads,
            rope_theta,
            q_proj: Linear::new(d_model, num_heads * head_dim, rng),
            k_proj: Linear::new(d_model, num_kv_heads * head_dim, rng),
            v_proj: Linear::new(d_model, num_kv_heads * head_dim, rng),
            out_proj: Linear::new(num_heads * head_dim, d_model, rng),
        }
    }

    // f64 精度による倍精度 RoPE 回転の計算
    fn apply_rope_high_precision(&self, vec: &mut [f32], abs_pos: usize, dim: usize) {
        for i in (0..dim).step_by(2) {
            let freq = 1.0 / (self.rope_theta as f64).powf(i as f64 / dim as f64);
            let theta = (abs_pos as f64) * freq;
            let cos_t = theta.cos();
            let sin_t = theta.sin();

            let v0 = vec[i] as f64;
            let v1 = vec[i + 1] as f64;

            vec[i] = (v0 * cos_t - v1 * sin_t) as f32;
            vec[i + 1] = (v0 * sin_t + v1 * cos_t) as f32;
        }
    }

    fn forward(&self, input: &[f32], output: &mut [f32], num_tokens: usize, kv_cache: &mut KVCache) {
        let start_pos = kv_cache.cached_seq_len;

        let mut q = vec![0.0f32; num_tokens * self.num_heads * self.head_dim];
        let mut new_k = vec![0.0f32; num_tokens * self.num_kv_heads * self.head_dim];
        let mut new_v = vec![0.0f32; num_tokens * self.num_kv_heads * self.head_dim];

        self.q_proj.forward(input, &mut q, num_tokens);
        self.k_proj.forward(input, &mut new_k, num_tokens);
        self.v_proj.forward(input, &mut new_v, num_tokens);

        for t in 0..num_tokens {
            let abs_pos = start_pos + t;
            for h in 0..self.num_heads {
                let offset = (t * self.num_heads + h) * self.head_dim;
                self.apply_rope_high_precision(&mut q[offset..offset + self.head_dim], abs_pos, self.head_dim);
            }
            for h in 0..self.num_kv_heads {
                let offset = (t * self.num_kv_heads + h) * self.head_dim;
                self.apply_rope_high_precision(&mut new_k[offset..offset + self.head_dim], abs_pos, self.head_dim);
            }
        }

        kv_cache.append(&new_k, &new_v, num_tokens, self.num_kv_heads, self.head_dim);

        let total_seq_len = kv_cache.cached_seq_len;
        let k_full = &kv_cache.k_cache;
        let v_full = &kv_cache.v_cache;

        let mut concat_attn_out = vec![0.0f32; num_tokens * self.num_heads * self.head_dim];
        let scale = (1.0 / (self.head_dim as f64).sqrt()) as f32;

        for h in 0..self.num_heads {
            let kv_h = h / self.num_queries_per_kv;

            for i in 0..num_tokens {
                let current_abs_pos = start_pos + i;
                let mut attn_scores = vec![-1e9f32; total_seq_len];

                for j in 0..=current_abs_pos {
                    let mut score = 0.0f64;
                    let q_offset = (i * self.num_heads + h) * self.head_dim;
                    let k_offset = (j * self.num_kv_heads + kv_h) * self.head_dim;

                    for d in 0..self.head_dim {
                        let q_val = q[q_offset + d];
                        let k_val = k_full[k_offset + d];
                        score += (q_val as f64) * (k_val as f64);
                    }
                    attn_scores[j] = (score as f32) * scale;
                }

                safe_softmax_inplace(&mut attn_scores[0..=current_abs_pos]);

                for d in 0..self.head_dim {
                    let mut head_out = 0.0f64;
                    for j in 0..=current_abs_pos {
                        let v_offset = (j * self.num_kv_heads + kv_h) * self.head_dim;
                        let v_val = v_full[v_offset + d];
                        head_out += (attn_scores[j] as f64) * (v_val as f64);
                    }
                    let out_offset = (i * self.num_heads + h) * self.head_dim + d;
                    concat_attn_out[out_offset] = head_out as f32;
                }
            }
        }

        self.out_proj.forward(&concat_attn_out, output, num_tokens);
    }
}

// ============================================================================
// 5. Transformer Block & LLM アーキテクチャ
// ============================================================================

struct QwenBlock {
    attn: QwenAttention,
    norm1: RMSNorm,
    ffn: SwiGLUFFN,
    norm2: RMSNorm,
}

impl QwenBlock {
    fn new(d_model: usize, num_heads: usize, num_kv_heads: usize, intermediate_size: usize, rng: &mut SimpleRng, rope_theta: f32) -> Self {
        QwenBlock {
            attn: QwenAttention::new(d_model, num_heads, num_kv_heads, rng, rope_theta),
            norm1: RMSNorm::new(d_model, 1e-6),
            ffn: SwiGLUFFN::new(d_model, intermediate_size, rng),
            norm2: RMSNorm::new(d_model, 1e-6),
        }
    }

    fn forward(&self, input: &[f32], output: &mut [f32], num_tokens: usize, kv_cache: &mut KVCache) {
        let d_model = self.attn.d_model;
        let total_size = num_tokens * d_model;

        let mut norm1_out = vec![0.0f32; total_size];
        self.norm1.forward(input, &mut norm1_out, num_tokens);

        let mut attn_out = vec![0.0f32; total_size];
        self.attn.forward(&norm1_out, &mut attn_out, num_tokens, kv_cache);

        let mut residual1 = vec![0.0f32; total_size];
        for i in 0..total_size {
            residual1[i] = input[i] + attn_out[i];
        }

        let mut norm2_out = vec![0.0f32; total_size];
        self.norm2.forward(&residual1, &mut norm2_out, num_tokens);

        let mut ffn_out = vec![0.0f32; total_size];
        self.ffn.forward(&norm2_out, &mut ffn_out, num_tokens);

        for i in 0..total_size {
            output[i] = residual1[i] + ffn_out[i];
        }
    }
}

struct QwenLLM {
    vocab_size: usize,
    d_model: usize,
    num_layers: usize,

    token_embedding_table: Vec<f32>,
    layers: Vec<QwenBlock>,
    final_norm: RMSNorm,
    lm_head: Linear,

    layer_caches: Vec<KVCache>,
    rng: SimpleRng,
}

impl QwenLLM {
    fn new(vocab_size: usize, d_model: usize, num_layers: usize, num_heads: usize, num_kv_heads: usize, intermediate_size: usize, seed: u64) -> Self {
        let mut rng = SimpleRng::new(seed);
        let stddev = (1.0 / d_model as f32).sqrt();

        let mut token_embedding_table = vec![0.0f32; vocab_size * d_model];
        for e in token_embedding_table.iter_mut() {
            *e = rng.gen_normal(0.0, stddev);
        }

        let mut layers = Vec::with_capacity(num_layers);
        for _ in 0..num_layers {
            layers.push(QwenBlock::new(d_model, num_heads, num_kv_heads, intermediate_size, &mut rng, 1000000.0));
        }

        let final_norm = RMSNorm::new(d_model, 1e-6);
        let lm_head = Linear::new(d_model, vocab_size, &mut rng);

        let mut layer_caches = Vec::with_capacity(num_layers);
        for _ in 0..num_layers {
            layer_caches.push(KVCache::default());
        }

        QwenLLM {
            vocab_size,
            d_model,
            num_layers,
            token_embedding_table,
            layers,
            final_norm,
            lm_head,
            layer_caches,
            rng,
        }
    }

    fn reset_cache(&mut self) {
        for cache in self.layer_caches.iter_mut() {
            cache.reset();
        }
    }

    fn forward(&mut self, input_ids: &[usize], logits: &mut Vec<f32>) {
        let num_tokens = input_ids.len();

        let mut hidden_states = vec![0.0f32; num_tokens * self.d_model];
        for t in 0..num_tokens {
            let token_id = input_ids[t];
            let emb_ptr = &self.token_embedding_table[token_id * self.d_model..(token_id + 1) * self.d_model];
            hidden_states[t * self.d_model..(t + 1) * self.d_model].copy_from_slice(emb_ptr);
        }

        let mut layer_output = vec![0.0f32; num_tokens * self.d_model];
        for l in 0..self.num_layers {
            self.layers[l].forward(&hidden_states, &mut layer_output, num_tokens, &mut self.layer_caches[l]);
            hidden_states.copy_from_slice(&layer_output);
        }

        let mut norm_output = vec![0.0f32; num_tokens * self.d_model];
        self.final_norm.forward(&hidden_states, &mut norm_output, num_tokens);

        logits.resize(num_tokens * self.vocab_size, 0.0);
        self.lm_head.forward(&norm_output, logits, num_tokens);
    }

    // Top-K & Top-P (Nucleus) サンプリングアルゴリズム
    fn sample_advanced(&mut self, logits_ptr: &[f32], temperature: f32, top_k: usize, top_p: f32) -> usize {
        if temperature <= 0.0 {
            // Greedy Search (Argmax)
            return logits_ptr
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(Ordering::Equal))
                .map(|(idx, _)| idx)
                .unwrap_or(0);
        }

        #[derive(Clone, Copy)]
        struct TokenProb {
            id: usize,
            prob: f32,
        }

        let mut temp_logits = vec![0.0f32; self.vocab_size];
        for v in 0..self.vocab_size {
            temp_logits[v] = logits_ptr[v] / temperature;
        }
        safe_softmax_inplace(&mut temp_logits);

        let mut candidates: Vec<TokenProb> = (0..self.vocab_size)
            .map(|i| TokenProb { id: i, prob: temp_logits[i] })
            .collect();

        // 確率降順にソート
        candidates.sort_by(|a, b| b.prob.partial_cmp(&a.prob).unwrap_or(Ordering::Equal));

        // 1. Top-K フィルタリング
        if top_k > 0 && top_k < self.vocab_size {
            candidates.truncate(top_k);
        }

        // 2. Top-P (Nucleus) フィルタリング
        let mut cum_sum = 0.0f32;
        let mut cutoff_index = candidates.len() - 1;
        for (i, cand) in candidates.iter().enumerate() {
            cum_sum += cand.prob;
            if cum_sum >= top_p {
                cutoff_index = i;
                break;
            }
        }
        candidates.truncate(cutoff_index + 1);

        // 再正規化
        let norm_factor: f32 = candidates.iter().map(|c| c.prob).sum();
        for cand in candidates.iter_mut() {
            cand.prob /= norm_factor;
        }

        // 累積確率によるサンプリング
        let r = self.rng.gen_f32();
        let mut acc = 0.0f32;
        for cand in &candidates {
            acc += cand.prob;
            if r <= acc {
                return cand.id;
            }
        }

        candidates.last().map(|c| c.id).unwrap_or(0)
    }
}

// ============================================================================
// 6. メイン実行プログラム
// ============================================================================

fn main() {
    let vocab_size = 500;
    let d_model = 64;
    let num_layers = 4;
    let num_heads = 8;
    let num_kv_heads = 2; // GQA (4 Queries per KV)
    let intermediate_size = 128;

    println!("=========================================================");
    println!("  Qwen2.5 High-Precision Engine (Pure Rust Implementation)");
    println!("=========================================================");

    let mut model = QwenLLM::new(
        vocab_size,
        d_model,
        num_layers,
        num_heads,
        num_kv_heads,
        intermediate_size,
        2026,
    );

    let prompt: Vec<usize> = vec![10, 256, 42, 88, 300];
    print!("\n[Input Prompt Tokens]: ");
    for t in &prompt {
        print!("{} ", t);
    }
    println!();

    // --- Phase 1: Prefill ---
    let mut logits = Vec::new();
    model.forward(&prompt, &mut logits);

    let last_token_logits = &logits[(prompt.len() - 1) * vocab_size..prompt.len() * vocab_size];

    // 高精度 Top-K / Top-P サンプリングの利用
    let mut next_token = model.sample_advanced(last_token_logits, 0.7, 40, 0.9);

    println!("\n--- Prefill Phase Complete ---");
    println!("First Generated Token: {}", next_token);

    // --- Phase 2: Autoregressive Generation ---
    let gen_length = 10;
    let mut generated = prompt.clone();
    generated.push(next_token);

    println!("\n--- Generation Phase ---");
    for step in 0..gen_length {
        let step_input = vec![next_token];
        model.forward(&step_input, &mut logits);

        next_token = model.sample_advanced(&logits[0..vocab_size], 0.7, 40, 0.9);
        generated.push(next_token);

        println!(
            "Step {:2} | Sampled Token: {:4} | KV Cache Sequence Length: {}",
            step + 1,
            next_token,
            model.layer_caches[0].cached_seq_len
        );
    }

    print!("\n[Final Output Sequence]: ");
    for id in &generated {
        print!("{} ", id);
    }
    println!();
}