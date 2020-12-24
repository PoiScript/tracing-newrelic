use std::collections::HashMap;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use uuid::Uuid;

pub use newrelic::attribute::Value;

/// Trace data collected from Event
pub struct TraceEvent {
    /// Event id
    pub id: String,
    /// Event created time
    pub created: SystemTime,
    /// Atrributes collected from Event
    pub attrs: HashMap<String, Value>,
}

impl TraceEvent {
    pub fn new() -> TraceEvent {
        TraceEvent {
            id: Uuid::new_v4().to_string(),
            created: SystemTime::now(),
            attrs: HashMap::new(),
        }
    }

    pub fn set_attribute(&mut self, key: &str, value: Value) {
        self.attrs.insert(key.into(), value);
    }

    pub fn set_parent_id(&mut self, id: &str) {
        self.set_attribute("parent.id", id.into());
    }
}

/// Trace data collected from Span
pub struct TraceSpan {
    /// Attributes collected from Span
    pub attrs: HashMap<String, newrelic::attribute::Value>,
    /// Events collected from Span
    ///
    /// Each `TraceSpan` contains at least one event
    /// which will be the root span in new relic
    pub events: Vec<TraceEvent>,
}

impl TraceSpan {
    pub fn new() -> TraceSpan {
        TraceSpan {
            events: vec![TraceEvent::new()],
            attrs: HashMap::new(),
        }
    }

    pub fn root(&mut self) -> &mut TraceEvent {
        &mut self.events[0]
    }

    pub fn update_duration(&mut self, key: &str) {
        let created = self.root().created;
        if let Ok(duration) = SystemTime::now().duration_since(created) {
            self.root()
                .set_attribute(key, (duration.as_millis() as u64).into());
        }
    }

    pub fn append(&mut self, mut span: TraceSpan) {
        span.root().set_parent_id(&self.root().id);

        for event in span.events.iter_mut() {
            for (key, value) in &span.attrs {
                event
                    .attrs
                    .entry(key.clone())
                    .or_insert_with(|| value.clone());
            }
        }

        self.events.append(&mut span.events);
    }

    pub fn into_batch(self) -> newrelic::SpanBatch {
        let TraceSpan { events, attrs, .. } = self;

        let mut batch = newrelic::SpanBatch::new();

        let trace_id = Uuid::new_v4().to_string();

        for (key, value) in attrs {
            batch.set_attribute(&key, value);
        }

        for event in events {
            let mut span = newrelic::Span::new(
                &event.id,
                &trace_id,
                event
                    .created
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or_default(),
            );

            for (key, value) in event.attrs {
                span.set_attribute(&key, value);
            }

            batch.record(span);
        }

        batch
    }
}

// ===== impl Visit =====

use tracing_core::field::{Field, Visit};

impl Visit for TraceSpan {
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.attrs.insert(field.name().to_string(), value.into());
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.attrs.insert(field.name().to_string(), value.into());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.attrs.insert(field.name().to_string(), value.into());
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.attrs.insert(field.name().to_string(), value.into());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.attrs
            .insert(field.name().into(), Value::Str(format!("{:?}", value)));
    }
}

impl Visit for TraceEvent {
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.attrs.insert(field.name().to_string(), value.into());
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.attrs.insert(field.name().to_string(), value.into());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.attrs.insert(field.name().to_string(), value.into());
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.attrs.insert(field.name().to_string(), value.into());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.attrs
            .insert(field.name().into(), Value::Str(format!("{:?}", value)));
    }
}