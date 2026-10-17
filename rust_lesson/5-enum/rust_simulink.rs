
//! Simulink風 ブロック図制御シミュレーションエンジン
//! 
//! このクレートは、Simulinkのようなブロック図ベースの制御則構築を
//! Rustで実現するための軽量シミュレーションエンジンです。
//!
//! # 使用例
//! ```
//! use rust_simulink::{Block, Step, Gain, Integrator, Sum, Pid, Simulation, Saturation};
//!
//! // ステップ入力 -> PID -> プラント(1/s) のフィードバック制御系
//! let mut step = Step::new(1.0, 0.0);  // 振幅1, t=0でステップ
//! let mut pid = Pid::new(1.0, 0.5, 0.1); // Kp=1, Ki=0.5, Kd=0.1
//! let mut plant = Integrator::new(0.0);  // 初期値0
//! let mut sum = Sum::new(vec![1.0, -1.0]); // 誤 = 指令 - 出力
//! let dt = 0.01;
//!
//! for i in 0..1000 {
//!     let t = i as f64 * dt;
//!     let r = step.output(t);
//!     let y = plant.output(t);
//!     let e = sum.compute(vec![r, y]);
//!     let u = pid.output(e, dt);
//!     plant.input(u, dt);
//! }
//! ```

use std::f64::consts::PI;

// ============================================================================
// コアトレイト
// ============================================================================

/// すべてのブロックが実装する基本トレイト
pub trait Block {
    /// ブロックの出力を計算（状態更新なし）
    fn output(&self, t: f64) -> f64;

    /// ブロックの内部状態を更新
    fn update(&mut self, input: f64, dt: f64);

    /// 1ステップ実行（出力計算 + 状態更新）
    fn step(&mut self, input: f64, t: f64, dt: f64) -> f64 {
        let out = self.output(t);
        self.update(input, dt);
        out
    }
}

/// SISO（単入力単出力）ダイナミックシステムトレイト
pub trait SisoSystem {
    fn initial_state(&self) -> Vec<f64>;
    fn dynamics(&self, state: &[f64], input: f64) -> Vec<f64>;
    fn output_fn(&self, state: &[f64], input: f64) -> f64;
}

// ============================================================================
// ソースブロック（入力信号）
// ============================================================================

/// ステップ入力ブロック
pub struct Step {
    amplitude: f64,
    step_time: f64,
}

impl Step {
    pub fn new(amplitude: f64, step_time: f64) -> Self {
        Step { amplitude, step_time }
    }

    pub fn output(&self, t: f64) -> f64 {
        if t >= self.step_time {
            self.amplitude
        } else {
            0.0
        }
    }
}

/// 正弦波入力ブロック
pub struct Sine {
    amplitude: f64,
    frequency: f64,  // Hz
    phase: f64,    // rad
    bias: f64,
}

impl Sine {
    pub fn new(amplitude: f64, frequency: f64, phase: f64, bias: f64) -> Self {
        Sine { amplitude, frequency, phase, bias }
    }

    pub fn output(&self, t: f64) -> f64 {
        self.bias + self.amplitude * (2.0 * PI * self.frequency * t + self.phase).sin()
    }
}

/// 定値（バイアス）ブロック
pub struct Constant {
    value: f64,
}

impl Constant {
    pub fn new(value: f64) -> Self {
        Constant { value }
    }

    pub fn output(&self, _t: f64) -> f64 {
        self.value
    }
}

/// ランプ入力ブロック
pub struct Ramp {
    slope: f64,
    start_time: f64,
    bias: f64,
}

impl Ramp {
    pub fn new(slope: f64, start_time: f64, bias: f64) -> Self {
        Ramp { slope, start_time, bias }
    }

    pub fn output(&self, t: f64) -> f64 {
        if t >= self.start_time {
            self.bias + self.slope * (t - self.start_time)
        } else {
            self.bias
        }
    }
}

// ============================================================================
// 算術ブロック（Math Operations）
// ============================================================================

