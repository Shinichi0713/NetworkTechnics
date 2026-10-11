//! U-Statistics computation module
//!
//! Supports:
//! - Degree-1: Sample mean
//! - Degree-2: Sample variance (kernel: h(x,y) = (x-y)^2 / 2)
//! - Degree-2: Gini mean difference (kernel: h(x,y) = |x - y|)
//! - Degree-2: Mann-Whitney type (kernel: h(x,y) = 1_{x <= y})

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KernelType {
    Mean,
    Variance,
    GiniMeanDifference,
    MannWhitney,
}

impl KernelType {
    pub fn degree(&self) -> usize {
        match self {
            KernelType::Mean => 1,
            _ => 2,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            KernelType::Mean => "Sample Mean (degree-1)",
            KernelType::Variance => "Sample Variance (degree-2)",
            KernelType::GiniMeanDifference => "Gini Mean Difference (degree-2)",
            KernelType::MannWhitney => "Mann-Whitney Type (degree-2)",
        }
    }
}

/// Compute U-statistic for the given kernel type
pub fn compute(values: &[f64], kernel: KernelType) -> Option<f64> {
    let n = values.len();
    match kernel {
        KernelType::Mean => {
            if n == 0 {
                return None;
            }
            Some(values.iter().sum::<f64>() / n as f64)
        }
        KernelType::Variance => {
            if n < 2 {
                return None;
            }
            let mut sum = 0.0;
            let mut count = 0usize;
            for i in 0..n {
                for j in (i + 1)..n {
                    let h = (values[i] - values[j]).powi(2) / 2.0;
                    sum += h;
                    count += 1;
                }
            }
            Some(sum / count as f64)
        }
        KernelType::GiniMeanDifference => {
            if n < 2 {
                return None;
            }
            let mut sum = 0.0;
            let mut count = 0usize;
            for i in 0..n {
                for j in (i + 1)..n {
                    sum += (values[i] - values[j]).abs();
                    count += 1;
                }
            }
            Some(sum / count as f64)
        }
        KernelType::MannWhitney => {
            if n < 2 {
                return None;
            }
            let mut sum = 0.0;
            let mut count = 0usize;
            for i in 0..n {
                for j in (i + 1)..n {
                    sum += if values[i] <= values[j] { 1.0 } else { 0.0 };
                    count += 1;
                }
            }
            Some(sum / count as f64)
        }
    }
}

/// Compute the asymptotic variance estimate (zeta_1) for degree-2 kernels
/// zeta_1 = Var(h_1(X_1)) where h_1(x) = E[h(x, X_2)] - theta
pub fn compute_zeta1(values: &[f64], kernel: KernelType) -> Option<f64> {
    if values.len() < 3 {
        return None;
    }
    let theta = compute(values, kernel)?;

    let h1_values: Vec<f64> = values
        .iter()
        .map(|&xi| {
            let conditional_mean = values
                .iter()
                .filter(|&&xj| xj != xi)
                .map(|&xj| kernel_h(xi, xj, kernel))
                .sum::<f64>()
                / (values.len() - 1) as f64;
            conditional_mean - theta
        })
        .collect();

    let mean_h1 = h1_values.iter().sum::<f64>() / h1_values.len() as f64;
    let variance = h1_values.iter().map(|&v| (v - mean_h1).powi(2)).sum::<f64>() / h1_values.len() as f64;
    Some(variance)
}

fn kernel_h(x: f64, y: f64, kernel: KernelType) -> f64 {
    match kernel {
        KernelType::Variance => (x - y).powi(2) / 2.0,
        KernelType::GiniMeanDifference => (x - y).abs(),
        KernelType::MannWhitney => if x <= y { 1.0 } else { 0.0 },
        _ => 0.0,
    }
}

/// Format result with explanation
pub fn format_result(values: &[f64], kernel: KernelType) -> String {
    let n = values.len();
    let m = kernel.degree();
    if n < m {
        return format!("Need at least {} value(s) for {}", m, kernel.name());
    }

    let theta = compute(values, kernel).unwrap_or(0.0);
    let mut result = format!("=== U-Statistic Result ===\n");
    result += &format!("Kernel: {}\n", kernel.name());
    result += &format!("Degree (m): {}\n", m);
    result += &format!("Sample size (n): {}\n", n);
    result += &format!("Number of combinations C(n,m): {}\n", combinations(n, m));
    result += &format!("U_n = {:.6}\n", theta);

    if m == 2 && n >= 3 {
        if let Some(zeta1) = compute_zeta1(values, kernel) {
            let asymptotic_var = 4.0 * zeta1 / n as f64;
            result += &format!("\n--- Asymptotic Properties ---\n");
            result += &format!("zeta_1 = {:.6}\n", zeta1);
            result += &format!("Asymptotic variance (4*zeta_1/n) = {:.6}\n", asymptotic_var);
            result += &format!("Standard error = {:.6}\n", asymptotic_var.sqrt());
            let ci_lower = theta - 1.96 * asymptotic_var.sqrt();
            let ci_upper = theta + 1.96 * asymptotic_var.sqrt();
            result += &format!("95% CI: [{:.6}, {:.6}]\n", ci_lower, ci_upper);
        }
    }

    result
}

fn combinations(n: usize, m: usize) -> usize {
    if m > n {
        return 0;
    }
    if m == 0 || m == n {
        return 1;
    }
    let mut result = 1usize;
    for i in 0..m {
        result = result * (n - i) / (i + 1);
    }
    result
}
