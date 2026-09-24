use ndarray::prelude::*;
use rand::distributions::{Distribution, Uniform};

struct DenseLayer {
    weights: Array2<f32>,
    biases: Array1<f32>,
}

impl DenseLayer {
    fn new(in_dim: usize, out_dim: usize) -> Self {
        let mut rng = rand::thread_rng();
        let die = Uniform::new(-0.5f32, 0.5f32);
        
        let weights = Array2::from_shape_fn((in_dim, out_dim), |_| die.sample(&mut rng));
        let biases = Array1::from_shape_fn(out_dim, |_| die.sample(&mut rng));
        
        Self { weights, biases }
    }

    fn forward(&self, input: &Array1<f32>) -> Array1<f32> {
        let mut output = input.dot(&self.weights) + &self.biases;
        // ReLU 活性化関数
        output.mapv_inplace(|x| if x > 0.0 { x } else { 0.0 });
        output
    }
}

fn main() {
    // 3入力 -> 4隠れ層 -> 1出力 の順伝播ネットワーク
    let layer1 = DenseLayer::new(3, 4);
    let layer2 = DenseLayer::new(4, 1);

    let input = array![1.0, -0.5, 2.0];
    let h1 = layer1.forward(&input);
    let output = layer2.forward(&h1);

    println!("Input: {:?}", input);
    println!("Output: {:?}", output);
}


use ndarray::prelude::*;
use rand_distr::{Distribution, Normal};

// --- 1. レイヤートレイトの定義 ---
pub trait Layer {
    // 順伝播
    fn forward(&mut self, input: &Array2<f32>) -> Array2<f32>;
    // 逆伝播（入力勾配 dL/dInput を返す）
    fn backward(&mut self, output_gradient: &Array2<f32>, lr: f32) -> Array2<f32>;
}

// --- 2. 全結合層 (Dense Layer) ---
pub struct Dense {
    pub weights: Array2<f32>,
    pub biases: Array1<f32>,
    input_cache: Option<Array2<f32>>,
}

impl Dense {
    pub fn new(in_features: usize, out_features: usize) -> Self {
        let mut rng = rand::thread_rng();
        // He初期化 (Kaiming Normal)
        let std_dev = (2.0 / in_features as f32).sqrt();
        let normal = Normal::new(0.0, std_dev).unwrap();

        let weights = Array2::from_shape_fn((in_features, out_features), |_| {
            normal.sample(&mut rng)
        });
        let biases = Array1::zeros(out_features);

        Self {
            weights,
            biases,
            input_cache: None,
        }
    }
}

impl Layer for Dense {
    fn forward(&mut self, input: &Array2<f32>) -> Array2<f32> {
        self.input_cache = Some(input.clone());
        // Y = X * W + b
        input.dot(&self.weights) + &self.biases
    }

    fn backward(&mut self, output_gradient: &Array2<f32>, lr: f32) -> Array2<f32> {
        let input = self.input_cache.as_ref().expect("Forward pass required before backward");
        
        // 勾配計算
        // dL/dW = X^T * dL/dY
        let d_weights = input.t().dot(output_gradient);
        // dL/db = sum(dL/dY, axis=0)
        let d_biases = output_gradient.sum_axis(Axis(0));
        // dL/dX = dL/dY * W^T
        let d_input = output_gradient.dot(&self.weights.t());

        // パラメータ更新 (SGD)
        self.weights = &self.weights - &(d_weights * lr);
        self.biases = &self.biases - &(d_biases * lr);

        d_input
    }
}

// --- 3. ReLU 活性化層 ---
pub struct ReLU {
    output_cache: Option<Array2<f32>>,
}

impl ReLU {
    pub fn new() -> Self {
        Self { output_cache: None }
    }
}

impl Layer for ReLU {
    fn forward(&mut self, input: &Array2<f32>) -> Array2<f32> {
        let output = input.mapv(|x| if x > 0.0 { x } else { 0.0 });
        self.output_cache = Some(output.clone());
        output
    }

