use pretty_assertions::assert_eq;
use std::cell::RefCell;
use tracing_newrelic::{Reporter, TraceSpan};

thread_local! {
    static FOUND: RefCell<Vec<TraceSpan>> = RefCell::new(Vec::new());
}

pub struct AssertReporter {
    expected: Vec<TraceSpan>,
}

impl AssertReporter {
    pub fn new(expected: Vec<TraceSpan>) -> Self {
        AssertReporter { expected }
    }
}

impl Reporter for AssertReporter {
    fn report(&self, span: TraceSpan) {
        FOUND.with(|found| found.borrow_mut().push(span));
    }
}

impl Drop for AssertReporter {
    fn drop(&mut self) {
        FOUND.with(|found| assert_eq!(self.expected, *found.borrow()))
    }
}

#[macro_export]
macro_rules! attrs {
    ( $( $key:expr => $value:expr, )+ ) => {{
        let mut map = ::std::collections::HashMap::<String, tracing_newrelic::Value>::new();
        $(
            map.insert($key.into(), $value.into());
        )+
        map
    }}
}
