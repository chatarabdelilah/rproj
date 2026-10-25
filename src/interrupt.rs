use std::sync::atomic::{AtomicBool, Ordering};

static CANCELLED: AtomicBool = AtomicBool::new(false);
thread_local! {
    static FINISH_ACTIVE_ITEM: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Machine Setup observes Stop between items. Existing installer helpers must
/// finish their current item rather than interpreting Ctrl+C halfway through it.
pub(crate) fn finish_active_item<T>(run: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            FINISH_ACTIVE_ITEM.set(self.0);
        }
    }
    let _restore = Restore(FINISH_ACTIVE_ITEM.replace(true));
    run()
}

fn should_cancel(requested: bool) -> bool {
    requested && !FINISH_ACTIVE_ITEM.get()
}

#[derive(Debug)]
pub struct Cancelled;

impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Operation cancelled")
    }
}

impl std::error::Error for Cancelled {}

pub fn install() -> anyhow::Result<()> {
    ctrlc::try_set_handler(|| CANCELLED.store(true, Ordering::SeqCst))?;
    Ok(())
}

pub fn reset() {
    CANCELLED.store(false, Ordering::SeqCst);
}

pub fn request() {
    CANCELLED.store(true, Ordering::SeqCst);
}

pub fn requested() -> bool {
    CANCELLED.load(Ordering::SeqCst)
}

pub fn check() -> anyhow::Result<()> {
    if should_cancel(requested()) {
        Err(Cancelled.into())
    } else {
        Ok(())
    }
}

pub fn is_cancelled(error: &anyhow::Error) -> bool {
    error.is::<Cancelled>()
        || error
            .downcast_ref::<inquire::InquireError>()
            .is_some_and(|error| {
                matches!(
                    error,
                    inquire::InquireError::OperationCanceled
                        | inquire::InquireError::OperationInterrupted
                )
            })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deferral_is_scoped_thread_local_and_restored_after_panic() {
        assert!(should_cancel(true));
        finish_active_item(|| {
            assert!(!should_cancel(true));
            assert!(std::thread::spawn(|| should_cancel(true)).join().unwrap());
            finish_active_item(|| assert!(!should_cancel(true)));
            assert!(!should_cancel(true));
        });
        assert!(should_cancel(true));
        let _ = std::panic::catch_unwind(|| finish_active_item(|| panic!("fixture")));
        assert!(should_cancel(true));
    }
}
