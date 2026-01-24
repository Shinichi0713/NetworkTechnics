use std::ops::{Add, Mul};

/// 行列構造体
struct Matrix<T> {
    data: Vec<Vec<T>>,
    rows: usize,
    cols: usize,
}

impl<T> Matrix<T> 
where 
    T: Copy + Default + Add<Output = T> + Mul<Output = T> 
{
    /// 新しい行列を作成
    fn new(data: Vec<Vec<T>>) -> Self {
        let rows = data.len();
        let cols = data[0].len();
        Matrix { data, rows, cols }
    }

    /// 行列の掛け算 (C = A * B)
    fn multiply(&self, other: &Matrix<T>) -> Option<Matrix<T>> {
        if self.cols != other.rows {
            return None; // サイズが合わない場合は計算不可
        }

        let mut result_data = vec![vec![T::default(); other.cols]; self.rows];

        for i in 0..self.rows {
            for j in 0..other.cols {
                for k in 0..self.cols {
                    result_data[i][j] = result_data[i][j] + (self.data[i][k] * other.data[k][j]);
                }
            }
        }

        Some(Matrix::new(result_data))
    }
}

fn main() {
    let a = Matrix::new(vec![
        vec![1, 2],
        vec![3, 4],
    ]);
    let b = Matrix::new(vec![
        vec![5, 6],
        vec![7, 8],
    ]);

    if let Some(c) = a.multiply(&b) {
        println!("Result: {:?}", c.data); // [[19, 22], [43, 50]]
    }
}