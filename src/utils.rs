use serde::Serializer;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[inline]
pub fn next_trace_id() -> String {
    if cfg!(feature = "__testing") {
        use std::cell::RefCell;

        thread_local! {
            static COUNT: RefCell<i32> = RefCell::new(0);
        }

        COUNT.with(|count| {
            *count.borrow_mut() += 1;
            format!("trace_{}", count.borrow())
        })
    } else {
        Uuid::new_v4().to_string()
    }
}

#[inline]
pub fn next_span_id() -> String {
    if cfg!(feature = "__testing") {
        use std::cell::RefCell;

        thread_local! {
            static COUNT: RefCell<i32> = RefCell::new(0);
        }

        COUNT.with(|count| {
            *count.borrow_mut() += 1;
            format!("span_{}", count.borrow())
        })
    } else {
        Uuid::new_v4().to_string()
    }
}

#[inline]
pub fn now() -> SystemTime {
    if cfg!(feature = "__testing") {
        UNIX_EPOCH
    } else {
        SystemTime::now()
    }
}

#[inline]
pub fn serialize_system_time<S>(time: &SystemTime, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if let Ok(duration) = time.duration_since(UNIX_EPOCH) {
        s.serialize_u64(duration.as_millis() as u64)
    } else {
        s.serialize_none()
    }
}
