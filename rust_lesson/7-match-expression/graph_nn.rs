use anyhow::Result;
use petgraph::graph::UnGraph;
use tch::{nn, nn::OptimizerConfig, Device, Kind, Tensor};

// -----------------------------------------------------------------------------
// 1. GCN (Graph Convolutional Network) 層の定義
// -----------------------------------------------------------------------------
// H^{(l+1)} = \sigma( \tilde{D}^{-1/2} \tilde{A} \tilde{D}^{-1/2} H^{(l)} W^{(l)} )

#[derive(Debug)]
struct GcnConv {
    weight: Tensor,
    bias: Tensor,
}

impl GcnConv {
    fn new(vs: nn::Path, in_features: i64, out_features: i64) -> Self {
        let weight = vs.var(
            "weight",
            &[in_features, out_features],
            nn::Init::KaimingUniform,
        );
        let bias = vs.var("bias", &[out_features], nn::Init::Const(0.0));
        GcnConv { weight, bias }
    }

    /// フォワードパス: 特徴量テンソル H と 正規化隣接行列 A_norm を入力
    fn forward(&self, x: &Tensor, adj_norm: &Tensor) -> Tensor {
        // 1. 線形変換: X * W
        let support = x.matmul(&self.weight);
        // 2. グラフ構造に沿ったメッセージパッシング (隣接ノード情報の集約): A_norm * (X * W)
        let output = adj_norm.matmul(&support);
        // 3. バイアス追加
        output + &self.bias
    }
}

// -----------------------------------------------------------------------------
// 2. 2層 GCN モデルの構築
// -----------------------------------------------------------------------------

struct GcnNet {
    conv1: GcnConv,
    conv2: GcnConv,
}

impl GcnNet {
    fn new(vs: nn::Path, in_dim: i64, hidden_dim: i64, out_dim: i64) -> Self {
        let conv1 = GcnConv::new(&vs / "conv1", in_dim, hidden_dim);
        let conv2 = GcnConv::new(&vs / "conv2", hidden_dim, out_dim);
        GcnNet { conv1, conv2 }
    }

    fn forward(&self, x: &Tensor, adj_norm: &Tensor) -> Tensor {
        // Layer 1 + ReLU
        let h = self.conv1.forward(x, adj_norm).relu();
        // Layer 2 (ロジット出力)
        self.conv2.forward(&h, adj_norm)
    }
}

// -----------------------------------------------------------------------------
// 3. petgraph から 隣接行列 A_norm（自己ループ含む正規化行列）への変換
// -----------------------------------------------------------------------------

fn build_normalized_adjacency(
    graph: &UnGraph<(), ()>,
    num_nodes: usize,
    device: Device,
) -> Tensor {
    // A_hat = A + I (自己ループの追加)
    let mut adj = Tensor::zeros(&[num_nodes as i64, num_nodes as i64], (Kind::Float, device));

    for i in 0..num_nodes {
        let _ = adj.get(i as i64).get(i as i64).fill_(1.0); // 自己ループ
    }

    for edge in graph.raw_edges() {
        let u = edge.source().index() as i64;
        let v = edge.target().index() as i64;
        let _ = adj.get(u).get(v).fill_(1.0);
        let _ = adj.get(v).get(u).fill_(1.0);
    }

    // 次数行列 D_hat の計算と D^{-1/2} A_hat D^{-1/2} の正規化
    let degree = adj.sum_dim_intlist(&[1], false, Kind::Float);
    let deg_inv_sqrt = degree.pow_tensor_scalar(-0.5);
    let deg_matrix = Tensor::diag(&deg_inv_sqrt);

    // Symmetric Normalization
    deg_matrix.matmul(&adj).matmul(&deg_matrix)
}

// -----------------------------------------------------------------------------
// 4. メインルーチン (学習・推論パイプライン)
// -----------------------------------------------------------------------------

