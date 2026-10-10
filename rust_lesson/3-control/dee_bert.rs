use candle_core::{Device, Tensor};
use candle_nn as nn;

// --- 1. Exit Head (各層の中間分類器) の定義 ---
struct ExitHead {
    classifier: nn.Linear,
}

impl ExitHead {
    fn new(vs: nn.VarBuilder, hidden_size: usize, num_classes: usize) -> candle_core::Result<Self> {
        let classifier = nn.linear(hidden_size, num_classes, vs.pp("classifier"))?;
        Ok(Self { classifier })
    }

    // 順伝播とエントロピー計算
    fn forward_and_entropy(&self, hidden_states: &Tensor) -> candle_core::Result<(Tensor, f32)> {
        // [batch_size, seq_len, hidden_size] の先頭トークン [CLS] を抽出: (batch_size, hidden_size)
        let cls_output = hidden_states.i((.., 0, ..))?;
        let logits = self.classifier.forward(&cls_output)?; // (batch_size, num_classes)

        // Softmax で確率分布に変換
        let probs = candle_nn::ops::softmax(&logits, 1)?;

        // エントロピー計算: -sum(p * log(p))
        // 簡易的にバッチ先頭の確率配列を取得して計算
        let probs_vec = probs.to_vec2::<f32>()?;
        let p = &probs_vec[0];
        let entropy: f32 = p.iter()
            .map(|&pi| if pi > 1e-12 { -pi * pi.ln() } else { 0.0 })
            .sum();

        Ok((logits, entropy))
    }
}

// --- 2. DeeBERT 推論エンジンの構造体 ---
struct DeeBertInferenceModel {
    // 簡易的に複数のレイヤーとExit Headを保持
    hidden_size: usize,
    num_classes: usize,
    num_layers: usize,
    entropy_threshold: f32,
}

impl DeeBertInferenceModel {
    fn new(hidden_size: usize, num_classes: usize, num_layers: usize, entropy_threshold: f32) -> Self {
        Self {
            hidden_size,
            num_classes,
            num_layers,
            entropy_threshold,
        }
    }

    /// 早期終了判定つきのダミー推論実行
    fn infer(&self, input_ids: &Tensor, device: &Device) -> candle_core::Result<(Tensor, usize, bool)> {
        // 疑似的な埋め込み層から初期隠れ状態を作成
        let batch_size = input_ids.dim(0)?;
        let seq_len = input_ids.dim(1)?;
        let mut x = Tensor::randn(0.0f32, 1.0f32, (batch_size, seq_len, self.hidden_size), device)?;

        let vs = nn.VarBuilder::zeros(candle_core::DType::F32, device);
        
        let mut final_logits = Tensor::zeros((batch_size, self.num_classes), candle_core::DType::F32, device)?;
        let mut exited_layer = self.num_layers;
        let mut is_early_exited = false;

        // 各層のループ
        for layer_idx in 0..self.num_layers {
            // 1. トランスフォーマー層の処理（ここでは簡略化のためランダム変形）
            let layer_weight = Tensor::randn(0.0f32, 0.1f32, (self.hidden_size, self.hidden_size), device)?;
            x = x.matmul(&layer_weight)?;

            // 2. この層のExit Headを生成して評価
            let exit_head = ExitHead::new(vs.pp(format!("exit_head_{}", layer_idx)), self.hidden_size, self.num_classes)?;
            let (logits, entropy) = exit_head.forward_and_entropy(&x)?;

            println!("- Layer {} 処理完了. エントロピー: {:.4} (閾値: {:.4})", layer_idx + 1, entropy, self.entropy_threshold);

            // 3. エントロピーが閾値を下回ったら早期終了 (Early Exit)
            if entropy < self.entropy_threshold {
                final_logits = logits;
                exited_layer = layer_idx + 1;
                is_early_exited = true;
                break;
            }

            final_logits = logits;
        }

        Ok((final_logits, exited_layer, is_early_exited))
    }
}

// --- 3. メイン関数 ---
fn main() -> candle_core::Result<()> {
    // デバイス設定 (CPU)
    let device = Device::Cpu;

    let hidden_size = 64;
    let num_classes = 2; // 0: NEGATIVE, 1: POSITIVE
    let num_layers = 6;
    let entropy_threshold = 0.5; // 終了判定の閾値

    let model = DeeBertInferenceModel::new(hidden_size, num_classes, num_layers, entropy_threshold);

    // ダミーの入力トークン ID (Batch size: 1, Sequence length: 8)
    let input_ids = Tensor::new(&[[101_u32, 2054, 2003, 1037, 3225, 102, 0, 0]], &device)?;

    println!("=== RustによるDeeBERT Early Exit 推論デモ ===");
    println!("設定閾値: {}\n", entropy_threshold);

    // 推論の実行
    let (logits, exited_layer, early_exited) = model.infer(&input_ids, &device)?;

    println!("\n--- 推論結果 ---");
    if early_exited {
        println!(">>> 早期終了 (Early Exit) がトリガーされました！");
        println!("    終了層: Layer {} / {}", exited_layer, num_layers);
    } else {
        println!(">>> すべての層を通過しました (通常推論)");
        println!("    最終層: Layer {}", num_layers);
    }
    
    println!("出力ロジット形状: {:?}", logits.shape());
    Ok(())
}