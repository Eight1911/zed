use futures::{FutureExt, StreamExt, channel::mpsc};
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn telemetry_drops_events_without_evaluating_properties() {
    let (sender, mut receiver) = mpsc::unbounded();
    telemetry::init(sender);
    let evaluated = AtomicBool::new(false);

    telemetry::event!({
        evaluated.store(true, Ordering::SeqCst);
        "test"
    });
    telemetry::event!("test", value = evaluated.swap(true, Ordering::SeqCst));
    telemetry::send_event(telemetry::Event {
        event_type: "direct".into(),
        event_properties: Default::default(),
    });

    assert!(!evaluated.load(Ordering::SeqCst));
    assert_eq!(receiver.next().now_or_never(), Some(None));
}
