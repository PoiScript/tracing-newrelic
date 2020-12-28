use crate::reporter::Reporter;
use crate::span::TraceSpan;
/// A [`Reporter`] that simply logs trace data to stdout
///
/// [`Reporter`]: ../Reporter
pub struct NoopReport;

impl Reporter for NoopReport {
    fn report(&self, span: TraceSpan) {
        println!("{:#?}", span);
    }
}
