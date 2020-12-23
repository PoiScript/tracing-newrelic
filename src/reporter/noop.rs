use crate::reporter::Reporter;

pub struct NoopReport;

impl Reporter for NoopReport {
    fn report(&self, batch: newrelic::SpanBatch) {
        println!("{:#?}", batch);
    }
}
