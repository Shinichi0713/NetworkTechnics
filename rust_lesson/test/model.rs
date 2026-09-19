use ndarray::Array2;
use ort::{inputs, session::Session};
use std::fs::File;
use std::io::Write;
use std::time::Instant;

// --- 1. Rustで実装した CartPole 物理環境 ---
struct CartPoleEnv {
    x: f32,         // カート位置 [m]
    x_dot: f32,     // カート速度 [m/s]
    theta: f32,     // ポール角度 [rad]
    theta_dot: f32, // ポール角速度 [rad/s]
}

impl CartPoleEnv {
    fn new() -> Self {
        Self {
            x: 0.0,
            x_dot: 0.0,
            theta: 0.05, // 初期値としてわずかに傾ける
            theta_dot: 0.0,
        }
    }

    // オイラー法による状態更新 (0: 左へ押す, 1: 右へ押す)
    fn step(&mut self, action: usize) -> bool {
        let gravity = 9.8f32;
        let masscart = 1.0f32;
        let masspole = 0.1f32;
        let total_mass = masscart + masspole;
        let length = 0.5f32;
        let polemass_length = masspole * length;
        let force_mag = 10.0f32;
        let tau = 0.02f32; // 制御周期 20ms

        let force = if action == 1 { force_mag } else { -force_mag };
        let costheta = self.theta.cos();
        let sintheta = self.theta.sin();

        let temp = (force + polemass_length * self.theta_dot.powi(2) * sintheta) / total_mass;
        let thetaacc = (gravity * sintheta - costheta * temp)
            / (length * (4.0 / 3.0 - masspole * costheta.powi(2) / total_mass));
        let xacc = temp - polemass_length * thetaacc * costheta / total_mass;

        // 状態更新
        self.x += tau * self.x_dot;
        self.x_dot += tau * xacc;
        self.theta += tau * self.theta_dot;
        self.theta_dot += tau * thetaacc;

        // 失敗条件の判定
        let failed = self.x < -2.4 || self.x > 2.4 || self.theta < -0.2095 || self.theta > 0.2095;
        !failed
    }

    // モデル入力用の状態ベクトルを取得
    fn get_state_array(&self) -> Array2<f32> {
        Array2::from_shape_vec((1, 4), vec![self.x, self.x_dot, self.theta, self.theta_dot])
            .unwrap()
    }
}

// --- 2. メイン推論・制御ループ ---
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Rust + ONNX Runtime 推論テスト ---");

    // ONNX Runtime セッションの作成
    let session = Session::builder()?
        .commit_from_file("cartpole_dqn.onnx")?;

    let mut env = CartPoleEnv::new();

    // ログ出力用ファイルの作成
    let mut log_file = File::create("rust_cartpole_log.csv")?;
    writeln!(log_file, "step,x,theta")?;

    let mut steps = 0;
    let start_time = Instant::now();

    while steps < 500 {
        // 現在ログの記録
        writeln!(log_file, "{},{},{}", steps, env.x, env.theta)?;

        // 1. 現在の状態をndarrayとして準備
        let state_tensor = env.get_state_array();

        // 2. ONNXモデルによる推論の実行
        let outputs = session.run(inputs!["state" => state_tensor.view()]?)?;
        
        // 3. 出力（Q値）から最適な行動を選択 (Argmax)
        let q_values = outputs["q_values"].try_extract_tensor::<f32>()?;
        let action = if q_values[[0, 1]] > q_values[[0, 0]] { 1 } else { 0 };

        // 4. 物理シミュレータを1ステップ進める
        let alive = env.step(action);
        steps += 1;

        if !alive {
            println!("制御失敗: ステップ {} で倒れました。", steps);
            break;
        }
    }

    let duration = start_time.elapsed();

    if steps >= 500 {
        println!("🎉 制御成功！最大500ステップ維持を達成しました。");
    }

    println!("総ステップ数: {}", steps);
    println!("総計算時間: {:?}", duration);
    if steps > 0 {
        println!(
            "1ステップあたりの平均処理時間: {:?}",
            duration / steps as u32
        );
    }

    Ok(())
}

use rand::seq::SliceRandom;
use rand::thread_rng;
use tch::nn::{self, Module, OptimizerConfig};
use tch::{Kind, Device, Tensor};

// --- 1. Q ネットワークの定義 (PyTorch の nn.Module に相当) ---
#[derive(Debug)]
struct QNetwork {
    fc1: nn::Linear,
    fc2: nn::Linear,
    fc3: nn::Linear,
}

impl QNetwork {
    fn new(vs: &nn::Path, state_dim: i64, action_dim: i64) -> Self {
        let fc1 = nn::linear(vs / "fc1", state_dim, 64, Default::default());
        let fc2 = nn::linear(vs / "fc2", 64, 64, Default::default());
        let fc3 = nn::linear(vs / "fc3", 64, action_dim, Default::default());
        QNetwork { fc1, fc2, fc3 }
    }
}

