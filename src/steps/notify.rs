use notify_rust::Notification;

/// Fires a desktop toast notification. Failures are logged, not propagated -
/// a missing/broken notification backend shouldn't fail the whole command.
pub fn summary(title: &str, body: &str) {
    if let Err(err) = Notification::new().summary(title).body(body).show() {
        eprintln!("(notification failed: {err})");
    }
}
