//! U-Statistics computation module
//! Supports degree-1 and degree-2 kernels with asymptotic inference.

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

fn kernel_h(x: f64, y: f64, kernel: KernelType) -> f64 {
    match kernel {
        KernelType::Variance => (x - y).powi(2) / 2.0,
        KernelType::GiniMeanDifference => (x - y).abs(),
        KernelType::MannWhitney => if x <= y { 1.0 } else { 0.0 },
        _ => 0.0,
    }
}

pub fn compute(values: &[f64], kernel: KernelType) -> Option<f64> {
    let n = values.len();
    match kernel {
        KernelType::Mean => {
            if n == 0 { return None; }
            Some(values.iter().sum::<f64>() / n as f64)
        }
        KernelType::Variance | KernelType::GiniMeanDifference | KernelType::MannWhitney => {
            if n < 2 { return None; }
            let mut sum = 0.0;
            let mut count = 0usize;
            for i in 0..n {
                for j in (i + 1)..n {
                    sum += kernel_h(values[i], values[j], kernel);
                    count += 1;
                }
            }
            Some(sum / count as f64)
        }
    }
}

/// Compute h_1(X_i) for each i (conditional expectation minus theta)
pub fn compute_h1_values(values: &[f64], kernel: KernelType) -> Option<Vec<f64>> {
    let n = values.len();
    if n < 3 { return None; }
    let theta = compute(values, kernel)?;
    let mut h1 = Vec::with_capacity(n);
    for i in 0..n {
        let mut sum = 0.0;
        let mut count = 0usize;
        for j in 0..n {
            if i == j { continue; }
            sum += kernel_h(values[i], values[j], kernel);
            count += 1;
        }
        let cond_mean = sum / count as f64;
        h1.push(cond_mean - theta);
    }
    Some(h1)
}

/// Compute zeta_1 = Var(h_1(X_1))
pub fn compute_zeta1(values: &[f64], kernel: KernelType) -> Option<f64> {
    let h1 = compute_h1_values(values, kernel)?;
    let mean = h1.iter().sum::<f64>() / h1.len() as f64;
    let var = h1.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / h1.len() as f64;
    Some(var)
}

/// Compute all pairwise kernel values (for Hoeffding decomposition and plotting)
pub fn pairwise_values(values: &[f64], kernel: KernelType) -> Vec<f64> {
    let n = values.len();
    let mut out = Vec::with_capacity(n * (n - 1) / 2);
    for i in 0..n {
        for j in (i + 1)..n {
            out.push(kernel_h(values[i], values[j], kernel));
        }
    }
    out
}

/// Compute U-statistic on a bootstrap sample (resampled with replacement indices)
pub fn compute_bootstrap(values: &[f64], indices: &[usize], kernel: KernelType) -> Option<f64> {
    let boot: Vec<f64> = indices.iter().map(|&i| values[i]).collect();
    compute(&boot, kernel)
}

/// Format comprehensive result
pub fn format_result(values: &[f64], kernel: KernelType) -> String {
    let n = values.len();
    let m = kernel.degree();
    if n < m {
        return format!("Need at least {} value(s) for {}", m, kernel.name());
    }
    let theta = compute(values, kernel).unwrap_or(0.0);
    let mut s = format!("=== U-Statistic Result ===\n");
    s += &format!("Kernel: {}\n", kernel.name());
    s += &format!("Degree (m): {}\n", m);
    s += &format!("Sample size (n): {}\n", n);
    s += &format!("Combinations C(n,m): {}\n", combinations(n, m));
    s += &format!("U_n = {:.8}\n", theta);

    if m == 2 && n >= 3 {
        if let Some(zeta1) = compute_zeta1(values, kernel) {
            let asym_var = 4.0 * zeta1 / n as f64;
            let se = asym_var.sqrt();
            s += &format!("\n--- Asymptotic Properties (Nondegenerate) ---\n");
            s += &format!("zeta_1 = {:.8}\n", zeta1);
            s += &format!("Asymptotic variance (4*zeta_1/n) = {:.8}\n", asym_var);
            s += &format!("Standard error = {:.8}\n", se);
            let ci_lower = theta - 1.96 * se;
            let ci_upper = theta + 1.96 * se;
            s += &format!("95% Asymptotic CI: [{:.8}, {:.8}]\n", ci_lower, ci_upper);
            s += &format!("\nAsymptotic normality (Hoeffding 1948):\n");
            s += &format!("  sqrt(n) * (U_n - theta) / (2*sqrt(zeta_1)) ~ N(0,1)\n");
        }
    }
    s
}

fn combinations(n: usize, m: usize) -> usize {
    if m > n { return 0; }
    if m == 0 || m == n { return 1; }
    let mut r = 1usize;
    for i in 0..m {
        r = r * (n - i) / (i + 1);
    }
    r
}
