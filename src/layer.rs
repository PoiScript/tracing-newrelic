use tracing_core::span::{self, Attributes, Id, Record};
use tracing_core::Metadata;
use tracing_core::{Event, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

use crate::reporter::Reporter;
use crate::span::{TraceEvent, TraceSpan, Value};

macro_rules! with_struct {
    ($name:ident, $default:expr) => {
        pub struct $name(Option<&'static str>);

        impl From<&'static str> for $name {
            fn from(val: &'static str) -> Self {
                $name(Some(val))
            }
        }

        impl From<bool> for $name {
            fn from(val: bool) -> Self {
                $name(if val { Some($default) } else { None })
            }
        }
    };
}

with_struct!(WithName, "name");
with_struct!(WithLevel, "level");
with_struct!(WithTarget, "source.target");
with_struct!(WithModulePath, "source.module");
with_struct!(WithFile, "source.file");
with_struct!(WithLine, "source.line");
with_struct!(WithDuration, "duration.ms");

macro_rules! with_method {
    ($(#[$meta:meta])* $name:ident, $ty:ident) => {
        $(#[$meta])*
        pub fn $name<T>(mut self, val: T) -> Self
        where
            T: Into<$ty>,
        {
            self.$name = val.into();
            self
        }
    };
}

/// A [`Layer`] that collects newrelic-compatible data from `tracing` span/event.
///
/// This layer collects data from `tracing` span/event and reports them using [`Reporter`].
///
/// By default it will includes information from fields, `name` from metadata and execute `duration`.
/// You can override the default behavior using `with_*` methods.
///
/// [`Layer`]: tracing_subscriber::layer::Layer
pub struct NewRelicLayer<R: Reporter> {
    reporter: R,
    with_event: bool,
    with_name: WithName,
    with_level: WithLevel,
    with_target: WithTarget,
    with_module_path: WithModulePath,
    with_file: WithFile,
    with_line: WithLine,
    with_duration: WithDuration,
}

impl<R> NewRelicLayer<R>
where
    R: Reporter,
{
    /// Create a new `NewRelicLayer` with given reporter
    pub fn new(reporter: R) -> Self {
        NewRelicLayer {
            reporter,
            with_event: true,
            with_name: true.into(),
            with_level: false.into(),
            with_target: false.into(),
            with_module_path: false.into(),
            with_file: false.into(),
            with_line: false.into(),
            with_duration: true.into(),
        }
    }

    /// Whether or not the events of span is collected
    pub fn with_event(mut self, enable: bool) -> Self {
        self.with_event = enable;
        self
    }

    with_method!(
        /// Whether or not the [`name`] of span/event is collected
        ///
        /// + `false`: disable
        /// + `true`: enable with default attribute key `name`
        /// + `&'static str`: enable with custom attribute key
        ///
        /// [`name`]: tracing_core::Metadata::name
        with_name,
        WithName
    );
    with_method!(
        /// Whether or not the [`level`] of span/event is collected
        ///
        /// + `false`: disable
        /// + `true`: enable with default attribute key `level`
        /// + `&'static str`: enable with custom attribute key
        ///
        /// [`level`]: tracing_core::Metadata::level
        with_level,
        WithLevel
    );
    with_method!(
        /// Whether or not the [`target`] of span/event is collected
        ///
        /// + `false`: disable
        /// + `true`: enable with default attribute key `source.target`
        /// + `&'static str`: enable with custom attribute key
        ///
        /// [`target`]: tracing_core::Metadata::target
        with_target,
        WithTarget
    );
    with_method!(
        /// Whether or not the [`module_path`] of span/event is collected
        ///
        /// + `false`: disable
        /// + `true`: enable with default attribute key `source.module`
        /// + `&'static str`: enable with custom attribute key
        ///
        /// [`module_path`]: tracing_core::Metadata::module_path
        with_module_path,
        WithModulePath
    );
    with_method!(
        /// Whether or not the [`file`] of span/event is collected
        ///
        /// + `false`: disable
        /// + `true`: enable with default attribute key `source.file`
        /// + `&'static str`: enable with custom attribute key
        ///
        /// [`file`]: tracing_core::Metadata::file
        with_file,
        WithFile
    );
    with_method!(
        /// Whether or not the [`line`] of span/event is collected
        ///
        /// + `false`: disable
        /// + `true`: enable with default attribute key `source.line`
        /// + `&'static str`: enable with custom attribute key
        ///
        /// [`line`]: tracing_core::Metadata::line
        with_line,
        WithLine
    );
    with_method!(
        /// Whether or not the `duration` of span is collected
        ///
        /// + `false`: disable
        /// + `true`: enable with default attribute key `duration.ms`
        /// + `&'static str`: enable with custom attribute key
        with_duration,
        WithDuration
    );

    fn record_metadata(&self, event: &mut TraceEvent, metadata: &Metadata) {
        if let WithName(Some(key)) = self.with_name {
            event.set_attribute(key, metadata.name().into());
        }
        if let WithLevel(Some(key)) = self.with_level {
            event.set_attribute(key, Value::Str(metadata.level().to_string()));
        }
        if let WithTarget(Some(key)) = self.with_target {
            event.set_attribute(key, metadata.target().into());
        }
        if let WithModulePath(Some(key)) = self.with_module_path {
            if let Some(module_path) = metadata.module_path() {
                event.set_attribute(key, module_path.into());
            }
        }
        if let WithFile(Some(key)) = self.with_file {
            if let Some(file) = metadata.file() {
                event.set_attribute(key, file.into());
            }
        }
        if let WithLine(Some(key)) = self.with_line {
            if let Some(line) = metadata.line() {
                event.set_attribute(key, line.into());
            }
        }
    }
}

impl<S, R> Layer<S> for NewRelicLayer<R>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    R: Reporter + 'static,
{
    fn new_span(&self, attrs: &Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("not found");

        let mut extensions = span.extensions_mut();

        let mut trace_span = TraceSpan::new();

        self.record_metadata(trace_span.root(), span.metadata());

        attrs.record(&mut trace_span);

        extensions.insert(trace_span);
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("span not found");
        let mut extensions = span.extensions_mut();

        if let Some(trace_span) = extensions.get_mut::<TraceSpan>() {
            values.record(trace_span);
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        if !self.with_event {
            return;
        }

        // ignore event that is out of current span
        if let Some(id) = ctx.current_span().id() {
            let span = ctx.span(id).expect("span not found");
            let mut extensions = span.extensions_mut();

            if let Some(trace_span) = extensions.get_mut::<TraceSpan>() {
                let mut trace_event = TraceEvent::new();

                trace_event.set_parent_id(&trace_span.root().id);

                self.record_metadata(&mut trace_event, event.metadata());

                event.record(&mut trace_event);

                trace_span.events.push(trace_event);
            }
        }
    }

    fn on_close(&self, id: span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(&id).expect("span not found");
        let mut extensions = span.extensions_mut();

        if let Some(mut trace_span) = extensions.remove::<TraceSpan>() {
            if let WithDuration(Some(key)) = self.with_duration {
                trace_span.update_duration(key);
            }

            for span in span.parents() {
                let mut extensions = span.extensions_mut();

                if let Some(parent_trace) = extensions.get_mut::<TraceSpan>() {
                    parent_trace.append(trace_span);
                    return;
                }
            }

            self.reporter.report(trace_span.into_batch());
        }
    }
}
