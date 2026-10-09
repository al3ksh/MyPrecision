//! Lock helpers.

use std::sync::{Mutex, MutexGuard};

pub trait LockExt<T> {
    /// Locks, ignoring poisoning: a panic elsewhere must not cascade into the poller,
    /// tray and commands. Every guarded value here stays valid between statements.
    fn lock_ok(&self) -> MutexGuard<'_, T>;
}

impl<T> LockExt<T> for Mutex<T> {
    fn lock_ok(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poisoned_lock_still_yields_its_value() {
        let m = Mutex::new(7);
        let _ = std::panic::catch_unwind(|| {
            let _g = m.lock().unwrap();
            panic!("poison");
        });
        assert!(m.is_poisoned());
        assert_eq!(*m.lock_ok(), 7);
    }
}
