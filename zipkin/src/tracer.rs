//  Copyright 2017 Palantir Technologies, Inc.
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

//! Tracers.
use rand::{self, Rng};
use std::marker::PhantomData;
use std::mem;
use std::sync::Arc;
use std::time::{Instant, SystemTime};
use thread_local_object::ThreadLocal;

use report::LoggingReporter;
use sample::AlwaysSampler;
use span;
use trace_context;
use tracer::private::Sealed;
use {Annotation, Endpoint, Kind, Report, Sample, SamplingFlags, Span, SpanId, TraceContext,
     TraceId};

/// A guard object for the thread-local current trace context.
///
/// It will restore the previous trace context when it drops.
pub struct CurrentGuard {
    tracer: Tracer,
    prev: Option<TraceContext>,
    // make sure this type is !Send since it pokes at thread locals
    _p: PhantomData<*const ()>,
}

unsafe impl Sync for CurrentGuard {}

impl Drop for CurrentGuard {
    fn drop(&mut self) {
        match self.prev.take() {
            Some(prev) => {
                self.tracer.0.current.set(prev);
            }
            None => {
                self.tracer.0.current.remove();
            }
        }
    }
}

mod private {
    use Tracer;

    pub trait Sealed {
        fn tracer(&self) -> &Tracer;
    }
}

/// A type indicating that an `OpenSpan` is "attached" to the tracer's with respect to context
/// management.
pub struct Attached(CurrentGuard);

impl Attachment for Attached {}

impl Sealed for Attached {
    fn tracer(&self) -> &Tracer {
        &self.0.tracer
    }
}

/// A type indicating that an `OpenSpan` is "detached" from the tracer's with respect to context
/// management.
pub struct Detached(Tracer);

impl Attachment for Detached {}

impl Sealed for Detached {
    fn tracer(&self) -> &Tracer {
        &self.0
    }
}

/// A marker trait for types which parameterize an `OpenSpan`'s attachment.
///
/// It is "sealed" such that it cannot be implemented outside of this crate.
pub trait Attachment: Sealed {}

