use qmeter_core::scheduler::compute_backoff_delay_ms;

#[test]
fn backoff_increases_with_failures_and_respects_max() {
    let d0 = compute_backoff_delay_ms(0, None, || 0.5);
    let d1 = compute_backoff_delay_ms(1, None, || 0.5);
    let d4 = compute_backoff_delay_ms(4, None, || 0.5);

    assert!(d1 >= d0);
    assert!(d4 >= d1);
    assert!(d4 <= 5 * 60_000);
}
