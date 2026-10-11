//! Two-sample U-statistics and asymptotic tests

pub struct TwoSampleResult {
    pub u_stat: f64,
    pub theta: f64,
    pub std_error: f64,
    pub z_score: f64,
    pub p_value_two_sided: f64,
    pub p_value_greater: f64,
    pub p_value_less: f64,
    pub n1: usize,
    pub n2: usize,
}

/// Mann-Whitney type U-statistic: theta = P(X <= Y)
/// U = (1/(n1*n2)) sum_{i,j} 1_{X_i <= Y_j}
pub fn mann_whitney_u(group_a: &[f64], group_b: &[f64]) -> Option<TwoSampleResult> {
    let n1 = group_a.len();
    let n2 = group_b.len();
    if n1 == 0 || n2 == 0 {
        return None;
    }

    let mut sum = 0.0;
    for x in group_a {
        for y in group_b {
            sum += if x <= y { 1.0 } else { 0.0 };
        }
    }
    let u = sum / (n1 as f64 * n2 as f64);

    // Estimate variance under H0: F = G using the null variance formula
    // Var(U) under null = (n1 + n2 + 1) / (12 * n1 * n2)
    // More generally, use the asymptotic variance estimator
    let null_var = (n1 as f64 + n2 as f64 + 1.0) / (12.0 * n1 as f64 * n2 as f64);
    let se = null_var.sqrt();

    let z = if se > 0.0 { (u - 0.5) / se } else { 0.0 };

    // Normal CDF approximation
    let p_two = 2.0 * (1.0 - erf(z.abs() / 2.0f64.sqrt()));
    let p_greater = 1.0 - erf(z / 2.0f64.sqrt()).max(0.0).min(1.0);
    let p_less = erf(z / 2.0f64.sqrt()).max(0.0).min(1.0);

    Some(TwoSampleResult {
        u_stat: sum,
        theta: u,
        std_error: se,
        z_score: z,
        p_value_two_sided: p_two,
        p_value_greater: p_greater,
        p_value_less: p_less,
        n1,
        n2,
    })
}

/// Two-sample comparison of means (degree-1 kernel difference)
pub fn mean_difference(group_a: &[f64], group_b: &[f64]) -> Option<TwoSampleResult> {
    let n1 = group_a.len();
    let n2 = group_b.len();
    if n1 == 0 || n2 == 0 {
        return None;
    }
    let mean_a = group_a.iter().sum::<f64>() / n1 as f64;
    let mean_b = group_b.iter().sum::<f64>() / n2 as f64;
    let diff = mean_a - mean_b;

    let var_a = group_a.iter().map(|&x| (x - mean_a).powi(2)).sum::<f64>() / n1 as f64;
    let var_b = group_b.iter().map(|&x| (x - mean_b).powi(2)).sum::<f64>() / n2 as f64;
    let se = (var_a / n1 as f64 + var_b / n2 as f64).sqrt();
    let z = if se > 0.0 { diff / se } else { 0.0 };

    let p_two = 2.0 * (1.0 - erf(z.abs() / 2.0f64.sqrt()));

    Some(TwoSampleResult {
        u_stat: diff,
        theta: diff,
        std_error: se,
        z_score: z,
        p_value_two_sided: p_two,
        p_value_greater: 0.0,
        p_value_less: 0.0,
        n1,
        n2,
    })
}

/// Error function approximation (Abramowitz & Stegun)
fn erf(x: f64) -> f64 {
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();

    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();

    sign * y
}