fn main() -> Result<()> {
    println!("=== Rust + LibTorch (tch-rs) による GNN ノード状態分析 ===");

    let device = Device::Cpu;

    // --- (A) グラフ構造の準備 (petgraph) ---
    let mut pet_graph = UnGraph::<(), ()>::new_undirected();
    let n0 = pet_graph.add_node(()); // Core-Router
    let n1 = pet_graph.add_node(()); // Dist-Switch-A
    let n2 = pet_graph.add_node(()); // Dist-Switch-B
    let n3 = pet_graph.add_node(()); // Web-Server-01
    let n4 = pet_graph.add_node(()); // Web-Server-02
    let n5 = pet_graph.add_node(()); // DB-Server

    pet_graph.add_edge(n0, n1, ());
    pet_graph.add_edge(n0, n2, ());
    pet_graph.add_edge(n1, n3, ());
    pet_graph.add_edge(n1, n4, ());
    pet_graph.add_edge(n2, n5, ());
    pet_graph.add_edge(n1, n2, ());

    let num_nodes = pet_graph.node_count();

    // --- (B) ノードの特徴量テンソル (X) の準備 ---
    // 特徴量次元 3: [レイテンシ(ms), トラフィック負荷率(0-1), ロール種別(0:Router, 1:Switch, 2:Server)]
    let raw_features: Vec<f32> = vec![
        1.2, 0.95, 0.0, // n0: 高負荷ルーター
        2.5, 0.85, 1.0, // n1: 高負荷スイッチ
        1.1, 0.20, 1.0, // n2: 正常スイッチ
        5.0, 0.70, 2.0, // n3: Web-Server-01
        1.8, 0.15, 2.0, // n4: Web-Server-02
        2.0, 0.40, 2.0, // n5: DB-Server
    ];
    let x = Tensor::from_slice(&raw_features)
        .view([num_nodes as i64, 3])
        .to_device(device);

    // 教師ラベル Y (0: 正常/低リスク, 1: 警戒, 2: 危険/SPOFボトルネック)
    let labels = Tensor::from_slice(&[2i64, 2, 0, 1, 0, 0]).to_device(device);

    // --- (C) 隣接行列の正規化計算 ---
    let adj_norm = build_normalized_adjacency(&pet_graph, num_nodes, device);

    // --- (D) モデル・最適化手法の初期化 ---
    let vs = nn::VarStore::new(device);
    let in_dim = 3;
    let hidden_dim = 16;
    let out_classes = 3; // 3クラス分類

    let model = GcnNet::new(vs.root(), in_dim, hidden_dim, out_classes);
    let mut opt = nn::Adam::default().build(&vs, 1e-2)?;

    // --- (E) GNNの学習ループ ---
    println!("\nGCN モデルの学習を開始します...");
    for epoch in 1..=100 {
        let logits = model.forward(&x, &adj_norm);
        
        // クロスエントロピー損失の計算
        let loss = logits.cross_entropy_for_logits(&labels);

        opt.backward_step(&loss);

        if epoch % 20 == 0 {
            let loss_val: f64 = loss.double_value(&[]);
            println!("Epoch {:3} | Loss: {:.6}", epoch, loss_val);
        }
    }

    // --- (F) ノード分析・推論結果の評価 ---
    println!("\n--- ノード状態の分析・分類結果 ---");
    let final_logits = model.forward(&x, &adj_norm);
    let predictions = final_logits.argmax(-1, false);

    let node_names = [
        "Core-Router",
        "Dist-Switch-A",
        "Dist-Switch-B",
        "Web-Server-01",
        "Web-Server-02",
        "DB-Server",
    ];

    for i in 0..num_nodes {
        let pred_class: i64 = predictions.get(i as i64).try_into()?;
        let status_str = match pred_class {
            0 => "\x1b[32m[正常 (Normal)]\x1b[0m",
            1 => "\x1b[33m[警戒 (Warning)]\x1b[0m",
            2 => "\x1b[31m[危険 (Critical SPOF/Bottleneck)]\x1b[0m",
            _ => "Unknown",
        };

        println!("ノード {:2}: {:<16} => 分析結果: {}", i, node_names[i], status_str);
    }

    Ok(())
}