enum SpanState {
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
pub struct OpenSpan<T>
where
    T: Attachment,
{
    attachment: T,
    context: TraceContext,
    state: SpanState,
}

impl<T> Drop for OpenSpan<T>
where
    T: Attachment,
{
    fn drop(&mut self) {
        if let SpanState::Real {
            mut span,
            start_instant,
        } = mem::replace(&mut self.state, SpanState::Nop)
        {
            let span = span.duration(start_instant.elapsed()).build();
            self.attachment.tracer().0.reporter.report(&span);
        }
    }
}

impl<T> OpenSpan<T>
where
    T: Attachment,
{
    /// Returns the context associated with this span.
    pub fn context(&self) -> TraceContext {
        self.context
    }

    /// Sets the name of this span.
    pub fn name(&mut self, name: &str) {
        if let SpanState::Real { ref mut span, .. } = self.state {
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
    pub fn kind(&mut self, kind: Kind) {
        if let SpanState::Real { ref mut span, .. } = self.state {
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
    pub fn remote_endpoint(&mut self, remote_endpoint: Endpoint) {
        if let SpanState::Real { ref mut span, .. } = self.state {
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
    pub fn annotate(&mut self, value: &str) {
        if let SpanState::Real { ref mut span, .. } = self.state {
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
    pub fn tag(&mut self, key: &str, value: &str) {
        if let SpanState::Real { ref mut span, .. } = self.state {
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
    /// Detaches this span's context from the tracer.
    #[inline]
    pub fn detach(mut self) -> OpenSpan<Detached> {
        OpenSpan {
            attachment: Detached(self.attachment.tracer().clone()),
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
            attachment: Attached(self.attachment.tracer().set_current(self.context)),
            context: self.context,
            // since we've swapped in Nop here, self's Drop impl won't do anything
            state: mem::replace(&mut self.state, SpanState::Nop),
        }
    }
}

struct Inner {
    current: ThreadLocal<TraceContext>,
    local_endpoint: Endpoint,
    reporter: Box<Report + Sync + Send>,
    sampler: Box<Sample + Sync + Send>,
}

/// The root tracing object.
///
/// Each thread has its own current span state - the `Tracer` should be a single
/// global resource.
#[derive(Clone)]
pub struct Tracer(Arc<Inner>);

impl Tracer {
    /// Creates a `Tracer` builder.
    pub fn builder() -> Builder {
        Builder {
            reporter: None,
            sampler: None,
        }
    }

    /// Starts a new trace with no parent.
    pub fn new_trace(&self) -> OpenSpan<Attached> {
        self.new_trace_from(SamplingFlags::default())
    }

    /// Starts a new trace with no parent with specific sampling flags.
    pub fn new_trace_from(&self, flags: SamplingFlags) -> OpenSpan<Attached> {
        let id = self.next_id();
        let context = TraceContext::builder()
            .trace_id(TraceId::from(id))
            .span_id(SpanId::from(id))
            .sampling_flags(flags)
            .build();
        self.ensure_sampled(context, false)
    }

    /// Joins an existing trace.
    ///
    /// The context can come from, for example, the headers of an HTTP request.
    pub fn join_trace(&self, context: TraceContext) -> OpenSpan<Attached> {
        self.ensure_sampled(context, true)
    }

    /// Starts a new span with the specified parent.
    pub fn new_child(&self, parent: TraceContext) -> OpenSpan<Attached> {
        let id = self.next_id();
        let context = TraceContext::builder()
            .trace_id(parent.trace_id())
            .parent_id(parent.span_id())
            .span_id(SpanId::from(id))
            .sampling_flags(parent.sampling_flags())
            .build();
        self.ensure_sampled(context, false)
    }

    /// Starts a new trace parented to the current span if one exists.
    pub fn next_span(&self) -> OpenSpan<Attached> {
        match self.current() {
            Some(context) => self.new_child(context),
            None => self.new_trace(),
        }
    }

    fn next_id(&self) -> [u8; 8] {
        let mut id = [0; 8];
        rand::thread_rng().fill_bytes(&mut id);
        id
    }

    fn ensure_sampled(&self, mut context: TraceContext, mut shared: bool) -> OpenSpan<Attached> {
        if let None = context.sampled() {
            context = trace_context::Builder::from(context)
                .sampled(self.0.sampler.sample(context.trace_id()))
                .build();
            // since the thing we got this context from didn't indicate if it should be sampled
            // we can't assume they're recording the span as well.
            shared = false;
        }

        let state = match context.sampled() {
            Some(false) => SpanState::Nop,
            _ => {
                let mut span = Span::builder();
                span.trace_id(context.trace_id())
                    .id(context.span_id())
                    .timestamp(SystemTime::now())
                    .shared(shared)
                    .local_endpoint(self.0.local_endpoint.clone());

                if let Some(parent_id) = context.parent_id() {
                    span.parent_id(parent_id);
                }

                SpanState::Real {
                    span,
                    start_instant: Instant::now(),
                }
            }
        };

        self.new_span(context, state)
    }

    fn new_span(&self, context: TraceContext, state: SpanState) -> OpenSpan<Attached> {
        let guard = self.set_current(context);

        OpenSpan {
            attachment: Attached(guard),
            context,
            state,
        }
    }

    /// Sets this thread's current trace context.
    ///
    /// This method does not start a span. It is designed to be used when
    /// propagating the trace of an existing span to a new thread.
    ///
    /// A guard object is returned which will restore the previous trace context
    /// when it falls out of scope.
    pub fn set_current(&self, context: TraceContext) -> CurrentGuard {
        CurrentGuard {
            tracer: self.clone(),
            prev: self.0.current.set(context),
            _p: PhantomData,
        }
    }

    /// Returns this thread's current trace context.
    pub fn current(&self) -> Option<TraceContext> {
        self.0.current.get_cloned()
    }
}

/// A builder type for `Tracer`s.
pub struct Builder {
    reporter: Option<Box<Report + Sync + Send>>,
    sampler: Option<Box<Sample + Sync + Send>>,
}

impl Builder {
    /// Sets the reporter which consumes completed spans.
    ///
    /// Defaults to the `LoggingReporter`.
    pub fn reporter(&mut self, reporter: Box<Report + Sync + Send>) -> &mut Builder {
        self.reporter = Some(reporter);
        self
    }

    /// Sets the sampler which determines if a trace should be tracked and reported.
    ///
    /// Defaults to the `AlwaysSampler`.
    pub fn sampler(&mut self, sampler: Box<Sample + Sync + Send>) -> &mut Builder {
        self.sampler = Some(sampler);
        self
    }

    /// Constructs a new `Tracer`.
    pub fn build(&mut self, local_endpoint: Endpoint) -> Tracer {
        let reporter = self.reporter
            .take()
            .unwrap_or_else(|| Box::new(LoggingReporter));

        let sampler = self.sampler
            .take()
            .unwrap_or_else(|| Box::new(AlwaysSampler));

        let inner = Inner {
            current: ThreadLocal::new(),
            local_endpoint,
            reporter,
            sampler,
        };

        Tracer(Arc::new(inner))
    }
}
