//! Simple plotting utilities using egui primitives

pub fn draw_histogram(ui: &mut egui::Ui, data: &[f64], bins: usize, color: egui::Color32) {
    if data.is_empty() { return; }
    let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if min == max { return; }

    let bin_width = (max - min) / bins as f64;
    let mut counts = vec![0usize; bins];
    for &v in data {
        let b = (((v - min) / bin_width).floor() as usize).min(bins - 1);
        counts[b] += 1;
    }
    let max_count = counts.iter().cloned().max().unwrap_or(1).max(1);

    let available_width = ui.available_width();
    let height = 120.0;
    let bar_width = available_width / bins as f32;

    let (rect, _) = ui.allocate_exact_size(egui::vec2(available_width, height), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    for (i, &count) in counts.iter().enumerate() {
        let h = (count as f32 / max_count as f32) * height;
        let x = rect.min.x + i as f32 * bar_width;
        let y = rect.max.y - h;
        painter.rect_filled(
            egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(bar_width - 1.0, h)),
            0.0,
            color,
        );
    }
}

pub fn draw_bar_chart(ui: &mut egui::Ui, labels: &[String], values: &[f64], color: egui::Color32) {
    if values.is_empty() { return; }
    let max_v = values.iter().cloned().fold(0.0f64, f64::max).max(1e-10);
    let available_width = ui.available_width();
    let height = 100.0;
    let bar_width = available_width / values.len().max(1) as f32;

    let (rect, _) = ui.allocate_exact_size(egui::vec2(available_width, height), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    for (i, &v) in values.iter().enumerate() {
        let h = (v / max_v) as f32 * height;
        let x = rect.min.x + i as f32 * bar_width;
        let y = rect.max.y - h;
        painter.rect_filled(
            egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(bar_width - 2.0, h)),
            0.0,
            color,
        );
    }
}

pub fn draw_qq_plot(ui: &mut egui::Ui, data: &[f64], color: egui::Color32) {
    if data.len() < 2 { return; }
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = sorted.len() as f64;

    let available_width = ui.available_width();
    let height = 120.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(available_width, height), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    let min_x = -3.0f64;
    let max_x = 3.0f64;
    let min_y = sorted.first().cloned().unwrap_or(0.0);
    let max_y = sorted.last().cloned().unwrap_or(1.0);
    let range_y = max_y - min_y;

    for (i, &y) in sorted.iter().enumerate() {
        let p = (i as f64 + 0.5) / n;
        let x = approx_inv_cdf_normal(p);
        let px = rect.min.x + ((x - min_x) / (max_x - min_x)) as f32 * available_width;
        let py = rect.max.y - ((y - min_y) / range_y.max(1e-10)) as f32 * height;
        painter.circle_filled(egui::pos2(px, py), 2.0, color);
    }
}

/// Approximate inverse normal CDF (Peter Acklam's approximation)
fn approx_inv_cdf_normal(p: f64) -> f64 {
    if p <= 0.0 { return -3.0; }
    if p >= 1.0 { return 3.0; }
    let a1 = -3.969683028665376e+01;
    let a2 = 2.209460984245205e+02;
    let a3 = -2.759285104469687e+02;
    let a4 = 1.383577518672690e+02;
    let a5 = -3.066479806614716e+01;
    let a6 = 2.506628277459239e+00;
    let b1 = -5.447609879822406e+01;
    let b2 = 1.615858368395409e+02;
    let b3 = -1.556989798598866e+02;
    let b4 = 6.680131301652241e+01;
    let b5 = -1.328068155288572e+01;
    let c1 = -7.784894002430592e-03;
    let c2 = -3.223964580411365e-01;
    let c3 = -2.400758277161838e+00;
    let c4 = -2.549732539343734e+00;
    let c5 = 4.374664141464968e+00;
    let c6 = 2.938163982698783e+00;
    let d1 = 7.784695709041462e-03;
    let d2 = 3.224671290700398e-01;
    let d3 = 2.445134137142996e+00;
    let d4 = 3.754408661907416e+00;
    let p_low = 0.02425;
    let p_high = 1.0 - p_low;

    let q = if p < p_low {
        (-2.0 * p.ln()).sqrt()
    } else if p <= p_high {
        p - 0.5
    } else {
        (-2.0 * (1.0 - p).ln()).sqrt()
    };

    if p < p_low {
        let x = (((((c1 * q + c2) * q + c3) * q + c4) * q + c5) * q + c6) /
                ((((d1 * q + d2) * q + d3) * q + d4) * q + 1.0);
        x
    } else if p <= p_high {
        let r = q * q;
        let x = (((((a1 * r + a2) * r + a3) * r + a4) * r + a5) * r + a6) * q /
                (((((b1 * r + b2) * r + b3) * r + b4) * r + b5) * r + 1.0);
        x
    } else {
        let x = (((((c1 * q + c2) * q + c3) * q + c4) * q + c5) * q + c6) /
                ((((d1 * q + d2) * q + d3) * q + d4) * q + 1.0);
        -x
    }
}
