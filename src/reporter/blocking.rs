use newrelic::blocking::Client;
use newrelic::ClientBuilder;

use crate::reporter::Reporter;
use crate::span::TraceSpan;

/// A [`Reporter`] using `newrelic_telemetry::blocking::Client`
///
/// [`Reporter`]: ../Reporter
pub struct BlockingReporter {
    client: Client,
}

impl BlockingReporter {
    pub fn new(api_key: &str) -> BlockingReporter {
        Self::with_builder(ClientBuilder::new(api_key))
    }

    pub fn with_builder(builder: ClientBuilder) -> BlockingReporter {
        BlockingReporter {
            client: builder.build_blocking().unwrap(),
        }
    }
}

impl Reporter for BlockingReporter {
    fn report(&self, span: TraceSpan) {
        self.client.send_spans(span.into_batch());
    }
}

impl Drop for BlockingReporter {
    fn drop(&mut self) {
        // FIXME: client.shutdown() takes ownership
        std::mem::replace(
            &mut self.client,
            ClientBuilder::new("").build_blocking().unwrap(),
        )
        .shutdown();
    }
}
