use serde::Serialize;
use std::fmt::Debug;
use std::{collections::HashMap, time::SystemTime};
use tracing_core::field::{Field, Visit};

use crate::utils::{next_span_id, now, serialize_system_time};

#[derive(Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum Value {
    I64(i64),
    U64(u64),
    Bool(bool),
    String(String),
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::I64(i)
    }
}

impl From<u64> for Value {
    fn from(i: u64) -> Self {
        Value::U64(i)
    }
}

impl From<bool> for Value {
    fn from(i: bool) -> Self {
        Value::Bool(i)
    }
}

impl From<String> for Value {
    fn from(i: String) -> Self {
        Value::String(i)
    }
}

impl From<&str> for Value {
    fn from(i: &str) -> Self {
        Value::String(i.to_string())
    }
}

#[derive(Serialize, Default, Clone, Debug)]
pub struct NrAttributes(pub HashMap<String, Value>);

impl NrAttributes {
    pub fn insert<V: Into<Value>>(&mut self, key: &str, val: V) {
        self.0.insert(key.into(), val.into());
    }

    pub fn merge(&mut self, map: NrAttributes) {
        for (k, v) in map.0 {
            self.0.entry(k).or_insert(v);
        }
    }
}

impl Visit for NrAttributes {
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.insert(field.name(), value);
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.insert(field.name(), value);
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.insert(field.name(), value);
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.insert(field.name(), value.to_string());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.insert(field.name(), format!("{:?}", value));
    }
}

#[derive(Serialize, Debug)]
pub struct NrSpan {
    /// Unique identifier for this span.
    pub id: String,
    /// Unique identifier shared by all spans within a single trace.
    #[serde(rename = "trace.id")]
    pub trace_id: Option<String>,
    #[serde(serialize_with = "serialize_system_time")]
    /// Span start time in milliseconds since the Unix epoch.
    pub timestamp: SystemTime,
    /// Any set of key: value pairs that add more details about a span.
    pub attributes: NrAttributes,
}

impl NrSpan {
    pub fn new(name: String) -> Self {
        let mut attributes = NrAttributes::default();
        attributes.insert("name", name);

        NrSpan {
            id: next_span_id(),
            trace_id: None,
            timestamp: now(),
            attributes,
        }
    }

    pub fn update_duration(&mut self) {
        if let Ok(duration) = SystemTime::now().duration_since(self.timestamp) {
            self.attributes
                .insert("duration.ms", duration.as_millis() as u64);
        }
    }
}

#[derive(Serialize, Debug)]
pub struct NrLog {
    #[serde(serialize_with = "serialize_system_time")]
    pub timestamp: SystemTime,
    // event contains a field named message
    // pub message: String,
    pub logtype: String,
    pub attributes: NrAttributes,
}

impl NrLog {
    pub fn new(logtype: String) -> Self {
        NrLog {
            timestamp: now(),
            logtype,
            attributes: NrAttributes::default(),
        }
    }
}
