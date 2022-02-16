# tracing-newrelic

New Relic integration for tracing

## Overview

This crate provides a layer for collecting trace data from [`tracing`] and sending them to [New Relic].

`tracing::Span` will be tried as Trace Span, and `tracing::Event` as Logs.

`tracing::Attribute` and `tracing::Metadata` wil be tried as Custom Attributes.

[`tracing`]: https://github.com/tokio-rs/tracing
[New Relic]: https://newrelic.com

## Examples

```rust
use std::thread::sleep;
use std::time::Duration;

use tracing_subscriber::layer::SubscriberExt;

#[tracing::instrument]
fn foo(_: u32) {
    sleep(Duration::from_millis(123));
}

#[tracing::instrument]
fn bar(a: u32) {
    sleep(Duration::from_millis(456));
    foo(a);
    sleep(Duration::from_millis(789));
}

#[tracing::instrument]
fn foobar() {
    foo(1);
    bar(2);
}

fn main() {
    let layer = tracing_newrelic::layer("YOUR-API-KEY");

    let subscriber = tracing_subscriber::Registry::default().with(layer);

    tracing::subscriber::with_default(subscriber, foobar);
}
```

1. Replace `YOUR-API-KEY` above with your api key and run it.

2. Open [New Relic One], navigate to `Entity explorer` and search for `tracing-newr-demo`.

3. You should see a entry span named `foobar` and click it for more details:

<img src="https://raw.githubusercontent.com/PoiScript/tracing-newrelic/a/screenshot.png" alt="newrelic screenshot" />

[New Relic One]: http://one.newrelic.com

And I strongly recommend include these attributes in your spans:

1. `span.kind`

    New Relic creates throught and response time dashboards for spans with `span.kind` set to `server` and `consumer`.

2. `otel.status_code` & `otel.status_description`

    New Relic creates error rate dashboard for spans with `otel.status_code` set to `ERROR`.

3. `service.name`

    New Relic group entity by their `service.name` field.

4. `name`

    New Relic group trnsations by their `name` field.

## License

MIT
