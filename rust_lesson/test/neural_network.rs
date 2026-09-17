%%writefile /content/rl_cartpole.rs

use std::f32::consts::PI;
use rand::distributions::{Distribution, Uniform};
use rand::seq::index::sample;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

// ===================== 1. Tensor（行列） =====================
#[derive(Clone)]
struct Tensor {
    data: Vec<f32>,
    rows: usize,
    cols: usize,
}

impl Tensor {
    fn new(rows: usize, cols: usize) -> Self {
        Tensor {
            data: vec![0.0; rows * cols],
            rows,
            cols,
        }
    }
    
    fn from_vec(data: Vec<f32>, rows: usize, cols: usize) -> Self {
        assert_eq!(data.len(), rows * cols);
        Tensor { data, rows, cols }
    }
    
    fn get(&self, i: usize, j: usize) -> f32 {
        self.data[i * self.cols + j]
    }
    
    fn set(&mut self, i: usize, j: usize, val: f32) {
        self.data[i * self.cols + j] = val;
    }
    
    fn size(&self) -> usize {
        self.rows * self.cols
    }
}

// ===================== 2. 線形層（Linear） =====================
struct Linear {
    weight: Tensor,  // [out_features, in_features]
    bias: Tensor,     // [out_features, 1]
    grad_w: Tensor,
    grad_b: Tensor,
    last_input: Tensor,
}

impl Linear {
    fn new(in_feat: usize, out_feat: usize, rng: &mut StdRng) -> Self {
        let scale = (2.0 / (in_feat + out_feat) as f32).sqrt();
        let dist = rand::distributions::Normal::new(0.0, scale as f64).unwrap();
        
        let mut w_data = vec![0.0; out_feat * in_feat];
        for v in w_data.iter_mut() {
            *v = dist.sample(rng) as f32;
        }
        
        Linear {
            weight: Tensor::from_vec(w_data, out_feat, in_feat),
            bias: Tensor::new(out_feat, 1),
            grad_w: Tensor::new(out_feat, in_feat),
            grad_b: Tensor::new(out_feat, 1),
            last_input: Tensor::new(1, in_feat),
        }
    }
    
    fn forward(&mut self, x: &Tensor) -> Tensor {
        self.last_input = x.clone();
        let mut out = Tensor::new(x.rows, self.weight.rows);
        for i in 0..x.rows {
            for j in 0..self.weight.rows {
                let mut sum = self.bias.get(j, 0);
                for k in 0..x.cols {
                    sum += x.get(i, k) * self.weight.get(j, k);
                }
                out.set(i, j, sum);
            }
        }
        out
    }
    
    fn zero_grad(&mut self) {
        self.grad_w.data.fill(0.0);
        self.grad_b.data.fill(0.0);
    }
}

// ===================== 3. 活性化関数 =====================
fn relu_inplace(tensor: &mut Tensor) {
    for v in tensor.data.iter_mut() {
        if *v < 0.0 {
            *v = 0.0;
        }
    }
}

fn relu_backward(grad_in: &mut Tensor, input: &Tensor, grad_out: &Tensor) {
    for i in 0..input.rows {
        for j in 0..input.cols {
            let val = if input.get(i, j) > 0.0 { grad_out.get(i, j) } else { 0.0 };
            grad_in.set(i, j, val);
        }
    }
}

fn softmax(probs: &mut Tensor, logits: &Tensor) {
    let max_val = logits.data.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.0;
    for j in 0..logits.cols {
        let e = (logits.get(0, j) - max_val).exp();
        probs.set(0, j, e);
        sum += e;
    }
    for j in 0..logits.cols {
        probs.set(0, j, probs.get(0, j) / sum);
    }
}

// ===================== 4. 逆伝播（線形層） =====================
fn linear_backward(layer: &mut Linear, grad_out: &Tensor) {
    let input = &layer.last_input;
    for i in 0..grad_out.rows {
        for j in 0..grad_out.cols {
            let g = layer.grad_b.get(j, 0) + grad_out.get(i, j);
            layer.grad_b.set(j, 0, g);
        }
    }
    
    for i in 0..layer.weight.rows {
        for j in 0..layer.weight.cols {
            let mut sum = 0.0;
            for k in 0..grad_out.rows {
                sum += grad_out.get(k, i) * input.get(k, j);
            }
            layer.grad_w.set(i, j, layer.grad_w.get(i, j) + sum);
        }
    }
}

