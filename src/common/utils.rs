use std::time::Duration;

#[inline]
#[must_use]
pub fn safe_duration(d: Duration) -> Option<Duration> {
    if d == Duration::from_secs(0) {
        None
    } else {
        Some(d)
    }
}
