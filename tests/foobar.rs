mod utils;

use std::time::UNIX_EPOCH;

use tracing_newrelic::{NewRelicLayer, TraceEvent, TraceSpan};
use tracing_subscriber::layer::SubscriberExt;

use utils::AssertReporter;

#[tracing::instrument]
fn foo() {}

#[tracing::instrument]
fn bar() {}

#[tracing::instrument(fields(service.name = "integration-testing"))]
fn foobar() {
    foo();
    bar();
}

#[test]
fn test() {
    let subscriber = tracing_subscriber::Registry::default().with(layer());

    tracing::subscriber::with_default(subscriber, foobar);
}

fn layer() -> NewRelicLayer<AssertReporter> {
    let expected = vec![TraceSpan {
        attrs: attrs! {
            "service.name" => "integration-testing",
        },
        events: vec![
            TraceEvent {
                id: "event1".to_string(),
                created: UNIX_EPOCH,
                attrs: attrs! {
                    "name" => "foobar",
                },
            },
            TraceEvent {
                id: "event2".to_string(),
                created: UNIX_EPOCH,
                attrs: attrs! {
                    "parent.id" => "event1",
                    "name" => "foo",
                },
            },
            TraceEvent {
                id: "event3".to_string(),
                created: UNIX_EPOCH,
                attrs: attrs! {
                    "parent.id" => "event1",
                    "name" => "bar",
                },
            },
        ],
    }];

    NewRelicLayer::new(AssertReporter::new(expected)).with_duration(false)
}
