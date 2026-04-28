pub fn compute_backoff_delay_ms(
    failure_count: u32,
    base_delay_ms: Option<u64>,
    mut random: impl FnMut() -> f64,
) -> u64 {
    let base = base_delay_ms.unwrap_or(5_000);
    let max = 5 * 60_000;
    let exponent = failure_count.min(6);
    let exponential = base.saturating_mul(2u64.saturating_pow(exponent));
    let capped = exponential.min(max);
    let jitter = (random().clamp(0.0, 1.0) * base as f64).round() as u64;
    capped.saturating_add(jitter).min(max)
}
