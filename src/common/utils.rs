use std::time::Duration;

#[inline]
pub fn safe_duration(d: Duration) -> Option<Duration> {
    if d == Duration::from_secs(0) {
        None
    } else {
        Some(d)
    }
}
