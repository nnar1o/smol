//! Signal handling for graceful shutdown.
//!
//! Registers handlers for `SIGINT` and `SIGTERM` that set a global atomic flag.
//! The main loop can poll this flag to detect cancellation requests.
//!
//! # Signal safety
//!
//! The raw signal handler only performs an atomic store, which is guaranteed
//! signal-safe on all POSIX platforms. No heap allocations, locks, or I/O occur
//! inside the handler.

use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag set to `true` when `SIGINT` or `SIGTERM` is received.
static CANCELLED: AtomicBool = AtomicBool::new(false);

/// Initialize signal handlers.
///
/// Registers `SIGINT` and `SIGTERM` handlers that set a cancellation flag.
/// Should be called once early during startup.
///
/// ## Panics
///
/// On Unix, if `libc::signal` fails (e.g., invalid signal number), this function
/// panics. In practice this never happens with the standard signals used here.
pub fn init() {
    #[cfg(unix)]
    // SAFETY: libc::signal is safe to call with standard signal numbers.
    // The handler only performs an atomic store, which is signal-safe.
    unsafe {
        if libc::signal(libc::SIGINT, handler as *const () as libc::sighandler_t) == libc::SIG_ERR {
            panic!("Failed to register SIGINT handler");
        }
        if libc::signal(libc::SIGTERM, handler as *const () as libc::sighandler_t) == libc::SIG_ERR {
            panic!("Failed to register SIGTERM handler");
        }
    }
}

/// Signal handler function.
///
/// Sets the global cancellation flag to `true`.
/// Only performs an atomic store (signal-safe).
#[cfg(unix)]
extern "C" fn handler(_sig: i32) {
    CANCELLED.store(true, Ordering::SeqCst);
}

/// Returns `true` if a `SIGINT` or `SIGTERM` has been received since the last
/// call to [`reset`] (or since process start if never reset).
pub fn is_cancelled() -> bool {
    CANCELLED.load(Ordering::SeqCst)
}

/// Reset the cancellation flag (useful for testing or re-initialization).
pub fn reset() {
    CANCELLED.store(false, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        reset();
        assert!(!is_cancelled());
    }

    #[test]
    fn test_set_and_reset() {
        reset();
        assert!(!is_cancelled());
        CANCELLED.store(true, Ordering::SeqCst);
        assert!(is_cancelled());
        reset();
        assert!(!is_cancelled());
    }
}