impl Module for QNetwork {
    fn forward(&self, xs: &Tensor) -> Tensor {
        xs.apply(&self.fc1)
            .relu()
            .apply(&self.fc2)
            .relu()
            .apply(&self.fc3)
    }
}

// --- 2. 経験再生バッファ (Replay Buffer) ---
struct Transition {
    state: Vec<f32>,
    action: i64,
    reward: f32,
    next_state: Vec<f32>,
    done: bool,
}

struct ReplayBuffer {
    buffer: Vec<Transition>,
    capacity: usize,
}

impl ReplayBuffer {
    fn new(capacity: usize) -> Self {
        ReplayBuffer {
            buffer: Vec::with_capacity(capacity),
            capacity,
        }
    }

    fn push(&mut self, transition: Transition) {
        if self.buffer.len() >= self.capacity {
            self.buffer.remove(0);
        }
        self.buffer.push(transition);
    }

    fn sample(&self, batch_size: usize) -> Vec<&Transition> {
        let mut rng = thread_rng();
        self.buffer.choose_multiple(&mut rng, batch_size).collect()
    }

    fn len(&self) -> usize {
        self.buffer.len()
    }
}

// --- 3. メイン学習ループ ---
fn main() {
    let device = Device::Cpu; // GPUを使用する場合は Device::Cuda(0)
    let state_dim = 4;        // CartPoleの状態変数 (x, x_dot, theta, theta_dot)
    let action_dim = 2;       // 行動の種類 (左: 0, 右: 1)

    // 変数ストアと Q ネットワークの初期化
    let vs = nn::VarStore::new(device);
    let q_net = QNetwork::new(&vs.root(), state_dim, action_dim);
    
    // Adam オプティマイザの設定
    let mut optimizer = nn::Adam::default().build(&vs, 1e-3).unwrap();

    let mut replay_buffer = ReplayBuffer::new(10000);
    let batch_size = 64;
    let gamma = 0.99f32;

    println!("--- Rust (tch-rs) による DQN モデル学習の開始 ---");

    // ダミーのデータ追加と学習ステップのシミュレーション
    // (実際の運用では環境の step() 結果を push します)
    for _ in 0..100 {
        replay_buffer.push(Transition {
            state: vec![0.0, 0.1, 0.01, 0.05],
            action: 1,
            reward: 1.0,
            next_state: vec![0.002, 0.12, 0.011, 0.04],
            done: false,
        });
    }

    if replay_buffer.len() >= batch_size {
        let transitions = replay_buffer.sample(batch_size);

        // データの テンソル (Tensor) 変換
        let states: Vec<f32> = transitions.iter().flat_map(|t| t.state.clone()).collect();
        let actions: Vec<i64> = transitions.iter().map(|t| t.action).collect();
        let rewards: Vec<f32> = transitions.iter().map(|t| t.reward).collect();
        let next_states: Vec<f32> = transitions.iter().flat_map(|t| t.next_state.clone()).collect();
        let dones: Vec<f32> = transitions.iter().map(|t| if t.done { 1.0 } else { 0.0 }).collect();

        let state_tensor = Tensor::from_slice(&states)
            .view([batch_size as i64, state_dim])
            .to_device(device);
        let action_tensor = Tensor::from_slice(&actions)
            .view([batch_size as i64, 1])
            .to_device(device);
        let reward_tensor = Tensor::from_slice(&rewards)
            .view([batch_size as i64, 1])
            .to_device(device);
        let next_state_tensor = Tensor::from_slice(&next_states)
            .view([batch_size as i64, state_dim])
            .to_device(device);
        let done_tensor = Tensor::from_slice(&dones)
            .view([batch_size as i64, 1])
            .to_device(device);

        // 1. 現在の Q 値の計算: Q(s, a)
        let current_q = q_net.forward(&state_tensor).gather(1, &action_tensor, false);

        // 2. ターゲット Q 値の計算: r + gamma * max(Q(s')) * (1 - done)
        let next_q = q_net.forward(&next_state_tensor).max_dim(1, false).0.unsqueeze(1);
        let target_q = &reward_tensor + (1.0 - &done_tensor) * gamma * next_q;

        // 3. 損失関数（MSE Loss）の計算と逆伝播・重み更新
        let loss = current_q.mse_loss(&target_q, tch::Reduction::Mean);

        optimizer.zero_grad();
        optimizer.backward_step(&loss);

        println!("Batch Loss: {:.5}", f64::try_from(&loss).unwrap());
    }

    // 学習済み重みの保存 (PyTorch形式: .pt)
    vs.save("cartpole_dqn_rust.pt").unwrap();
    println!("モデルの重みを cartpole_dqn_rust.pt に保存しました。");
}