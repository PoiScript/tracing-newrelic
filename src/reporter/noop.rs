use crate::reporter::Reporter;
/// A [`Reporter`] that simply logs trace data to stdout
///
/// [`Reporter`]: ../Reporter
pub struct NoopReport;

impl Reporter for NoopReport {
    fn report(&self, batch: newrelic::SpanBatch) {
        println!("{:#?}", batch);
    }
}