    fn backward(&mut self, output_gradient: &Array2<f32>, _lr: f32) -> Array2<f32> {
        let output = self.output_cache.as_ref().unwrap();
        // ReLUの微分: 順伝播時の出力が > 0 であれば 1.0, それ以外は 0.0
        output_gradient * &output.mapv(|x| if x > 0.0 { 1.0 } else { 0.0 })
    }
}

// --- 4. Sigmoid 活性化層 ---
pub struct Sigmoid {
    output_cache: Option<Array2<f32>>,
}

impl Sigmoid {
    pub fn new() -> Self {
        Self { output_cache: None }
    }
}

impl Layer for Sigmoid {
    fn forward(&mut self, input: &Array2<f32>) -> Array2<f32> {
        let output = input.mapv(|x| 1.0 / (1.0 + (-x).exp()));
        self.output_cache = Some(output.clone());
        output
    }

    fn backward(&mut self, output_gradient: &Array2<f32>, _lr: f32) -> Array2<f32> {
        let output = self.output_cache.as_ref().unwrap();
        // Sigmoidの微分: sig * (1 - sig)
        let d_sigmoid = output * &(1.0 - output);
        output_gradient * &d_sigmoid
    }
}

// --- 5. 積層モデル (Sequential) ---
pub struct Sequential {
    layers: Vec<Box<dyn Layer>>,
}

impl Sequential {
    pub fn new(layers: Vec<Box<dyn Layer>>) -> Self {
        Self { layers }
    }

    pub fn forward(&mut self, mut x: Array2<f32>) -> Array2<f32> {
        for layer in self.layers.iter_mut() {
            x = layer.forward(&x);
        }
        x
    }

    pub fn backward(&mut self, mut gradient: Array2<f32>, lr: f32) {
        // 逆伝播はレイヤーを後ろから順に処理
        for layer in self.layers.iter_mut().rev() {
            gradient = layer.backward(&gradient, lr);
        }
    }
}

// --- 6. メイン実行コード (XOR問題の学習) ---
fn main() {
    // 2入力 -> 8隠れ層(ReLU) -> 4隠れ層(ReLU) -> 1出力(Sigmoid) の深層モデル構築
    let mut model = Sequential::new(vec![
        Box::new(Dense::new(2, 8)),
        Box::new(ReLU::new()),
        Box::new(Dense::new(8, 4)),
        Box::new(ReLU::new()),
        Box::new(Dense::new(4, 1)),
        Box::new(Sigmoid::new()),
    ]);

    // XOR問題のデータセット (4サンプル, 2特徴量)
    let x = array![[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]];
    let y = array![[0.0], [1.0], [1.0], [0.0]];

    let epochs = 10000;
    let lr = 0.1;

    println!("--- 学習開始 ---");
    for epoch in 0..=epochs {
        // 1. 順伝播
        let predictions = model.forward(x.clone());

        // 2. 二乗誤差 (MSE) の損失計算
        let loss = (&predictions - &y).mapv(|diff| diff.powi(2)).mean().unwrap();

        // 3. 損失の微分 (dL/dPred = 2 * (pred - y) / N)
        let loss_gradient = 2.0 * (&predictions - &y) / (x.nrows() as f32);

        // 4. 逆伝播によるパラメータ更新
        model.backward(loss_gradient, lr);

        if epoch % 2000 == 0 {
            println!("Epoch {:5} | Loss: {:.6}", epoch, loss);
        }
    }

    println!("\n--- 学習完了後の予測結果 ---");
    let final_preds = model.forward(x.clone());
    for i in 0..x.nrows() {
        println!(
            "Input: {:?} -> Pred: {:.4} (Target: {})",
            x.row(i).to_slice().unwrap(),
            final_preds[[i, 0]],
            y[[i, 0]]
        );
    }
}