//! New Relic integration for tracing
//!
//! # Overview
//!
//! This crate provides a layer for collecting trace data from [`tracing`] and sending them to [New Relic].
//!
//! `tracing::Span` will be tried as Trace Span, and `tracing::Event` as Logs.
//!
//! `tracing::Attribute` and `tracing::Metadata` wil be tried as Custom Attributes.
//!
//! [`tracing`]: https://github.com/tokio-rs/tracing
//! [New Relic]: https://newrelic.com
//!
//! # Examples
//!
//! ```rust
//! use std::thread::sleep;
//! use std::time::Duration;
//!
//! use tracing_subscriber::layer::SubscriberExt;
//!
//! #[tracing::instrument]
//! fn foo(_: u32) {
//!     sleep(Duration::from_millis(123));
//! }
//!
//! #[tracing::instrument]
//! fn bar(a: u32) {
//!     sleep(Duration::from_millis(456));
//!     foo(a);
//!     sleep(Duration::from_millis(789));
//! }
//!
//! #[tracing::instrument]
//! fn foobar() {
//!     foo(1);
//!     bar(2);
//! }
//!
//! fn main() {
//!     let layer = tracing_newrelic::layer("YOUR-API-KEY")
//!         .with_service_name(String::from("tracing-newr-demo"));
//!
//!     let subscriber = tracing_subscriber::Registry::default().with(layer);
//!
//!     tracing::subscriber::with_default(subscriber, foobar);
//! }
//! ```
//!
//! 1. Replace `YOUR-API-KEY` above with your api key and run it.
//!
//! 2. Open [New Relic One], navigate to `Entity explorer` and search for `tracing-newr-demo`.
//!
//! 3. You should see a entry span named `foobar` and click it for more details:
//!
//! <img src="https://raw.githubusercontent.com/PoiScript/tracing-newrelic/a/screenshot.png" alt="newrelic screenshot" />
//!
//! [New Relic One]: http://one.newrelic.com
//!
//! # License
//!
//! MIT

#![warn(missing_docs)]

mod api;
mod layer;
mod types;
mod utils;

pub use api::{Api, ApiEndpoint};
pub use layer::NewRelicLayer;

use std::thread;
use tokio::runtime;
use tokio::sync::mpsc::unbounded_channel;
use types::{NewrLogs, NewrSpans};

/// Create a new NewRelic layer and spawn a thread for sending data
pub fn layer(api: impl Into<Api>) -> NewRelicLayer {
    let mut api = api.into();

    let (tx, mut rx) = unbounded_channel::<(NewrLogs, NewrSpans)>();

    let handle = thread::Builder::new()
        .name("newrelic-report".into())
        .spawn(move || {
            let rt = match runtime::Builder::new_current_thread().enable_all().build() {
                Err(e) => {
                    eprintln!("Failed to communicate runtime creation failure: {:?}", e);
                    return;
                }
                Ok(v) => v,
            };

            rt.block_on(async move {
                while let Some((logs, spans)) = rx.recv().await {
                    api.push(logs, spans).await
                }

                api.flush().await;
            });

            drop(rt);
        })
        .expect("failed to spawn thread");

    NewRelicLayer {
        service_name: None,
        hostname: None,
        handle: Some(handle),
        channel: Some(tx),
    }
}
