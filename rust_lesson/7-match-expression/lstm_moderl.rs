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


use anyhow::{anyhow, Result};
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;
use tch::{nn, nn::Module, nn::OptimizerConfig, Device, Kind, Tensor};

// -----------------------------------------------------------------------------
// 1. DB レコードデータモデル
// -----------------------------------------------------------------------------

#[derive(FromRow, Debug)]
struct MetricRecord {
    response_time_ms: i64,
    mem_kb: i64,
}

// -----------------------------------------------------------------------------
// 2. LSTM ネットワークモデル定義
// -----------------------------------------------------------------------------

#[derive(Debug)]
struct LstmModel {
    lstm: nn::LSTM,
    fc: nn::Linear,
}

impl LstmModel {
    fn new(vs: nn::Path, input_dim: i64, hidden_dim: i64, output_dim: i64) -> Self {
        let lstm = nn::lstm(&vs / "lstm", input_dim, hidden_dim, Default::default());
        let fc = nn::linear(&vs / "fc", hidden_dim, output_dim, Default::default());
        LstmModel { lstm, fc }
    }

    fn forward(&self, x: &Tensor) -> Tensor {
        // x: [BatchSize, SeqLen, InputDim]
        let (output, _) = self.lstm.step(x, &self.lstm.zero_state(x.size()[0]));
        let last_hidden = output.select(1, -1); // 最終ステップの隠れ状態
        self.fc.forward(&last_hidden)
    }
}

// -----------------------------------------------------------------------------
// 3. データ前処理 helper
// -----------------------------------------------------------------------------

struct MinMaxScalar {
    min: f32,
    max: f32,
}

impl MinMaxScalar {
    fn fit_transform(data: &[f32]) -> (Vec<f32>, Self) {
        let min = data.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = data.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        
        let range = if (max - min).abs() < 1e-6 { 1.0 } else { max - min };
        let normalized = data.iter().map(|&x| (x - min) / range).collect();

        (normalized, MinMaxScalar { min, max })
    }
}

/// 2変量時系列 [N, 2] から スライディングウィンドウ テンソル [Batch, SeqLen, Features] を構築
fn create_dataset_tensors(
    records: &[f32],
    num_features: usize,
    seq_len: usize,
    device: Device,
) -> (Tensor, Tensor) {
    let total_timesteps = records.len() / num_features;
    let num_samples = total_timesteps - seq_len;

    let mut x_vec = Vec::new();
    let mut y_vec = Vec::new();

    for i in 0..num_samples {
        // [i .. i + seq_len] の期間を入力特徴量 X とする
        let start_idx = i * num_features;
        let end_idx = (i + seq_len) * num_features;
        x_vec.extend_from_slice(&records[start_idx..end_idx]);

        // 次ステップ (i + seq_len) の予測対象 Y (例: RTT)
        let target_idx = (i + seq_len) * num_features;
        y_vec.push(records[target_idx]); // 応答時間をターゲットとする
    }

    let x_tensor = Tensor::from_slice(&x_vec)
        .view([num_samples as i64, seq_len as i64, num_features as i64])
        .to_device(device);

    let y_tensor = Tensor::from_slice(&y_vec)
        .view([num_samples as i64, 1])
        .to_device(device);

    (x_tensor, y_tensor)
}

