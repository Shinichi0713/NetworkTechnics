
//! PIDフィードバック制御系シミュレーション例
//!
//! 制御系構成:
//!   r(t) [ステップ入力] --->(+)---> e(t) ---> [PID] ---> u(t) ---> [プラント: 1/(s+1)] ---> y(t)
//!                            ^ -                                          |
//!                            |--------------------------------------------|
//!
//! これはSimulinkで最も基本的なフィードバック制御系と同等の構成です。

use rust_simulink::{
    Step, Pid, Sum, FirstOrderLag, Simulation, SimulationConfig, SimulationResult,
    Gain, Saturation, Integrator,
};

fn main() {
    println!("=== PIDフィードバック制御系シミュレーション ===");

    // シミュレーション設定
    let config = SimulationConfig::new(0.0, 10.0, 0.001);
    let dt = config.time_step;

    // ブロックの初期化
    let mut step = Step::new(1.0, 0.0);      // 単位ステップ入力（t=0で振幅1）
    let mut sum = Sum::new(vec![1.0, -1.0]);  // 誤差 = 指令値 - 出力
    let mut pid = Pid::new(2.0, 1.0, 0.1)     // PIDゲイン調整
        .with_anti_windup(-10.0, 10.0);       // アンチウィンドアップ付き
    let mut plant = FirstOrderLag::new(1.0, 1.0, 0.0); // プラント: G(s)=1/(s+1)

    // シミュレーション実行
    let mut result = SimulationResult::new();
    let mut t = config.start_time;

    while t <= config.end_time {
        // 各ブロックの信号を計算
        let r = step.output(t);           // 指令値
        let y = plant.output(t);          // プラント出力
        let e = sum.compute(vec![r, y]);  // 制御偏差
        let u = pid.output(e, dt);        // PID操作量

        // 結果を記録（指令値、出力、操作量、偏差）
        result.add(t, vec![r, y, u, e]);

        // プラントの状態を更新
        plant.update(u, dt);

        t += dt;
    }

    // 結果の表示（100点サンプリング）
    println!("
時間	指令値(r)	出力(y)	操作量(u)	偏差(e)");
    println!("--------------------------------------------------------");
    let skip = result.time.len() / 20;
    for i in (0..result.time.len()).step_by(skip.max(1)) {
        let signals = &result.outputs[i];
        println!("{:.2}	{:.4}		{:.4}	{:.4}		{:.4}",
            result.time[i], signals[0], signals[1], signals[2], signals[3]);
    }

    // 定常値の確認
    let final_output = result.outputs.last().unwrap()[1];
    println!("
定常値: y(∞) = {:.6}", final_output);
    println!("目標値: r = 1.0");
    println!("定常偏差: e_ss = {:.6}", 1.0 - final_output);

    // CSV出力
    let csv = result.to_csv(vec![
        "reference".to_string(),
        "output".to_string(),
        "control_input".to_string(),
        "error".to_string(),
    ]);
    std::fs::write("feedback_pid_result.csv", csv).expect("CSV書き込み失敗");
    println!("
結果を feedback_pid_result.csv に保存しました。");
}
