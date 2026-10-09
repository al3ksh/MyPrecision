//! CPU load from two `GetSystemTimes` samples.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CpuTimes {
    pub idle: u64,
    /// Includes idle time, as reported by `GetSystemTimes`.
    pub kernel: u64,
    pub user: u64,
}

/// Busy percentage (0..=100) between two samples; `None` when no time elapsed.
pub fn load_between(prev: CpuTimes, now: CpuTimes) -> Option<f32> {
    let idle = now.idle.checked_sub(prev.idle)?;
    let total = now.kernel.checked_sub(prev.kernel)? + now.user.checked_sub(prev.user)?;
    if total == 0 {
        return None;
    }
    Some(((total.saturating_sub(idle)) as f32 / total as f32 * 100.0).clamp(0.0, 100.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_load_basic() {
        let now = CpuTimes { idle: 750, kernel: 900, user: 100 };
        assert_eq!(load_between(CpuTimes::default(), now), Some(25.0));
    }

    #[test]
    fn cpu_load_zero_delta_none() {
        let s = CpuTimes { idle: 5, kernel: 10, user: 3 };
        assert_eq!(load_between(s, s), None);
    }

    #[test]
    fn cpu_load_counter_going_backwards_none() {
        let prev = CpuTimes { idle: 750, kernel: 900, user: 100 };
        assert_eq!(load_between(prev, CpuTimes::default()), None);
    }
}