// ===================== 5. Adamオプティマイザ =====================
struct Adam {
    lr: f32,
    b1: f32,
    b2: f32,
    eps: f32,
    t: i32,
    m_w: Tensor,
    v_w: Tensor,
    m_b: Tensor,
    v_b: Tensor,
}

impl Adam {
    fn new(layer: &Linear, lr: f32) -> Self {
        Adam {
            lr,
            b1: 0.9,
            b2: 0.999,
            eps: 1e-8,
            t: 0,
            m_w: Tensor::new(layer.weight.rows, layer.weight.cols),
            v_w: Tensor::new(layer.weight.rows, layer.weight.cols),
            m_b: Tensor::new(layer.bias.rows, layer.bias.cols),
            v_b: Tensor::new(layer.bias.rows, layer.bias.cols),
        }
    }
    
    fn step(&mut self, layer: &mut Linear) {
        self.t += 1;
        let lr_t = self.lr * (1.0 - self.b2.powi(self.t)).sqrt() / (1.0 - self.b1.powi(self.t));
        
        for i in 0..layer.weight.size() {
            self.m_w.data[i] = self.b1 * self.m_w.data[i] + (1.0 - self.b1) * layer.grad_w.data[i];
            self.v_w.data[i] = self.b2 * self.v_w.data[i] + (1.0 - self.b2) * layer.grad_w.data[i] * layer.grad_w.data[i];
            layer.weight.data[i] -= lr_t * self.m_w.data[i] / (self.v_w.data[i].sqrt() + self.eps);
        }
        
        for i in 0..layer.bias.size() {
            self.m_b.data[i] = self.b1 * self.m_b.data[i] + (1.0 - self.b1) * layer.grad_b.data[i];
            self.v_b.data[i] = self.b2 * self.v_b.data[i] + (1.0 - self.b2) * layer.grad_b.data[i] * layer.grad_b.data[i];
            layer.bias.data[i] -= lr_t * self.m_b.data[i] / (self.v_b.data[i].sqrt() + self.eps);
        }
    }
}

// ===================== 6. Actor-Critic ネットワーク =====================
struct ActorCritic {
    shared1: Linear,
    shared2: Linear,
    actor_head: Linear,
    critic_head: Linear,
    
    h1: Tensor,
    h1_relu: Tensor,
    h2: Tensor,
    h2_relu: Tensor,
    logits: Tensor,
    value: Tensor,
}

impl ActorCritic {
    fn new(state_dim: usize, hidden_dim: usize, action_dim: usize, rng: &mut StdRng) -> Self {
        ActorCritic {
            shared1: Linear::new(state_dim, hidden_dim, rng),
            shared2: Linear::new(hidden_dim, hidden_dim, rng),
            actor_head: Linear::new(hidden_dim, action_dim, rng),
            critic_head: Linear::new(hidden_dim, 1, rng),
            h1: Tensor::new(1, hidden_dim),
            h1_relu: Tensor::new(1, hidden_dim),
            h2: Tensor::new(1, hidden_dim),
            h2_relu: Tensor::new(1, hidden_dim),
            logits: Tensor::new(1, action_dim),
            value: Tensor::new(1, 1),
        }
    }
    
    fn forward(&mut self, state: &Tensor) {
        self.h1 = self.shared1.forward(state);
        self.h1_relu = self.h1.clone();
        relu_inplace(&mut self.h1_relu);
        
        self.h2 = self.shared2.forward(&self.h1_relu);
        self.h2_relu = self.h2.clone();
        relu_inplace(&mut self.h2_relu);
        
        self.logits = self.actor_head.forward(&self.h2_relu);
        self.value = self.critic_head.forward(&self.h2_relu);
    }
    
    fn zero_grad(&mut self) {
        self.shared1.zero_grad();
        self.shared2.zero_grad();
        self.actor_head.zero_grad();
        self.critic_head.zero_grad();
    }
    
