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