/// 加算ブロック（複数入力の加減算）
pub struct Sum {
    signs: Vec<f64>, // +1.0 または -1.0
}

impl Sum {
    pub fn new(signs: Vec<f64>) -> Self {
        Sum { signs }
    }

    pub fn compute(&self, inputs: Vec<f64>) -> f64 {
        assert_eq!(inputs.len(), self.signs.len(), "入力数と符号数が一致しません");
        inputs.iter().zip(self.signs.iter())
            .map(|(x, s)| x * s)
            .sum()
    }
}

/// ゲインブロック
pub struct Gain {
    gain: f64,
}

impl Gain {
    pub fn new(gain: f64) -> Self {
        Gain { gain }
    }

    pub fn compute(&self, input: f64) -> f64 {
        self.gain * input
    }
}

impl Block for Gain {
    fn output(&self, _t: f64) -> f64 {
        0.0 // 状態を持たないため、computeを直接使用
    }

    fn update(&mut self, _input: f64, _dt: f64) {
        // 状態なし
    }
}

/// 乗算ブロック
pub struct Product;

impl Product {
    pub fn new() -> Self {
        Product
    }

    pub fn compute(&self, inputs: Vec<f64>) -> f64 {
        inputs.iter().product()
    }
}

/// サチュレーション（飽和）ブロック
pub struct Saturation {
    upper_limit: f64,
    lower_limit: f64,
}

impl Saturation {
    pub fn new(upper_limit: f64, lower_limit: f64) -> Self {
        assert!(upper_limit >= lower_limit);
        Saturation { upper_limit, lower_limit }
    }

    pub fn compute(&self, input: f64) -> f64 {
        input.clamp(self.lower_limit, self.upper_limit)
    }
}

/// デッドゾーンブロック
pub struct DeadZone {
    lower: f64,
    upper: f64,
}

impl DeadZone {
    pub fn new(lower: f64, upper: f64) -> Self {
        DeadZone { lower, upper }
    }

    pub fn compute(&self, input: f64) -> f64 {
        if input > self.upper {
            input - self.upper
        } else if input < self.lower {
            input - self.lower
        } else {
            0.0
        }
    }
}

// ============================================================================
// 連続時間ダイナミックブロック
// ============================================================================

/// 積分器ブロック（1/s）
pub struct Integrator {
    state: f64,
    initial_condition: f64,
}

impl Integrator {
    pub fn new(initial_condition: f64) -> Self {
        Integrator {
            state: initial_condition,
            initial_condition,
        }
    }

    pub fn reset(&mut self) {
        self.state = self.initial_condition;
    }

    pub fn state(&self) -> f64 {
        self.state
    }
}

impl Block for Integrator {
    fn output(&self, _t: f64) -> f64 {
        self.state
    }

    fn update(&mut self, input: f64, dt: f64) {
        // 前方オイラー法
        self.state += input * dt;
    }
}

/// 微分器ブロック（s）
pub struct Derivative {
    prev_input: f64,
    prev_time: f64,
    initialized: bool,
}

impl Derivative {
    pub fn new() -> Self {
        Derivative {
            prev_input: 0.0,
            prev_time: 0.0,
            initialized: false,
        }
    }

    pub fn output(&mut self, input: f64, t: f64) -> f64 {
        if !self.initialized {
            self.prev_input = input;
            self.prev_time = t;
            self.initialized = true;
            0.0
        } else {
            let dt = t - self.prev_time;
            if dt > 1e-10 {
                let deriv = (input - self.prev_input) / dt;
                self.prev_input = input;
                self.prev_time = t;
                deriv
            } else {
                0.0
            }
        }
    }
}

/// 1次遅れシステム（Low Pass Filter）
/// 伝達関数: G(s) = K / (T*s + 1)
pub struct FirstOrderLag {
    gain: f64,      // K
    time_constant: f64, // T
    state: f64,
}

