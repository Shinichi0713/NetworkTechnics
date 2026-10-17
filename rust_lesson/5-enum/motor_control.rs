
//! DCモータ速度制御シミュレーション例
//!
//! DCモータモデル（状態空間モデル）:
//!   dx1/dt = -(R/L)*x1 - (Ke/L)*x2 + (1/L)*u  ... 電機子電流
//!   dx2/dt = (Kt/J)*x1 - (B/J)*x2              ... 回転速度
//!   y = x2                                      ... 出力: 角速度
//!
//! ここで:
//!   R = 電機子抵抗 [Ω], L = 電機子インダクタンス [H]
//!   Ke = 逆起電力定数 [V/(rad/s)], Kt = トルク定数 [Nm/A]
//!   J = 慣性モーメント [kg·m²], B = 粘性摩擦係数 [N·m·s/rad]

use rust_simulink::{
    Step, Pid, Sum, StateSpace, Simulation, SimulationConfig, SimulationResult,
    Ramp, Saturation,
};

fn main() {
    println!("=== DCモータ速度制御シミュレーション ===");

    // DCモータパラメータ（例: 小型DCモータ）
    let r = 2.0;       // Ω
    let l = 0.023;     // H
    let ke = 0.052;    // V/(rad/s)
    let kt = 0.052;    // Nm/A
    let j = 0.00001;   // kg·m²
    let b = 0.000001;  // N·m·s/rad

    // 状態空間表現
    // x = [i_a, omega]^T
    // dx/dt = A*x + B*u
    let a = vec![
        vec![-r / l, -ke / l],
        vec![kt / j, -b / j],
    ];
    let b_vec = vec![1.0 / l, 0.0];
    let c = vec![0.0, 1.0]; // 出力は角速度
    let d = 0.0;

    // シミュレーション設定
    let config = SimulationConfig::new(0.0, 0.5, 0.0001);
    let dt = config.time_step;

    // ブロック初期化
    let mut reference = Ramp::new(100.0, 0.0, 0.0); // 0から100 rad/sへのランプ
    let mut sum = Sum::new(vec![1.0, -1.0]);
    let mut pid = Pid::new(0.5, 5.0, 0.001)
        .with_anti_windup(-12.0, 12.0); // 電圧制限 ±12V
    let mut motor = StateSpace::new(a, b_vec, c, d, Some(vec![0.0, 0.0]));
    let mut saturation = Saturation::new(12.0, -12.0); // 電圧飽和

    // シミュレーション実行
    let mut result = SimulationResult::new();
    let mut t = config.start_time;

    while t <= config.end_time {
        let r = reference.output(t);
        let y = motor.output(t);
        let e = sum.compute(vec![r, y]);
        let u_raw = pid.output(e, dt);
        let u = saturation.compute(u_raw); // 電圧飽和

        result.add(t, vec![r, y, u, e]);
        motor.update(u, dt);

        t += dt;
    }

    // 結果表示
    println!("
時間[ms]	指令値[rad/s]	出力[rad/s]	電圧[V]	偏差");
    println!("----------------------------------------------------------------");
    let skip = result.time.len() / 15;
    for i in (0..result.time.len()).step_by(skip.max(1)) {
        let s = &result.outputs[i];
        println!("{:.1}		{:.1}		{:.1}		{:.2}	{:.2}",
            result.time[i] * 1000.0, s[0], s[1], s[2], s[3]);
    }

    // CSV保存
    let csv = result.to_csv(vec![
        "reference_rad_s".to_string(),
        "speed_rad_s".to_string(),
        "voltage_V".to_string(),
        "error".to_string(),
    ]);
    std::fs::write("motor_control_result.csv", csv).expect("CSV書き込み失敗");
    println!("
結果を motor_control_result.csv に保存しました。");
}
