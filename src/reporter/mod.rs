mod blocking;
mod noop;

pub trait Reporter {
    fn report(&self, batch: newrelic::SpanBatch);
}

pub use blocking::BlockingReporter;
pub use noop::NoopReport;