    fn backward(&mut self, grad_actor: &Tensor, grad_critic: &Tensor) {
        linear_backward(&mut self.critic_head, grad_critic);
        let mut grad_h2_critic = Tensor::new(1, self.h2_relu.cols);
        for i in 0..self.h2_relu.cols {
            let mut sum = 0.0;
            for j in 0..self.critic_head.weight.rows {
                sum += grad_critic.get(0, j) * self.critic_head.weight.get(j, i);
            }
            grad_h2_critic.set(0, i, sum);
        }
        
        linear_backward(&mut self.actor_head, grad_actor);
        let mut grad_h2_actor = Tensor::new(1, self.h2_relu.cols);
        for i in 0..self.h2_relu.cols {
            let mut sum = 0.0;
            for j in 0..self.actor_head.weight.rows {
                sum += grad_actor.get(0, j) * self.actor_head.weight.get(j, i);
            }
            grad_h2_actor.set(0, i, sum);
        }
        
        let mut grad_h2 = Tensor::new(1, self.h2_relu.cols);
        for i in 0..self.h2_relu.cols {
            grad_h2.set(0, i, grad_h2_critic.get(0, i) + grad_h2_actor.get(0, i));
        }
        
        let mut grad_h2_pre = Tensor::new(1, self.h2.rows);
        relu_backward(&mut grad_h2_pre, &self.h2, &grad_h2);
        linear_backward(&mut self.shared2, &grad_h2_pre);
        
        let mut grad_h1 = Tensor::new(1, self.h1.cols);
        for i in 0..self.h1.cols {
            let mut sum = 0.0;
            for j in 0..self.shared2.weight.rows {
                sum += grad_h2_pre.get(0, j) * self.shared2.weight.get(j, i);
            }
            grad_h1.set(0, i, sum);
        }
        
        let mut grad_h1_pre = Tensor::new(1, self.h1.rows);
        relu_backward(&mut grad_h1_pre, &self.h1, &grad_h1);
        linear_backward(&mut self.shared1, &grad_h1_pre);
    }
    
    fn update(&mut self, opt1: &mut Adam, opt2: &mut Adam, opt3: &mut Adam, opt4: &mut Adam) {
        opt1.step(&mut self.shared1);
        opt2.step(&mut self.shared2);
        opt3.step(&mut self.actor_head);
        opt4.step(&mut self.critic_head);
    }
}

// ===================== 7. CartPole 環境 =====================
struct CartPoleEnv {
    x: f32,
    x_dot: f32,
    theta: f32,
    theta_dot: f32,
    step_count: i32,
}

impl CartPoleEnv {
    fn new() -> Self {
        let mut env = CartPoleEnv {
            x: 0.0, x_dot: 0.0, theta: 0.0, theta_dot: 0.0, step_count: 0,
        };
        env.reset();
        env
    }
    
    fn reset(&mut self) {
        let mut rng = StdRng::seed_from_u64(123);
        let dist = Uniform::new(-0.05_f32, 0.05_f32);
        self.x = dist.sample(&mut rng);
        self.x_dot = dist.sample(&mut rng);
        self.theta = dist.sample(&mut rng);
        self.theta_dot = dist.sample(&mut rng);
        self.step_count = 0;
    }
    
    fn get_state(&self) -> Vec<f32> {
        vec![self.x, self.x_dot, self.theta, self.theta_dot]
    }
    
    fn step(&mut self, action: usize) -> (f32, bool) {
        let force = if action == 1 { 10.0 } else { -10.0 };
        let g = 9.8;
        let mc = 1.0;
        let mp = 0.1;
        let mt = mc + mp;
        let l = 0.5;
        let mpl = mp * l;
        let dt = 0.02;
        
        let ct = self.theta.cos();
        let st = self.theta.sin();
        
        let temp = (force + mpl * self.theta_dot * self.theta_dot * st) / mt;
        let thetaacc = (g * st - ct * temp) / (l * (4.0/3.0 - mp * ct * ct / mt));
        let xacc = temp - mpl * thetaacc * ct / mt;
        
        self.x += dt * self.x_dot;
        self.x_dot += dt * xacc;
        self.theta += dt * self.theta_dot;
        self.theta_dot += dt * thetaacc;
        
        self.step_count += 1;
        
        let done = self.x < -2.4 || self.x > 2.4 || self.theta < -0.2095 || self.theta > 0.2095;
        (1.0, done)
    }
}

