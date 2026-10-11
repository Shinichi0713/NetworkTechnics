//! Advanced analysis: Hoeffding decomposition, Jackknife, Bootstrap

use crate::u_stats::{compute, compute_h1_values, KernelType};
use rand::distributions::{Distribution, Uniform};

/// Hoeffding decomposition for degree-2 kernels
pub struct HoeffdingDecomp {
    pub h1_values: Vec<f64>,
    pub linear_component: f64,
    pub quadratic_component: f64,
    pub theta: f64,
}

pub fn hoeffding_decompose(values: &[f64], kernel: KernelType) -> Option<HoeffdingDecomp> {
    if values.len() < 3 { return None; }
    let theta = compute(values, kernel)?;
    let h1 = compute_h1_values(values, kernel)?;

    let linear = h1.iter().sum::<f64>();
    // For degree-2, U_n - theta = (2/n) * sum_i h_1(X_i) + residual
    // The linear component contribution to sqrt(n)(U_n - theta) is (2/sqrt(n)) * sum_i h_1(X_i)
    let linear_scaled = 2.0 * linear / values.len() as f64;

    // Quadratic residual (approximate)
    let mut quad_sum = 0.0;
    let n = values.len();
    for i in 0..n {
        for j in (i + 1)..n {
            let h_val = crate::u_stats::pairwise_values(&[values[i], values[j]], kernel)[0];
            let residual = h_val - h1[i] - h1[j] - theta;
            quad_sum += residual;
        }
    }
    let quadratic = quad_sum / crate::u_stats::pairwise_values(values, kernel).len().max(1) as f64;

    Some(HoeffdingDecomp {
        h1_values: h1,
        linear_component: linear_scaled,
        quadratic_component: quadratic,
        theta,
    })
}

pub struct BootstrapResult {
    pub estimates: Vec<f64>,
    pub mean: f64,
    pub std_error: f64,
    pub ci_lower: f64,
    pub ci_upper: f64,
    pub bias: f64,
}

pub fn bootstrap(values: &[f64], kernel: KernelType, b: usize) -> Option<BootstrapResult> {
    let n = values.len();
    if n < kernel.degree() || b == 0 { return None; }
    let theta_hat = compute(values, kernel)?;

    let mut rng = rand::thread_rng();
    let between = Uniform::from(0..n);
    let mut estimates = Vec::with_capacity(b);

    for _ in 0..b {
        let indices: Vec<usize> = (0..n).map(|_| between.sample(&mut rng)).collect();
        if let Some(est) = crate::u_stats::compute_bootstrap(values, &indices, kernel) {
            estimates.push(est);
        }
    }

    if estimates.is_empty() { return None; }

    estimates.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean = estimates.iter().sum::<f64>() / estimates.len() as f64;
    let var = estimates.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / estimates.len() as f64;
    let se = var.sqrt();
    let bias = mean - theta_hat;
    let ci_lower = estimates[(b as f64 * 0.025) as usize.min(b - 1)];
    let ci_upper = estimates[(b as f64 * 0.975) as usize.min(b - 1)];

    Some(BootstrapResult {
        estimates,
        mean,
        std_error: se,
        ci_lower,
        ci_upper,
        bias,
    })
}

/// Jackknife (delete-1) variance estimation for U-statistics
pub fn jackknife(values: &[f64], kernel: KernelType) -> Option<(f64, f64)> {
    let n = values.len();
    if n <= kernel.degree() + 1 { return None; }
    let theta_full = compute(values, kernel)?;

    let mut pseudo_values = Vec::with_capacity(n);
    for i in 0..n {
        let mut reduced = values.to_vec();
        reduced.remove(i);
        let theta_i = compute(&reduced, kernel)?;
        pseudo_values.push(n as f64 * theta_full - (n - 1) as f64 * theta_i);
    }

    let mean_pseudo = pseudo_values.iter().sum::<f64>() / n as f64;
    let var = pseudo_values.iter().map(|&v| (v - mean_pseudo).powi(2)).sum::<f64>() / (n as f64 * (n - 1) as f64);

    Some((mean_pseudo, var.sqrt()))
}
