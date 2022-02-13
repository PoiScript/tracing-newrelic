use crate::types::{NrLog, NrSpan};

/// Reports trace data
///
/// Currently, there are only two reporters:
///
/// 1. [`NoopReport`]: simply logs trace data to stdout.
///
/// 2. [`BlockingReporter`]: a reporter using `newrelic_telemetry::blocking::Client`
///
/// [`NoopReport`]: NoopReport
/// [`BlockingReporter`]: BlockingReporter
pub trait Reporter {
    fn report(&self, spans: Vec<NrSpan>, logs: Vec<NrLog>);
}

mod blocking;

pub use blocking::BlockingReporter;