impl FirstOrderLag {
    pub fn new(gain: f64, time_constant: f64, initial_condition: f64) -> Self {
        assert!(time_constant > 0.0, "時定数は正である必要があります");
        FirstOrderLag {
            gain,
            time_constant,
            state: initial_condition,
        }
    }
}

impl Block for FirstOrderLag {
    fn output(&self, _t: f64) -> f64 {
        self.state
    }

    fn update(&mut self, input: f64, dt: f64) {
        // dx/dt = (K*u - x) / T
        let dx = (self.gain * input - self.state) / self.time_constant;
        self.state += dx * dt;
    }
}

/// 2次系システム
/// 伝達関数: G(s) = K * omega_n^2 / (s^2 + 2*zeta*omega_n*s + omega_n^2)
pub struct SecondOrderSystem {
    natural_frequency: f64, // omega_n
    damping_ratio: f64,     // zeta
    gain: f64,              // K
    state1: f64,            // x1
    state2: f64,            // x2 = dx1/dt
}

impl SecondOrderSystem {
    pub fn new(omega_n: f64, zeta: f64, gain: f64, x1_0: f64, x2_0: f64) -> Self {
        assert!(omega_n > 0.0, "固有角周波数は正である必要があります");
        SecondOrderSystem {
            natural_frequency: omega_n,
            damping_ratio: zeta,
            gain,
            state1: x1_0,
            state2: x2_0,
        }
    }

    pub fn states(&self) -> (f64, f64) {
        (self.state1, self.state2)
    }
}

impl Block for SecondOrderSystem {
    fn output(&self, _t: f64) -> f64 {
        self.gain * self.state1
    }

    fn update(&mut self, input: f64, dt: f64) {
        let x1 = self.state1;
        let x2 = self.state2;
        let wn = self.natural_frequency;
        let zeta = self.damping_ratio;

        // dx1/dt = x2
        // dx2/dt = -2*zeta*wn*x2 - wn^2*x1 + wn^2*input
        let dx1 = x2;
        let dx2 = -2.0 * zeta * wn * x2 - wn * wn * x1 + wn * wn * input;

        self.state1 += dx1 * dt;
        self.state2 += dx2 * dt;
    }
}

// ============================================================================
// PID制御ブロック
// ============================================================================

/// 連続時間PIDコントローラ
/// u(t) = Kp*e(t) + Ki*∫e(t)dt + Kd*de(t)/dt
pub struct Pid {
    kp: f64,
    ki: f64,
    kd: f64,
    integral: f64,
    prev_error: f64,
    prev_time: f64,
    initialized: bool,
    // アンチウィンドアップ
    anti_windup: bool,
    output_limit: Option<(f64, f64)>, // (lower, upper)
}

impl Pid {
    pub fn new(kp: f64, ki: f64, kd: f64) -> Self {
        Pid {
            kp,
            ki,
            kd,
            integral: 0.0,
            prev_error: 0.0,
            prev_time: 0.0,
            initialized: false,
            anti_windup: false,
            output_limit: None,
        }
    }

    /// アンチウィンドアップを有効化
    pub fn with_anti_windup(mut self, lower: f64, upper: f64) -> Self {
        self.anti_windup = true;
        self.output_limit = Some((lower, upper));
        self
    }

    /// 積分項をリセット
    pub fn reset_integral(&mut self) {
        self.integral = 0.0;
    }

    pub fn output(&mut self, error: f64, dt: f64) -> f64 {
        if !self.initialized {
            self.prev_error = error;
            self.initialized = true;
        }

        // 積分項更新
        self.integral += error * dt;

        // 微分項（バックワード差分）
        let derivative = (error - self.prev_error) / dt;
        self.prev_error = error;

        // PID出力
        let mut output = self.kp * error + self.ki * self.integral + self.kd * derivative;

        // アンチウィンドアップ
        if let Some((lower, upper)) = self.output_limit {
            if output > upper {
                if self.anti_windup {
                    // 過剰分を積分項から除去
                    self.integral -= (output - upper) / self.ki;
                }
                output = upper;
            } else if output < lower {
                if self.anti_windup {
                    self.integral -= (output - lower) / self.ki;
                }
                output = lower;
            }
        }

        output
    }

