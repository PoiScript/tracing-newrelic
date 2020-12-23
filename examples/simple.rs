use std::thread::sleep;
use std::time::Duration;

use tracing_subscriber;
use tracing_subscriber::layer::SubscriberExt;

use tracing_newrelic::{NewRelicLayer, NoopReport};

#[tracing::instrument(name = "fibonacci()")]
fn fibonacci(n: u32) -> u32 {
    sleep(Duration::from_millis(100 * n as u64));

    match n {
        0 | 1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn main() {
    env_logger::init();

    let layer = NewRelicLayer::new(NoopReport)
        .with_name(true)
        .with_level(true)
        .with_target(true)
        .with_module_path(true)
        .with_file(true)
        .with_line(true)
        .with_duration(true);

    let subscriber = tracing_subscriber::Registry::default().with(layer);

    tracing::subscriber::with_default(subscriber, || {
        let span = tracing::info_span!("calculating fibonacci(3)", service.name = "fibonacci");

        let _enter = span.enter();

        fibonacci(3);
    });
}
