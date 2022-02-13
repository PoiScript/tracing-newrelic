use std::env::var;
use std::thread::sleep;
use std::time::Duration;

use tracing_subscriber::{layer::SubscriberExt, Registry};

use tracing_newrelic::{Api, NewRelicLayer};

#[tracing::instrument(name = "fibonacci()")]
fn fibonacci(n: u32) -> u32 {
    let ms = 100 * n as u64;

    tracing::info!("sleep {}ms", ms);

    sleep(Duration::from_millis(ms));

    match n {
        0 | 1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn main() {
    env_logger::init();

    let layer = NewRelicLayer::blocking(Api {
        key: var("API_KEY").expect("API_KEY not found"),
        ..Default::default()
    });

    let subscriber = Registry::default().with(layer);

    tracing::subscriber::with_default(subscriber, || {
        let span = tracing::info_span!("calculating fibonacci(3)", service.name = "fibonacci");

        let _enter = span.enter();

        fibonacci(3);
    });
}