    pub fn set_gains(&mut self, kp: f64, ki: f64, kd: f64) {
        self.kp = kp;
        self.ki = ki;
        self.kd = kd;
    }
}

// ============================================================================
// 伝達関数ブロク
// ============================================================================

/// 単入力単出力伝達関数
/// G(s) = (b_m*s^m + ... + b_0) / (a_n*s^n + ... + a_0)
/// 分子・分母の係数は最高次から順に指定
pub struct TransferFunction {
    num_coeffs: Vec<f64>, // 分子係数 [b_m, b_{m-1}, ..., b_0]
    den_coeffs: Vec<f64>, // 分母係数 [a_n, a_{n-1}, ..., a_0]
    states: Vec<f64>,     // 状態変数
}

impl TransferFunction {
    pub fn new(num_coeffs: Vec<f64>, den_coeffs: Vec<f64>, initial_states: Option<Vec<f64>>) -> Self {
        let order = den_coeffs.len() - 1;
        let states = initial_states.unwrap_or_else(|| vec![0.0; order]);
        assert_eq!(states.len(), order, "状態変数の数は分母の次数と一致する必要があります");

        // 分母の最高次係数で正規化
        let a_n = den_coeffs[0];
        let num_norm: Vec<f64> = num_coeffs.iter().map(|c| c / a_n).collect();
        let den_norm: Vec<f64> = den_coeffs.iter().map(|c| c / a_n).collect();

        TransferFunction {
            num_coeffs: num_norm,
            den_coeffs: den_norm,
            states,
        }
    }

    pub fn order(&self) -> usize {
        self.den_coeffs.len() - 1
    }

    pub fn states(&self) -> &[f64] {
        &self.states
    }
}

impl Block for TransferFunction {
    fn output(&self, _t: f64) -> f64 {
        let n = self.order();
        let mut output = 0.0;

        // 直接項の計算
        let num_diff = self.num_coeffs.len() as isize - self.den_coeffs.len() as isize;

        // 出力 y = b_0*x_1 + b_1*x_2 + ... + b_m*x_{m+1} (+ 直接項)
        for (i, b_i) in self.num_coeffs.iter().enumerate().skip(1) {
            let idx = n.saturating_sub(i);
            if idx < self.states.len() {
                output += b_i * self.states[idx];
            }
        }

        // b_0 * (入力項) は update で処理
        output
    }

    fn update(&mut self, input: f64, dt: f64) {
        let n = self.order();

        // 可制御正準形での状態方程式更新
        // dx_i/dt = x_{i+1} (i=1,...,n-1)
        // dx_n/dt = -a_0*x_1 - a_1*x_2 - ... - a_{n-1}*x_n + input

        if n == 0 {
            return;
        }

        // 最後の状態の微分
        let dxn = -self.den_coeffs[n] * self.states[0]
            - if n >= 2 { self.den_coeffs[n-1] * self.states[1] } else { 0.0 }
            - if n >= 3 { self.den_coeffs[n-2] * self.states[2] } else { 0.0 }
            + input;

        // 前方オイラー法で状態更新（後ろから更新）
        for i in (1..n).rev() {
            self.states[i] += self.states[i-1] * dt;
        }
        self.states[0] += dxn * dt;
    }
}

// ============================================================================
// 状態空間モデル
// ============================================================================

/// 線形状態空間モデル
/// dx/dt = A*x + B*u
/// y = C*x + D*u
pub struct StateSpace {
    a: Vec<Vec<f64>>, // n x n
    b: Vec<f64>,      // n x 1
    c: Vec<f64>,      // 1 x n
    d: f64,           // 1 x 1
    states: Vec<f64>, // n x 1
    order: usize,
}

