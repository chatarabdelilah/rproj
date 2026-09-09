use std::sync::atomic::{AtomicBool, Ordering};

static CANCELLED: AtomicBool = AtomicBool::new(false);

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
    if requested() {
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
