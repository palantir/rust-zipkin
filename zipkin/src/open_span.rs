use crate::{span, tracer, Annotation, CurrentGuard, Endpoint, Kind, TraceContext};
use std::mem;
use std::time::Instant;

/// A type indicating that an `OpenSpan` is "attached" to the current thread.
pub struct Attached(CurrentGuard);

/// A type indicating that an `OpenSpan` is "detached" from the current thread.
pub struct Detached(());

pub(crate) enum SpanState {
    Real {
        span: span::Builder,
        start_instant: Instant,
    },
    Nop,
}

/// An open span.
///
/// This is a guard object - the span will be finished and reported when it
/// falls out of scope.
///
/// Spans can either be "attached" to or "detached" from their tracer. An attached span manages its
/// tracer's current span - it acts like a `CurrentGuard`. A detached span does not but is `Send`
/// unlike an attached span. Spans are attached by default, but can be detached or reattached via
/// the `detach` and `attach` methods.
///
/// Detached spans are intended for use when you need to manually maintain the current trace
/// context. For example, when working with nonblocking futures a single OS thread is managing many
/// separate tasks. The `futures-zipkin` crate provides a wrapper type which ensures a context is
/// registered as the current whenever a task is running. If some computation starts executing on
/// one thread and finishes executing on another, you can detach the span, send it to the other
/// thread, and then reattach it to properly model that behavior.
pub struct OpenSpan<T> {
    _mode: T,
    context: TraceContext,
    state: SpanState,
}

impl<T> Drop for OpenSpan<T> {
    fn drop(&mut self) {
        if let SpanState::Real {
            span,
            start_instant,
        } = &mut self.state
        {
            if let Some(tracer) = tracer::TRACER.borrow() {
                let span = span.duration(start_instant.elapsed()).build();
                tracer.reporter.report2(span);
            }
        }
    }
}

impl<T> OpenSpan<T> {
    /// Returns the context associated with this span.
    #[inline]
    pub fn context(&self) -> TraceContext {
        self.context
    }

    /// Sets the name of this span.
    #[inline]
    pub fn name(&mut self, name: &str) {
        if let SpanState::Real { span, .. } = &mut self.state {
            span.name(name);
        }
    }

    /// A builder-style version of `name`.
    #[inline]
    pub fn with_name(mut self, name: &str) -> OpenSpan<T> {
        self.name(name);
        self
    }

    /// Sets the kind of this span.
    #[inline]
    pub fn kind(&mut self, kind: Kind) {
        if let SpanState::Real { span, .. } = &mut self.state {
            span.kind(kind);
        }
    }

    /// A builder-style version of `kind`.
    #[inline]
    pub fn with_kind(mut self, kind: Kind) -> OpenSpan<T> {
        self.kind(kind);
        self
    }

    /// Sets the remote endpoint of this span.
    #[inline]
    pub fn remote_endpoint(&mut self, remote_endpoint: Endpoint) {
        if let SpanState::Real { span, .. } = &mut self.state {
            span.remote_endpoint(remote_endpoint);
        }
    }

    /// A builder-style version of `remote_endpoint`.
    #[inline]
    pub fn with_remote_endpoint(mut self, remote_endpoint: Endpoint) -> OpenSpan<T> {
        self.remote_endpoint(remote_endpoint);
        self
    }

    /// Attaches an annotation to this span.
    #[inline]
    pub fn annotate(&mut self, value: &str) {
        if let SpanState::Real { span, .. } = &mut self.state {
            let annotation = Annotation::now(value);
            span.annotation(annotation);
        }
    }

    /// A builder-style version of `annotate`.
    #[inline]
    pub fn with_annotation(mut self, value: &str) -> OpenSpan<T> {
        self.annotate(value);
        self
    }

    /// Attaches a tag to this span.
    #[inline]
    pub fn tag(&mut self, key: &str, value: &str) {
        if let SpanState::Real { span, .. } = &mut self.state {
            span.tag(key, value);
        }
    }

    /// A builder-style version of `tag`.
    #[inline]
    pub fn with_tag(mut self, key: &str, value: &str) -> OpenSpan<T> {
        self.tag(key, value);
        self
    }
}

impl OpenSpan<Attached> {
    #[inline]
    pub(crate) fn new(context: TraceContext, state: SpanState) -> OpenSpan<Attached> {
        OpenSpan {
            _mode: Attached(crate::set_current(context)),
            context,
            state,
        }
    }

    /// Detaches this span's context from the tracer.
    #[inline]
    pub fn detach(mut self) -> OpenSpan<Detached> {
        OpenSpan {
            _mode: Detached(()),
            context: self.context,
            // since we've swapped in Nop here, self's Drop impl won't do anything
            state: mem::replace(&mut self.state, SpanState::Nop),
        }
    }
}

impl OpenSpan<Detached> {
    /// Re-attaches this span's context to the tracer.
    #[inline]
    pub fn attach(mut self) -> OpenSpan<Attached> {
        OpenSpan {
            _mode: Attached(crate::set_current(self.context)),
            context: self.context,
            // since we've swapped in Nop here, self's Drop impl won't do anything
            state: mem::replace(&mut self.state, SpanState::Nop),
        }
    }
}
