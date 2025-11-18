use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Trait for checking if computation should be cancelled
pub trait CancellationChecker: Clone {
    /// Returns true if computation should be cancelled
    fn is_cancelled(&self) -> bool;
}

/// Never cancels - for single-threaded or non-cancellable contexts
#[derive(Clone, Copy, Default)]
pub struct NeverCancel;

impl CancellationChecker for NeverCancel {
    fn is_cancelled(&self) -> bool {
        false
    }
}

/// Checks an atomic boolean flag for cancellation
#[derive(Clone)]
pub struct AtomicBoolChecker {
    flag: Arc<AtomicBool>,
}

impl AtomicBoolChecker {
    pub fn new(flag: Arc<AtomicBool>) -> Self {
        Self { flag }
    }
}

impl CancellationChecker for AtomicBoolChecker {
    fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_never_cancel_always_returns_false() {
        let checker = NeverCancel;
        assert!(!checker.is_cancelled());
        assert!(!checker.is_cancelled());
    }

    #[test]
    fn test_atomic_bool_checker_reads_flag() {
        let flag = Arc::new(AtomicBool::new(false));
        let checker = AtomicBoolChecker::new(Arc::clone(&flag));

        assert!(!checker.is_cancelled());

        flag.store(true, Ordering::Relaxed);
        assert!(checker.is_cancelled());

        flag.store(false, Ordering::Relaxed);
        assert!(!checker.is_cancelled());
    }

    #[test]
    fn test_atomic_bool_checker_cloneable() {
        let flag = Arc::new(AtomicBool::new(false));
        let checker1 = AtomicBoolChecker::new(Arc::clone(&flag));
        let checker2 = checker1.clone();

        flag.store(true, Ordering::Relaxed);
        assert!(checker1.is_cancelled());
        assert!(checker2.is_cancelled());
    }
}
