//! デジタルフィルタモジュール
//!
//! センサーデータに対して各種デジタルフィルタ処理を提供します。
//! - ローパスフィルタ（1次・2次）
//! - 移動平均フィルタ
//! - カルマンフィルタ（簡易版）
//! - メディアンフィルタ

use heapless::Deque;

/// フィルタ係数の精度（固定小数点）
/// 値は 0〜1000 の範囲で表現（1000 = 1.0）
pub const FILTER_SCALE: i32 = 1000;

/// 1次ローパスフィルタ
///
/// 式: y[n] = alpha * x[n] + (1 - alpha) * y[n-1]
/// alpha: 0〜1000（大きいほど新しい値に追従）
pub struct LowPassFilter1 {
    alpha: i32,
    prev_output: i32,
    initialized: bool,
}

impl LowPassFilter1 {
    /// 新しいフィルタを作成
    ///
    /// # Arguments
    /// * `alpha` - フィルタ係数（0〜1000）
    pub fn new(alpha: i32) -> Self {
        assert!(alpha >= 0 && alpha <= FILTER_SCALE, "alpha must be in 0..=1000");
        Self {
            alpha,
            prev_output: 0,
            initialized: false,
        }
    }

    /// フィルタ係数を更新
    pub fn set_alpha(&mut self, alpha: i32) {
        assert!(alpha >= 0 && alpha <= FILTER_SCALE);
        self.alpha = alpha;
    }

    /// 入力値をフィルタ処理
    pub fn process(&mut self, input: i32) -> i32 {
        if !self.initialized {
            self.prev_output = input;
            self.initialized = true;
            return input;
        }

        let output = (self.alpha * input
            + (FILTER_SCALE - self.alpha) * self.prev_output)
            / FILTER_SCALE;
        self.prev_output = output;
        output
    }

    /// フィルタをリセット
    pub fn reset(&mut self) {
        self.initialized = false;
        self.prev_output = 0;
    }
}

/// 2次ローパスフィルタ（バイリニア変換による設計）
pub struct LowPassFilter2 {
    /// フィルタ係数 a1
    a1: i32,
    /// フィルタ係数 a2
    a2: i32,
    /// フィルタ係数 b0
    b0: i32,
    /// フィルタ係数 b1
    b1: i32,
    /// フィルタ係数 b2
    b2: i32,
    /// 過去の入力 x[n-1], x[n-2]
    x_history: [i32; 2],
    /// 過去の出力 y[n-1], y[n-2]
    y_history: [i32; 2],
    /// 係数スケール
    coeff_scale: i32,
}

impl LowPassFilter2 {
    /// 新しい2次フィルタを作成
    ///
    /// 係数は coeff_scale でスケーリングされた整数値
    pub fn new(a1: i32, a2: i32, b0: i32, b1: i32, b2: i32, coeff_scale: i32) -> Self {
        Self {
            a1, a2, b0, b1, b2,
            x_history: [0; 2],
            y_history: [0; 2],
            coeff_scale,
        }
    }

    /// 入力値をフィルタ処理
    pub fn process(&mut self, input: i32) -> i32 {
        let output = (self.b0 * input
            + self.b1 * self.x_history[0]
            + self.b2 * self.x_history[1]
            - self.a1 * self.y_history[0]
            - self.a2 * self.y_history[1])
            / self.coeff_scale;

        // 履歴をシフト
        self.x_history[1] = self.x_history[0];
        self.x_history[0] = input;
        self.y_history[1] = self.y_history[0];
        self.y_history[0] = output;

        output
    }

    /// フィルタをリセット
    pub fn reset(&mut self) {
        self.x_history = [0; 2];
        self.y_history = [0; 2];
    }
}

/// 移動平均フィルタ
///
/// N個のサンプルの平均を計算する。
pub struct MovingAverageFilter<const N: usize> {
    buffer: Deque<i32, N>,
    sum: i32,
}

impl<const N: usize> MovingAverageFilter<N> {
    /// 新しい移動平均フィルタを作成
    pub fn new() -> Self {
        Self {
            buffer: Deque::new(),
            sum: 0,
        }
    }

    /// 入力値をフィルタ処理
    pub fn process(&mut self, input: i32) -> i32 {
        if self.buffer.is_full() {
            if let Some(old) = self.buffer.pop_front() {
                self.sum -= old;
            }
        }
        let _ = self.buffer.push_back(input);
        self.sum += input;

        self.sum / self.buffer.len() as i32
    }

