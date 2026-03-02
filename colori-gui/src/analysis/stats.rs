/// Wilson confidence interval at 95% confidence level.
/// Returns (lower, upper) as percentages, or None if games == 0.
pub fn wilson_confidence_interval(wins: f64, games: f64) -> Option<(f64, f64)> {
    if games == 0.0 {
        return None;
    }
    let z = 1.96_f64;
    let p = wins / games;
    let z2 = z * z;
    let denom = 1.0 + z2 / games;
    let center = (p + z2 / (2.0 * games)) / denom;
    let margin =
        z * ((p * (1.0 - p) / games + z2 / (4.0 * games * games)).sqrt()) / denom;
    Some((
        (center - margin).max(0.0) * 100.0,
        (center + margin).min(1.0) * 100.0,
    ))
}