// ===================== 8. 学習ループ =====================
fn main() {
    const STATE_DIM: usize = 4;
    const HIDDEN_DIM: usize = 128;
    const ACTION_DIM: usize = 2;
    const GAMMA: f32 = 0.99;
    const MAX_EPISODES: i32 = 1000;
    const MAX_STEPS: i32 = 500;
    
    let mut rng = StdRng::seed_from_u64(42);
    
    let mut ac = ActorCritic::new(STATE_DIM, HIDDEN_DIM, ACTION_DIM, &mut rng);
    let mut opt1 = Adam::new(&ac.shared1, 1e-3);
    let mut opt2 = Adam::new(&ac.shared2, 1e-3);
    let mut opt3 = Adam::new(&ac.actor_head, 1e-3);
    let mut opt4 = Adam::new(&ac.critic_head, 1e-3);
    
    let mut env = CartPoleEnv::new();
    
    println!("=== Rust Actor-Critic: CartPole Training ===");
    
    for episode in 0..MAX_EPISODES {
        env.reset();
        let mut state = Tensor::from_vec(env.get_state(), 1, STATE_DIM);
        let mut total_reward = 0.0;
        let mut steps = 0;
        
        for _ in 0..MAX_STEPS {
            ac.forward(&state);
            
            let mut probs = Tensor::new(1, ACTION_DIM);
            softmax(&mut probs, &ac.logits);
            
            let action = if rng.gen::<f32>() < probs.get(0, 0) { 0 } else { 1 };
            
            let (reward, done) = env.step(action);
            total_reward += reward;
            steps += 1;
            
            let next_value = if done {
                0.0
            } else {
                let next_state = Tensor::from_vec(env.get_state(), 1, STATE_DIM);
                ac.forward(&next_state);
                ac.value.get(0, 0)
            };
            
            let current_value = ac.value.get(0, 0);
            let advantage = reward + GAMMA * next_value - current_value;
            
            ac.zero_grad();
            
            let mut grad_actor = Tensor::new(1, ACTION_DIM);
            for j in 0..ACTION_DIM {
                let g = -advantage * probs.get(0, j);
                grad_actor.set(0, j, g);
            }
            let action_g = -advantage * (probs.get(0, action) - 1.0);
            grad_actor.set(0, action, action_g);
            
            let mut grad_critic = Tensor::new(1, 1);
            grad_critic.set(0, 0, 2.0 * advantage);
            
            ac.backward(&grad_actor, &grad_critic);
            ac.update(&mut opt1, &mut opt2, &mut opt3, &mut opt4);
            
            if done {
                break;
            }
            
            state = Tensor::from_vec(env.get_state(), 1, STATE_DIM);
        }
        
        if (episode + 1) % 100 == 0 {
            println!("Episode {}/{} | Reward: {} | Steps: {}", episode + 1, MAX_EPISODES, total_reward, steps);
        }
    }
    
    println!("\n学習完了。");
    
    // 推論テスト
    println!("\n=== 推論テスト（貪欲法） ===");
    env.reset();
    let mut state = Tensor::from_vec(env.get_state(), 1, STATE_DIM);
    let mut test_reward = 0.0;
    
    for _ in 0..MAX_STEPS {
        ac.forward(&state);
        let mut probs = Tensor::new(1, ACTION_DIM);
        softmax(&mut probs, &ac.logits);
        let action = if probs.get(0, 1) > probs.get(0, 0) { 1 } else { 0 };
        
        let (reward, done) = env.step(action);
        test_reward += reward;
        
        if done { break; }
        state = Tensor::from_vec(env.get_state(), 1, STATE_DIM);
    }
    println!("テスト報酬: {} ステップ", test_reward);
}