    /// バッファが満杯か確認
    pub fn is_full(&self) -> bool {
        self.buffer.is_full()
    }

    /// 現在のサンプル数
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// フィルタをリセット
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.sum = 0;
    }
}

/// 簡易カルマンフィルタ
///
/// 1次元の状態推定を行う簡易カルマンフィルタ。
/// ノイズを含む観測値から真の値を推定する。
pub struct KalmanFilter {
    /// 推定誤差共分散
    estimate_error: i32,
    /// 観測ノイズ共分散
    measurement_error: i32,
    /// プロセノイズ共分散
    process_noise: i32,
    /// 現在の推定値
    estimate: i32,
    /// 初期化済みフラグ
    initialized: bool,
    /// 内部スケール
    scale: i32,
}

impl KalmanFilter {
    /// 新しいカルマンフィルタを作成
    ///
    /// # Arguments
    /// * `measurement_error` - 観測ノイズ（大きいほど観測値を信頼しにくい）
    /// * `process_noise` - プロセスノイズ（大きいほど推定値の変化に追従しやすい）
    /// * `scale` - 内部計算のスケール係数
    pub fn new(measurement_error: i32, process_noise: i32, scale: i32) -> Self {
        Self {
            estimate_error: measurement_error,
            measurement_error,
            process_noise,
            estimate: 0,
            initialized: false,
            scale,
        }
    }

    /// 入力値をフィルタ処理
    pub fn process(&mut self, measurement: i32) -> i32 {
        if !self.initialized {
            self.estimate = measurement;
            self.initialized = true;
            return measurement;
        }

        // 予測ステップ
        self.estimate_error += self.process_noise;

        // カルマンゲイン計算
        let kalman_gain = (self.estimate_error * self.scale)
            / (self.estimate_error + self.measurement_error);

        // 更新ステップ
        self.estimate = self.estimate
            + (kalman_gain * (measurement - self.estimate)) / self.scale;

        // 誤差共分散の更新
        self.estimate_error =
            (self.scale - kalman_gain) * self.estimate_error / self.scale;

        self.estimate
    }

    /// 現在の推定値を取得
    pub fn estimate(&self) -> i32 {
        self.estimate
    }

    /// フィルタをリセット
    pub fn reset(&mut self) {
        self.initialized = false;
        self.estimate_error = self.measurement_error;
        self.estimate = 0;
    }
}

/// メディアンフィルタ
///
/// ノイズスパイク（外れ値）を除去するフィルタ。
pub struct MedianFilter<const N: usize> {
    buffer: Deque<i32, N>,
}

impl<const N: usize> MedianFilter<N> {
    /// 新しいメディアンフィルタを作成
    pub fn new() -> Self {
        Self {
            buffer: Deque::new(),
        }
    }

    /// 入力値をフィルタ処理
    pub fn process(&mut self, input: i32) -> i32 {
        if self.buffer.is_full() {
            let _ = self.buffer.pop_front();
        }
        let _ = self.buffer.push_back(input);

        // バッファをソートして中央値を取得
        let mut sorted: [i32; N] = [0; N];
        for (i, v) in self.buffer.iter().enumerate() {
            sorted[i] = *v;
        }
        // 簡易バブルソート（Nは小さいので問題なし）
        let len = self.buffer.len();
        for i in 0..len {
            for j in 0..(len - i - 1) {
                if sorted[j] > sorted[j + 1] {
                    sorted.swap(j, j + 1);
                }
            }
        }

        sorted[len / 2]
    }

    /// フィルタをリセット
    pub fn reset(&mut self) {
        self.buffer.clear();
    }
}

/// フィルタチェーン
///
/// 複数のフィルタを順次適用する。
pub struct FilterChain {
    lp_filter: LowPassFilter1,
    ma_filter: MovingAverageFilter<8>,
}

impl FilterChain {
    /// 新しいフィルタチェーンを作成
    pub fn new(lp_alpha: i32) -> Self {
        Self {
            lp_filter: LowPassFilter1::new(lp_alpha),
            ma_filter: MovingAverageFilter::new(),
        }
    }

    /// フィルタチェーンを適用
    pub fn process(&mut self, input: i32) -> i32 {
        let stage1 = self.lp_filter.process(input);
        self.ma_filter.process(stage1)
    }

    /// リセット
    pub fn reset(&mut self) {
        self.lp_filter.reset();
        self.ma_filter.reset();
    }
}