impl StateSpace {
    pub fn new(
        a: Vec<Vec<f64>>,
        b: Vec<f64>,
        c: Vec<f64>,
        d: f64,
        initial_states: Option<Vec<f64>>,
    ) -> Self {
        let n = a.len();
        assert_eq!(a[0].len(), n, "A行列は正方行列である必要があります");
        assert_eq!(b.len(), n, "Bベクトルの次元はAと一致する必要があります");
        assert_eq!(c.len(), n, "Cベクトルの次元はAと一致する必要があります");

        let states = initial_states.unwrap_or_else(|| vec![0.0; n]);
        assert_eq!(states.len(), n);

        StateSpace { a, b, c, d, states, order: n }
    }

    pub fn states(&self) -> &[f64] {
        &self.states
    }

    /// 2次系の標準形を作成
    /// dx1/dt = x2
    /// dx2/dt = -wn^2*x1 - 2*zeta*wn*x2 + input
    /// y = K*wn^2*x1
    pub fn second_order(omega_n: f64, zeta: f64, gain: f64) -> Self {
        let a = vec![
            vec![0.0, 1.0],
            vec![-omega_n * omega_n, -2.0 * zeta * omega_n],
        ];
        let b = vec![0.0, 1.0];
        let c = vec![gain * omega_n * omega_n, 0.0];
        let d = 0.0;

        StateSpace::new(a, b, c, d, None)
    }
}

impl Block for StateSpace {
    fn output(&self, _t: f64) -> f64 {
        let mut y = self.d * 0.0; // 入力はupdateで使用
        for (i, c_i) in self.c.iter().enumerate() {
            y += c_i * self.states[i];
        }
        y
    }

    fn update(&mut self, input: f64, dt: f64) {
        let n = self.order;
        let mut dx = vec![0.0; n];

        // dx/dt = A*x + B*u
        for i in 0..n {
            for j in 0..n {
                dx[i] += self.a[i][j] * self.states[j];
            }
            dx[i] += self.b[i] * input;
        }

        // 前方オイラー法
        for i in 0..n {
            self.states[i] += dx[i] * dt;
        }
    }
}

// ============================================================================
// シミュレーションエンジン
// ============================================================================

/// シミュレーション結果を格納
#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub time: Vec<f64>,
    pub outputs: Vec<Vec<f64>>, // 各ブロックの出力時系列
}

impl SimulationResult {
    pub fn new() -> Self {
        SimulationResult {
            time: Vec::new(),
            outputs: Vec::new(),
        }
    }

    pub fn add(&mut self, t: f64, signals: Vec<f64>) {
        self.time.push(t);
        self.outputs.push(signals);
    }

    /// 特定の信号を取得
    pub fn get_signal(&self, index: usize) -> Vec<f64> {
        self.outputs.iter().map(|v| v[index]).collect()
    }

