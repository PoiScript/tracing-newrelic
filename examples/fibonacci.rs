use std::env::var;
use std::thread::sleep;
use std::time::Duration;

use tracing_subscriber::{layer::SubscriberExt, Registry};

#[tracing::instrument(name = "fibonacci()")]
fn fibonacci(n: u32) -> u32 {
    let ms = 100 * n as u64;

    tracing::info!(n = n, "sleep {}ms", ms);

    sleep(Duration::from_millis(ms));

    match n {
        0 | 1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn main() {
    env_logger::init();

    let newrelic = tracing_newrelic::layer(var("API_KEY").expect("API_KEY not found"))
        .with_service_name(String::from("fibonacci"));

    let fmt = tracing_subscriber::fmt::layer();

    let subscriber = Registry::default().with(newrelic).with(fmt);

    tracing::subscriber::with_default(subscriber, || {
        let span = tracing::info_span!("calculating fibonacci(3)");

        let _enter = span.enter();

        fibonacci(3);
    });
}
