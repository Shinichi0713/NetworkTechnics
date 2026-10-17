
//! カスケード制御シミュレーション例
//!
//! 外側ループ: 位置制御（P制御）
//! 内側ループ: 速度制御（PI制御）
//!
//! 構成:
//!   位置指令 ---> [P制御] ---> 速度指令 ---> [PI制御] ---> 電圧 ---> [モータ] ---> 速度 ---> [積分] ---> 位置
//!      ^                                                                                           |
//!      |-------------------------------------------------------------------------------------------|
//!
//! これは実用的なサーボ制御系の典型例です。

use rust_simulink::{
    Step, Pid, Sum, Integrator, SecondOrderSystem, Simulation, SimulationConfig,
    SimulationResult, Saturation, Gain,
};

fn main() {
    println!("=== カスケード制御（位置・速度2重ループ）シミュレーション ===");

    // シミュレーション設定
    let config = SimulationConfig::new(0.0, 5.0, 0.001);
    let dt = config.time_step;

    // モータモデル（2次系で近似）
    // G(s) = K / (s^2 + 2*zeta*wn*s + wn^2)
    let omega_n = 20.0;  // rad/s
    let zeta = 0.5;
    let motor_gain = 50.0;
    let mut motor = SecondOrderSystem::new(omega_n, zeta, motor_gain, 0.0, 0.0);

    // 位置は速度の積分
    let mut position_integ = Integrator::new(0.0);

    // 外側ループ: 位置P制御
    let mut pos_pid = Pid::new(2.0, 0.0, 0.0); // Kpのみ

    // 内側ループ: 速度PI制御
    let mut vel_pid = Pid::new(0.1, 0.5, 0.0)
        .with_anti_windup(-10.0, 10.0);

    // 加算器
    let mut pos_sum = Sum::new(vec![1.0, -1.0]); // 位置偏差
    let mut vel_sum = Sum::new(vec![1.0, -1.0]); // 速度偏差

    // 電圧飽和
    let saturation = Saturation::new(10.0, -10.0);

    // 位置指令（ステップ）
    let mut pos_ref = Step::new(1.0, 0.0); // 1 radへのステップ

    // シミュレーション実行
    let mut result = SimulationResult::new();
    let mut t = config.start_time;

    while t <= config.end_time {
        // 外側ループ: 位置制御
        let r_pos = pos_ref.output(t);
        let y_pos = position_integ.output(t);
        let e_pos = pos_sum.compute(vec![r_pos, y_pos]);
        let vel_ref = pos_pid.output(e_pos, dt); // 速度指令

        // 内側ループ: 速度制御
        let y_vel = motor.output(t);
        let e_vel = vel_sum.compute(vec![vel_ref, y_vel]);
        let u_raw = vel_pid.output(e_vel, dt);
        let u = saturation.compute(u_raw);

        // 結果記録
        result.add(t, vec![r_pos, y_pos, y_vel, vel_ref, u, e_pos]);

        // 状態更新
        motor.update(u, dt);
        position_integ.update(y_vel, dt);

        t += dt;
    }

    // 結果表示
    println!("
時間[s]	位置指令	位置出力	速度出力	速度指令	電圧	位置偏差");
    println!("------------------------------------------------------------------------");
    let skip = result.time.len() / 15;
    for i in (0..result.time.len()).step_by(skip.max(1)) {
        let s = &result.outputs[i];
        println!("{:.2}	{:.4}	{:.4}	{:.4}	{:.4}	{:.4}	{:.4}",
            result.time[i], s[0], s[1], s[2], s[3], s[4], s[5]);
    }

    // CSV保存
    let csv = result.to_csv(vec![
        "pos_reference".to_string(),
        "position".to_string(),
        "velocity".to_string(),
        "vel_reference".to_string(),
        "voltage".to_string(),
        "pos_error".to_string(),
    ]);
    std::fs::write("cascade_control_result.csv", csv).expect("CSV書き込み失敗");
    println!("
結果を cascade_control_result.csv に保存しました。");
}