    /// CSV形式で出力
    pub fn to_csv(&self, headers: Vec<String>) -> String {
        let mut csv = String::new();
        csv.push_str("time,");
        csv.push_str(&headers.join(","));
        csv.push('
');

        for (t, signals) in self.time.iter().zip(self.outputs.iter()) {
            csv.push_str(&format!("{:.6}", t));
            for s in signals {
                csv.push_str(&format!(", {:.6}", s));
            }
            csv.push('
');
        }

        csv
    }
}

/// シミュレーション設定
pub struct SimulationConfig {
    pub start_time: f64,
    pub end_time: f64,
    pub time_step: f64,
}

impl SimulationConfig {
    pub fn new(start_time: f64, end_time: f64, time_step: f64) -> Self {
        assert!(time_step > 0.0, "タイムステップは正である必要があります");
        assert!(end_time > start_time, "終了時刻は開始時刻より後である必要があります");
        SimulationConfig {
            start_time,
            end_time,
            time_step,
        }
    }
}

/// 簡易シミュレーションランナー
pub struct Simulation {
    config: SimulationConfig,
}

impl Simulation {
    pub fn new(config: SimulationConfig) -> Self {
        Simulation { config }
    }

    /// クロージャを使用した柔軟なシミュレーション実行
    /// step_fn: |t, dt, states| -> new_states
    /// output_fn: |t, states| -> outputs
    pub fn run<F, G>(
        &self,
        mut step_fn: F,
        mut output_fn: G,
        initial_states: Vec<f64>,
    ) -> SimulationResult
    where
        F: FnMut(f64, f64, &mut [f64]),
        G: FnMut(f64, &[f64]) -> Vec<f64>,
    {
        let mut result = SimulationResult::new();
        let mut states = initial_states;
        let mut t = self.config.start_time;

        while t <= self.config.end_time {
            let outputs = output_fn(t, &states);
            result.add(t, outputs);

            step_fn(t, self.config.time_step, &mut states);
            t += self.config.time_step;
        }

        result
    }
}

// ============================================================================
// ユーティリティ
// ============================================================================

/// ルンゲ・クッタ法（4次）による数値積分
pub fn rk4_step(f: impl Fn(f64, &[f64]) -> Vec<f64>, state: &mut [f64], input: f64, dt: f64) {
    let n = state.len();

    let k1 = f(input, state);

    let mut state_k2: Vec<f64> = state.iter().zip(k1.iter()).map(|(s, k)| s + dt * 0.5 * k).collect();
    let k2 = f(input, &state_k2);

    let mut state_k3: Vec<f64> = state.iter().zip(k2.iter()).map(|(s, k)| s + dt * 0.5 * k).collect();
    let k3 = f(input, &state_k3);

    let mut state_k4: Vec<f64> = state.iter().zip(k3.iter()).map(|(s, k)| s + dt * k).collect();
    let k4 = f(input, &state_k4);

    for i in 0..n {
        state[i] += (dt / 6.0) * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
    }
}

// ============================================================================
// テスト
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step() {
        let step = Step::new(5.0, 1.0);
        assert_eq!(step.output(0.0), 0.0);
        assert_eq!(step.output(1.0), 5.0);
        assert_eq!(step.output(2.0), 5.0);
    }

    #[test]
    fn test_sum() {
        let sum = Sum::new(vec![1.0, -1.0]);
        assert_eq!(sum.compute(vec![5.0, 3.0]), 2.0);
    }

    #[test]
    fn test_gain() {
        let gain = Gain::new(2.5);
        assert_eq!(gain.compute(4.0), 10.0);
    }

    #[test]
    fn test_integrator() {
        let mut integ = Integrator::new(0.0);
        integ.update(1.0, 0.1);
        assert!((integ.output(0.0) - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_saturation() {
        let sat = Saturation::new(1.0, -1.0);
        assert_eq!(sat.compute(2.0), 1.0);
        assert_eq!(sat.compute(-2.0), -1.0);
        assert_eq!(sat.compute(0.5), 0.5);
    }

    #[test]
    fn test_pid() {
        let mut pid = Pid::new(1.0, 0.0, 0.0);
        assert_eq!(pid.output(5.0, 0.01), 5.0);
    }

    #[test]
    fn test_first_order_lag() {
        let mut lag = FirstOrderLag::new(2.0, 0.5, 0.0);
        // ステップ入力に対する応答をシミュレーション
        let dt = 0.001;
        let mut y = 0.0;
        for _ in 0..10000 {
            y = lag.output(0.0);
            lag.update(1.0, dt);
        }
        // 時定数0.5秒後、約63%に到達
        assert!((y - 2.0 * 0.632).abs() < 0.05);
    }

    #[test]
    fn test_state_space_second_order() {
        let mut ss = StateSpace::second_order(10.0, 0.7, 1.0);
        let dt = 0.0001;
        let mut y = 0.0;
        for _ in 0..10000 {
            y = ss.output(0.0);
            ss.update(1.0, dt);
        }
        // ステップ入力に対する定常値は gain = 1.0
        assert!((y - 1.0).abs() < 0.01);
    }
}