// -----------------------------------------------------------------------------
// 4. DB 取得 & 学習メインプロセス
// -----------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== 蓄積DBデータを用いた LSTM モデルの学習パイプライン ===");

    let db_path = "app_monitor.db";
    let model_save_path = "lstm_model.ot";
    let device = Device::Cpu;

    // --- (Step 1) SQLite から学習用データの抽出 ---
    println!("[-] SQLite データベース ({}) から過去メトリクスを取得中...", db_path);
    let pool = SqlitePool::connect(&format!("sqlite:{}?mode=ro", db_path)).await
        .map_err(|_| anyhow!("DB接続失敗: 先に監視アプリでデータ（{}）を生成してください", db_path))?;

    // node_logs または app_health_logs から時系列順に取得
    let rows = sqlx::query_as::<_, MetricRecord>(
        "SELECT rtt_ms as response_time_ms, total_mem_kb as mem_kb FROM app_health_logs ORDER BY id ASC",
    )
    .fetch_all(&pool)
    .await?;

    if rows.len() < 30 {
        return Err(anyhow!(
            "学習に必要なデータ件数が不足しています（現在: {} 件, 推奨: 30 件以上）",
            rows.len()
        ));
    }

    println!("[+] 取得完了: {} 件の学習データ", rows.len());

    // --- (Step 2) データの前処理とテンソル変換 ---
    let mut raw_rtt = Vec::new();
    let mut raw_mem = Vec::new();

    for r in &rows {
        raw_rtt.push(r.response_time_ms as f32);
        raw_mem.push(r.mem_kb as f32);
    }

    // 各メトリクスを 0.0 ～ 1.0 に正規化
    let (norm_rtt, scalar_rtt) = MinMaxScalar::fit_transform(&raw_rtt);
    let (norm_mem, _scalar_mem) = MinMaxScalar::fit_transform(&raw_mem);

    // インターリーブ形式 [RTT_0, MEM_0, RTT_1, MEM_1, ...] に整形
    let mut interleaved_data = Vec::new();
    for i in 0..rows.len() {
        interleaved_data.push(norm_rtt[i]);
        interleaved_data.push(norm_mem[i]);
    }

    let seq_len = 10;      // 過去10ポイントのデータから未来を予測
    let num_features = 2; // 入力特徴量数 (RTT, メモリ使用量)

    let (x_train, y_train) = create_dataset_tensors(&interleaved_data, num_features, seq_len, device);

    println!(
        "[-] テンソル整形完了: X_shape = {:?}, Y_shape = {:?}",
        x_train.size(),
        y_train.size()
    );

    // --- (Step 3) モデル・最適化関数の設定 ---
    let vs = nn::VarStore::new(device);
    let hidden_dim = 32;
    let output_dim = 1;

    let model = LstmModel::new(vs.root(), num_features as i64, hidden_dim, output_dim);
    let mut optimizer = nn::Adam::default().build(&vs, 1e-3)?;

    // --- (Step 4) 学習ループの実行 ---
    let epochs = 150;
    println!("\n[-] LSTM モデルの学習を開始します (Epochs: {})...", epochs);

    for epoch in 1..=epochs {
        let predictions = model.forward(&x_train);
        
        // MSE (Mean Squared Error) 損失
        let loss = predictions.mse_loss(&y_train, tch::Reduction::Mean);

        optimizer.backward_step(&loss);

        if epoch % 30 == 0 || epoch == epochs {
            let loss_val: f64 = loss.double_value(&[]);
            println!("- Epoch {:3}/{} | MSE Loss: {:.6}", epoch, epochs, loss_val);
        }
    }

    // --- (Step 5) 学習済みモデル重みの保存 ---
    println!("\n[-] 学習済みモデルパラメータを保存中: {}", model_save_path);
    vs.save(model_save_path)?;
    println!("[+] モデルの保存が完了しました！");

    // --- (Step 6) 簡易検証 (Validation Check) ---
    println!("\n--- 学習モデルの予測テスト (最新サンプル) ---");
    let test_pred = model.forward(&x_train);
    let last_idx = x_train.size()[0] - 1;

    let pred_norm: f32 = test_pred.get(last_idx).get(0).try_into()?;
    let actual_norm: f32 = y_train.get(last_idx).get(0).try_into()?;

    // 正規化を逆変換して元の単位 (ms) に戻す
    let pred_rtt_ms = pred_norm * (scalar_rtt.max - scalar_rtt.min) + scalar_rtt.min;
    let actual_rtt_ms = actual_norm * (scalar_rtt.max - scalar_rtt.min) + scalar_rtt.min;

    println!(
        "実測値 (最新): {:.2} ms | モデル予測値: {:.2} ms (差分: {:.2} ms)",
        actual_rtt_ms,
        pred_rtt_ms,
        (actual_rtt_ms - pred_rtt_ms).abs()
    );

    Ok(())
}