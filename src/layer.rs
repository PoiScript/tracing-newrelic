use std::thread::JoinHandle;

use tokio::sync::mpsc::UnboundedSender;
use tracing_core::span::{Attributes, Id, Record};
use tracing_core::{Event, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

use crate::types::{NewrAttributes, NewrCommon, NewrLog, NewrLogs, NewrSpan, NewrSpans};
use crate::utils::next_trace_id;

/// A [`Layer`] that collects newrelic-compatible data from `tracing` span/event.
///
/// [`Layer`]: tracing_subscriber::layer::Layer
pub struct NewRelicLayer {
    pub(crate) service_name: Option<String>,
    pub(crate) hostname: Option<String>,

    pub(crate) channel: Option<UnboundedSender<(NewrLogs, NewrSpans)>>,
    pub(crate) handle: Option<JoinHandle<()>>,
}

impl NewRelicLayer {
    /// Set service.name
    pub fn with_service_name(mut self, i: String) -> Self {
        self.service_name = Some(i);
        self
    }

    /// Set hostname
    pub fn with_hostname(mut self, i: String) -> Self {
        self.hostname = Some(i);
        self
    }
}

impl<S> Layer<S> for NewRelicLayer
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("span not found");
        let metadata = span.metadata();

        // create a new span
        let mut nr_span = NewrSpan::new(metadata.name().to_string());

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

        if let Some(nr_span) = extensions.get_mut::<NewrSpan>() {
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
            let mut nr_log = NewrLog::new(metadata.level());

            if let Some(span_id) = extensions.get_mut::<NewrSpan>().map(|s| s.id.clone()) {
                // add linking metadata
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
            if let Some(nr_logs) = extensions.get_mut::<Vec<NewrLog>>() {
                nr_logs.push(nr_log);
            } else {
                extensions.insert(vec![nr_log]);
            }
        }
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let span = ctx.span(&id).expect("span not found");
        let mut extensions = span.extensions_mut();

        if let Some(mut nr_span) = extensions.remove::<NewrSpan>() {
            // update duration
            nr_span.update_duration();

            let mut logs = extensions.remove::<Vec<NewrLog>>().unwrap_or_default();

            let mut spans = vec![nr_span];

            if let Some(mut children) = extensions.remove::<Vec<NewrSpan>>() {
                spans.append(&mut children);
            };

            if let Some(parent) = span.parent() {
                let mut parent_extensions = parent.extensions_mut();

                if let Some(parent_span) = parent_extensions.get_mut::<NewrSpan>() {
                    spans[0]
                        .attributes
                        .insert("parent.id", parent_span.id.clone());

                    if let Some(siblings) = parent_extensions.get_mut::<Vec<NewrSpan>>() {
                        siblings.append(&mut spans);
                    } else {
                        parent_extensions.insert(spans);
                    }

                    if !logs.is_empty() {
                        if let Some(parent_logs) = parent_extensions.get_mut::<Vec<NewrLog>>() {
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

            if let Some(channel) = &self.channel {
                let mut attributes = NewrAttributes::default();

                if let Some(service_name) = &self.service_name {
                    attributes.insert("service.name", service_name.as_str());
                }

                if let Some(hostname) = &self.hostname {
                    attributes.insert("hostname", hostname.as_str());
                }

                // TODO: error handling
                let _ = channel.send((
                    NewrLogs {
                        logs,
                        common: NewrCommon {
                            attributes: attributes.clone(),
                        },
                    },
                    NewrSpans {
                        spans,
                        common: NewrCommon { attributes },
                    },
                ));
            }
        }
    }
}

impl Drop for NewRelicLayer {
    fn drop(&mut self) {
        if let Some(channel) = self.channel.take() {
            drop(channel);
        }

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
