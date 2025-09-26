use std::sync::{Mutex, MutexGuard, OnceLock};

static GLOBAL_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// Acquire a global test lock for tests that mutate process-wide state
/// such as the current working directory. This allows re-enabling end-to-end
/// tests across modules without introducing external dependencies.
pub fn test_lock() -> MutexGuard<'static, ()> {
    GLOBAL_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}
