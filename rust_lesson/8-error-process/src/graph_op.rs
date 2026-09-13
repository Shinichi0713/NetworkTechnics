use std::collections::HashMap;

// ============================================
// 1. 隣接行列 (Adjacency Matrix)
// ============================================
// メリット: 辺の存在確認が O(1)、実装が簡単
// デメリット: メモリ消費が O(V^2)、疎なグラフに非効率

struct AdjacencyMatrix {
    matrix: Vec<Vec<bool>>,
    n: usize,
}

impl AdjacencyMatrix {
    fn new(n: usize) -> Self {
        Self {
            matrix: vec![vec![false; n]; n],
            n,
        }
    }

    // 有向グラフ: u -> v に辺を追加
    fn add_edge(&mut self, u: usize, v: usize) {
        assert!(u < self.n && v < self.n);
        self.matrix[u][v] = true;
    }

    // 無向グラフ用: 両方向に辺を追加
    fn add_undirected_edge(&mut self, u: usize, v: usize) {
        self.add_edge(u, v);
        self.add_edge(v, u);
    }

    fn has_edge(&self, u: usize, v: usize) -> bool {
        assert!(u < self.n && v < self.n);
        self.matrix[u][v]
    }

    fn neighbors(&self, u: usize) -> Vec<usize> {
        assert!(u < self.n);
        (0..self.n)
            .filter(|&v| self.matrix[u][v])
            .collect()
    }
}

// ============================================
// 2. 隣接リスト (Adjacency List)
// ============================================
// メリット: メモリ効率が良い O(V+E)、疎なグラフに最適
// デメリット: 辺の存在確認が O(degree)

struct AdjacencyList {
    list: Vec<Vec<usize>>,
    n: usize,
}

impl AdjacencyList {
    fn new(n: usize) -> Self {
        Self {
            list: vec![vec![]; n],
            n,
        }
    }

    fn add_edge(&mut self, u: usize, v: usize) {
        assert!(u < self.n && v < self.n);
        self.list[u].push(v);
    }

    fn add_undirected_edge(&mut self, u: usize, v: usize) {
        self.add_edge(u, v);
        self.add_edge(v, u);
    }

    fn has_edge(&self, u: usize, v: usize) -> bool {
        assert!(u < self.n && v < self.n);
        self.list[u].contains(&v)
    }

    fn neighbors(&self, u: usize) -> &[usize] {
        assert!(u < self.n);
        &self.list[u]
    }
}

// ============================================
// 3. 重み付き隣接リスト (Weighted Adjacency List)
// ============================================

struct WeightedAdjacencyList {
    list: Vec<Vec<(usize, i32)>>, // (隣接頂点, 重み)
    n: usize,
}

impl WeightedAdjacencyList {
    fn new(n: usize) -> Self {
        Self {
            list: vec![vec![]; n],
            n,
        }
    }

    fn add_edge(&mut self, u: usize, v: usize, w: i32) {
        assert!(u < self.n && v < self.n);
        self.list[u].push((v, w));
    }

    fn add_undirected_edge(&mut self, u: usize, v: usize, w: i32) {
        self.add_edge(u, v, w);
        self.add_edge(v, u, w);
    }

    fn neighbors(&self, u: usize) -> &[(usize, i32)] {
        assert!(u < self.n);
        &self.list[u]
    }
}

// ============================================
// 4. 辺リスト (Edge List)
// ============================================
// メリット: Kruskal法などで使いやすい、メモリ効率が良い
// デメリット: 頂点ごとの隣接関係の取得が遅い

struct EdgeList {
    edges: Vec<(usize, usize)>,
}

impl EdgeList {
    fn new() -> Self {
        Self { edges: vec![] }
    }

    fn add_edge(&mut self, u: usize, v: usize) {
        self.edges.push((u, v));
    }

    fn add_undirected_edge(&mut self, u: usize, v: usize) {
        self.add_edge(u, v);
        self.add_edge(v, u);
    }

    fn get_edges(&self) -> &[(usize, usize)] {
        &self.edges
    }
}

// ============================================
// 5. HashMap を使った動的グラフ
// ============================================
// 頂点が整数でない場合や、頂点数が事前に不明な場合に有効

struct DynamicGraph<T> {
    adj: HashMap<T, Vec<T>>,
}

impl<T: std::cmp::Eq + std::hash::Hash + Clone> DynamicGraph<T> {
    fn new() -> Self {
        Self {
            adj: HashMap::new(),
        }
    }

    fn add_vertex(&mut self, v: T) {
        self.adj.entry(v).or_insert_with(Vec::new);
    }

    fn add_edge(&mut self, u: T, v: T) {
        self.adj.entry(u.clone()).or_insert_with(Vec::new).push(v.clone());
        self.adj.entry(v).or_insert_with(Vec::new);
    }

    fn add_undirected_edge(&mut self, u: T, v: T) {
        self.adj.entry(u.clone()).or_insert_with(Vec::new).push(v.clone());
        self.adj.entry(v.clone()).or_insert_with(Vec::new).push(u);
    }

    fn neighbors(&self, u: &T) -> Option<&Vec<T>> {
        self.adj.get(u)
    }
}

// ============================================
// 使用例
// ============================================

fn main() {
    // 隣接行列の例
    println!("=== Adjacency Matrix ===");
    let mut mat = AdjacencyMatrix::new(4);
    mat.add_undirected_edge(0, 1);
    mat.add_undirected_edge(1, 2);
    mat.add_edge(2, 3);
    println!("0 -> 1? {}", mat.has_edge(0, 1));
    println!("Neighbors of 1: {:?}", mat.neighbors(1));

    // 隣接リストの例
    println!("\n=== Adjacency List ===");
    let mut adj = AdjacencyList::new(4);
    adj.add_undirected_edge(0, 1);
    adj.add_undirected_edge(1, 2);
    adj.add_edge(2, 3);
    println!("Neighbors of 1: {:?}", adj.neighbors(1));

    // 重み付き隣接リストの例
    println!("\n=== Weighted Adjacency List ===");
    let mut wadj = WeightedAdjacencyList::new(4);
    wadj.add_undirected_edge(0, 1, 5);
    wadj.add_edge(1, 2, 3);
    println!("Neighbors of 1: {:?}", wadj.neighbors(1));

    // 辺リストの例
    println!("\n=== Edge List ===");
    let mut edges = EdgeList::new();
    edges.add_undirected_edge(0, 1);
    edges.add_edge(1, 2);
    println!("Edges: {:?}", edges.get_edges());

    // 動的グラフの例
    println!("\n=== Dynamic Graph ===");
    let mut dg = DynamicGraph::new();
    dg.add_undirected_edge("A", "B");
    dg.add_undirected_edge("B", "C");
    println!("Neighbors of B: {:?}", dg.neighbors(&"B"));
}
