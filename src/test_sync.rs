use std::sync::{Mutex, MutexGuard, OnceLock};

static GLOBAL_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// Acquire a global test lock.
pub fn test_lock() -> MutexGuard<'static, ()> {
    GLOBAL_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}
