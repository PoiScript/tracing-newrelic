use tracing_core::{
    span::{Attributes, Id, Record},
    Event, Subscriber,
};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

use crate::{
    api::Api,
    reporter::{BlockingReporter, Reporter},
    types::{NrLog, NrSpan},
    utils::next_trace_id,
};

/// A [`Layer`] that collects newrelic-compatible data from `tracing` span/event.
///
/// This layer collects data from `tracing` span/event and reports them using [`Reporter`].
///
/// By default it will includes fields, `span_name` and `duration`.
/// You can override the default behavior using `with_*` methods.
///
/// [`Layer`]: tracing_subscriber::layer::Layer
pub struct NewRelicLayer<R: Reporter> {
    pub(crate) reporter: R,
}

impl NewRelicLayer<BlockingReporter> {
    /// Create a new `NewRelicLayer` with given reporter
    pub fn blocking(api: Api) -> Self {
        NewRelicLayer {
            reporter: BlockingReporter::new(api),
        }
    }
}

impl<S, R> Layer<S> for NewRelicLayer<R>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    R: Reporter + 'static,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("span not found");
        let metadata = span.metadata();

        // create a new span
        let mut nr_span = NrSpan::new(metadata.name().to_string());

        nr_span.attributes.insert(
            "source",
            format!(
                "{}:{}",
                metadata.file().unwrap_or_default(),
                metadata.line().unwrap_or_default()
            ),
        );

        // record span attributes
        attrs.record(&mut nr_span.attributes);

        // insert into extensions
        span.extensions_mut().insert(nr_span);
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("span not found");
        let mut extensions = span.extensions_mut();

        if let Some(nr_span) = extensions.get_mut::<NrSpan>() {
            values.record(&mut nr_span.attributes);
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        // ignore event that is out of current span
        if let Some(id) = ctx.current_span().id() {
            let span = ctx.span(id).expect("span not found");
            let mut extensions = span.extensions_mut();
            let metadata = event.metadata();

            // create a log
            let mut nr_log = NrLog::new(metadata.level().to_string());

            if let Some(span_id) = extensions.get_mut::<NrSpan>().map(|s| s.id.clone()) {
                // sÎet linking metadata
                // https://github.com/newrelic/node-newrelic/blob/91967dd5cd997aa283b8aa0b2fdacc2a5f10a628/api.js#L132
                nr_log.attributes.insert("span.id", span_id);
            }

            nr_log.attributes.insert(
                "source",
                format!(
                    "{}:{}",
                    metadata.file().unwrap_or_default(),
                    metadata.line().unwrap_or_default()
                ),
            );

            // record event attributes
            event.record(&mut nr_log.attributes);

            // insert into extensions
            if let Some(nr_logs) = extensions.get_mut::<Vec<NrLog>>() {
                nr_logs.push(nr_log);
            } else {
                extensions.insert(vec![nr_log]);
            }
        }
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let span = ctx.span(&id).expect("span not found");
        let mut extensions = span.extensions_mut();

        if let Some(mut nr_span) = extensions.remove::<NrSpan>() {
            // update duration
            nr_span.update_duration();

            let mut logs = extensions.remove::<Vec<NrLog>>().unwrap_or_default();

            let mut spans = vec![nr_span];

            if let Some(mut children) = extensions.remove::<Vec<NrSpan>>() {
                spans.append(&mut children);
            };

            if let Some(parent) = span.parent() {
                let mut parent_extensions = parent.extensions_mut();

                if let Some(parent_span) = parent_extensions.get_mut::<NrSpan>() {
                    spans[0]
                        .attributes
                        .insert("parent.id", parent_span.id.clone());

                    for span in &mut spans {
                        span.attributes.merge(parent_span.attributes.clone());
                    }

                    for log in &mut logs {
                        log.attributes.merge(parent_span.attributes.clone());
                    }

                    if let Some(siblings) = parent_extensions.get_mut::<Vec<NrSpan>>() {
                        siblings.append(&mut spans);
                    } else {
                        parent_extensions.insert(spans);
                    }

                    if !logs.is_empty() {
                        if let Some(parent_logs) = parent_extensions.get_mut::<Vec<NrLog>>() {
                            parent_logs.append(&mut logs);
                        } else {
                            parent_extensions.insert(logs);
                        }
                    }
                }

                return;
            }

            let trace_id = next_trace_id();

            for span in &mut spans {
                span.trace_id = Some(trace_id.clone());
            }

            for log in &mut logs {
                log.attributes.insert("trace.id", trace_id.clone());
            }

            dbg!(&spans);
            dbg!(&logs);

            self.reporter.report(spans, logs);
        }
    }
}
