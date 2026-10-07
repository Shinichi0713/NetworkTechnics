use anyhow::Result;
use chrono::Local;
use tch::{nn, nn::Module, nn::OptimizerConfig, Device, Kind, Tensor};

// -----------------------------------------------------------------------------
// 1. LSTM 異常検知モデル構造体の定義
// -----------------------------------------------------------------------------

#[derive(Debug)]
struct LstmAnomalyDetector {
    lstm: nn::LSTM,
    fc: nn::Linear,
}

impl LstmAnomalyDetector {
    fn new(vs: nn::Path, input_dim: i64, hidden_dim: i64, output_dim: i64) -> Self {
        let lstm = nn::lstm(&vs / "lstm", input_dim, hidden_dim, Default::default());
        let fc = nn::linear(&vs / "fc", hidden_dim, output_dim, Default::default());

        LstmAnomalyDetector { lstm, fc }
    }

    /// 時系列シーケンス [Batch, SeqLen, InputDim] を入力し、次ステップの予測値 [Batch, OutputDim] を出力
    fn forward(&self, x: &Tensor) -> Tensor {
        // LSTM の出力: (output, (h_n, c_n))
        // output テンソル形状: [Batch, SeqLen, HiddenDim]
        let (output, _) = self.lstm.step(&x, &self.lstm.zero_state(x.size()[0]));

        // 最終タイムステップの隠れ状態を取得: [Batch, HiddenDim]
        let last_hidden = output.select(1, -1);

        // 全結合層で予測値を計算: [Batch, OutputDim]
        self.fc.forward(&last_hidden)
    }
}

// -----------------------------------------------------------------------------
// 2. 時系列スライディングウィンドウ処理
// -----------------------------------------------------------------------------

/// 1次元の時系列データから、[Batch, SeqLen, InputDim] の入力 X と [Batch, InputDim] のターゲット Y を作成
fn create_sequences(
    data: &[f32],
    seq_len: usize,
    device: Device,
) -> (Tensor, Tensor) {
    let num_samples = data.len() - seq_len;
    let mut x_vec = Vec::new();
    let mut y_vec = Vec::new();

    for i in 0..num_samples {
        x_vec.extend_from_slice(&data[i..i + seq_len]);
        y_vec.push(data[i + seq_len]);
    }

    let x_tensor = Tensor::from_slice(&x_vec)
        .view([num_samples as i64, seq_len as i64, 1])
        .to_device(device);

    let y_tensor = Tensor::from_slice(&y_vec)
        .view([num_samples as i64, 1])
        .to_device(device);

    (x_tensor, y_tensor)
}

// -----------------------------------------------------------------------------
// 3. メインパイプライン (学習 ＆ リアルタイム異常検知)
// -----------------------------------------------------------------------------

fn main() -> Result<()> {
    println!("=== Rust + LibTorch (tch-rs) による LSTM 時系列異常検知 ===");

    let device = Device::Cpu;

    // --- (A) 正常時の収集メトリクスデータ（学習用） ---
    // 例: 周期的なトラフィック負荷やレイテンシの変動パターン
    let mut normal_time_series: Vec<f32> = Vec::new();
    for i in 0..200 {
        // 正弦波にわずかなノイズを加えた周期パターン
        let val = (i as f32 * 0.1).sin() * 0.5 + 0.5 + (i as f32 * 0.01).sin() * 0.1;
        normal_time_series.push(val);
    }

    let seq_len = 10; // 10ステップの過去データから11ステップ目を予測
    let (x_train, y_train) = create_sequences(&normal_time_series, seq_len, device);

    // --- (B) モデルと最適化手法の初期化 ---
    let vs = nn::VarStore::new(device);
    let input_dim = 1;  // 1変量メトリクス (例: トラフィック率)
    let hidden_dim = 32; // LSTM 隠れ層の次元
    let output_dim = 1;  // 予測値の次元

    let model = LstmAnomalyDetector::new(vs.root(), input_dim, hidden_dim, output_dim);
    let mut optimizer = nn::Adam::default().build(&vs, 1e-3)?;

    // --- (C) LSTM の学習 (正常パターンの獲得) ---
    println!("\n正常パターンの学習を開始します...");
    for epoch in 1..=200 {
        let pred = model.forward(&x_train);
        
        // 平均二乗誤差 (MSE) の計算
        let loss = pred.mse_loss(&y_train, tch::Reduction::Mean);

        optimizer.backward_step(&loss);

        if epoch % 50 == 0 {
            let loss_val: f64 = loss.double_value(&[]);
            println!("Epoch {:3} | Training MSE Loss: {:.6}", epoch, loss_val);
        }
    }

    // --- (D) テストデータ（正常データ ＋ サイレント障害・スパイク異常を含んだデータ） ---
    let mut test_time_series: Vec<f32> = Vec::new();
    for i in 200..250 {
        let mut val = (i as f32 * 0.1).sin() * 0.5 + 0.5 + (i as f32 * 0.01).sin() * 0.1;

        // ステップ 225〜230 で不調（リソース急増・応答遅延スパイク）が発生したと仮定
        if (225..=230).contains(&i) {
            val += 1.8; // パターンから大きく逸脱する異常スパイク
        }
        test_time_series.push(val);
    }

    let (x_test, y_test) = create_sequences(&test_time_series, seq_len, device);

    // --- (E) 推論および予測誤差による異常判定 ---
    println!("\n--- リアルタイム時系列異常検知の評価 ---");

    let predictions = model.forward(&x_test);
    let anomaly_threshold = 0.25; // 予測誤差の閾値 (MSE Threshold)

    let num_test_samples = x_test.size()[0] as usize;

    for i in 0..num_test_samples {
        let actual: f32 = y_test.get(i as i64).get(0).try_into()?;
        let predicted: f32 = predictions.get(i as i64).get(0).try_into()?;

        // 二乗誤差 (Square Error)
        let error = (actual - predicted).powi(2);

        let now = Local::now().format("%H:%M:%S").to_string();
        let timestamp_idx = 200 + seq_len + i;

        if error > anomaly_threshold {
            println!(
                "[{}] [T={:3}] \x1b[31m[ALERT 異常検知]\x1b[0m 実測値: {:.3} | 予測値: {:.3} | 誤差(MSE): {:.4} (> 閾値 {:.2})",
                now, timestamp_idx, actual, predicted, error, anomaly_threshold
            );
        } else {
            println!(
                "[{}] [T={:3}] [正常]              実測値: {:.3} | 予測値: {:.3} | 誤差(MSE): {:.4}",
                now, timestamp_idx, actual, predicted, error
            );
        }
    }

    Ok(())
}