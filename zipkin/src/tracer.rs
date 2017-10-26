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

use {Annotation, BinaryAnnotation, Endpoint, Report, Sample, Span, SpanId, TraceContext, TraceId};
use report::LoggingReporter;
use sample::AlwaysSampler;

const CS_SET: u8 = 1 << 0;
const CR_SET: u8 = 1 << 1;
const SR_SET: u8 = 1 << 2;
const SS_SET: u8 = 1 << 3;
const LC_SET: u8 = 1 << 4;

enum Kind {
    Client,
    Server,
    Local,
}

enum SpanState {
    Real {
        name: String,
        start_time: SystemTime,
        start_instant: Instant,
        shared: bool,
        kind: Kind,
        annotations: Vec<Annotation>,
        binary_annotations: Vec<BinaryAnnotation>,
        annotation_set: u8,
    },
    Nop,
}

/// A guard object for the thread-local current trace context.
///
/// It will restore the previous trace context when it drops.
pub struct CurrentGuard {
    tracer: Tracer,
    prev: Option<TraceContext>,
    done: bool,
    // make sure this type is !Send and !Sync since it pokes at thread locals
    _p: PhantomData<*const ()>,
}

impl Drop for CurrentGuard {
    fn drop(&mut self) {
        self.detach();
    }
}

impl CurrentGuard {
    fn detach(&mut self) {
        if self.done {
            return;
        }
        self.done = true;

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

/// An open span.
///
/// This is a guard object - the span will be finished and reported when it
/// falls out of scope.
///
/// An open span is either a "local", "client", or "server" span. Local spans
/// are for operations that are local and do not talk to remote services. Client
/// spans are for operations that involve making a request to a remote service.
/// Server spans are for the other end of that - a server which receives a
/// requests from a remote client.
///
/// This controls how the span is reported - local spans have an "lc" binary
/// annotation identifying the local endpoint, client spans have "cs" and "cr"
/// annotations indicating the times at which the request was sent and the
/// response was received, and server spans have "sr" and "ss" annotations
/// indicating the times at which the request was received and the response was
/// sent.
///
/// A span defaults to being local - use the `client()` and `server()` methods
/// to change that for an individual span.
pub struct OpenSpan {
    guard: CurrentGuard,
    context: TraceContext,
    state: SpanState,
}

impl Drop for OpenSpan {
    fn drop(&mut self) {
        if let SpanState::Real {
            name,
            start_time,
            start_instant,
            shared,
            kind,
            annotations,
            binary_annotations,
            annotation_set,
        } = mem::replace(&mut self.state, SpanState::Nop)
        {
            let mut span = Span::builder();

            if let Some(parent_id) = self.context.parent_id() {
                span.parent_id(parent_id);
            }

            if !shared {
                span.timestamp(start_time).duration(start_instant.elapsed());
            }

            // fill in standard annotations if they haven't already been set
            match kind {
                Kind::Client => {
                    if annotation_set & CS_SET == 0 {
                        let annotation = Annotation::builder()
                            .timestamp(start_time)
                            .endpoint(self.guard.tracer.0.local_endpoint.clone())
                            .build("cs");
                        span.annotation(annotation);
                    }

                    if annotation_set & CR_SET == 0 {
                        let annotation = Annotation::builder()
                            .endpoint(self.guard.tracer.0.local_endpoint.clone())
                            .build("cr");
                        span.annotation(annotation);
                    }
                }
                Kind::Server => {
                    if annotation_set & SR_SET == 0 {
                        let annotation = Annotation::builder()
                            .timestamp(start_time)
                            .endpoint(self.guard.tracer.0.local_endpoint.clone())
                            .build("sr");
                        span.annotation(annotation);
                    }

                    if annotation_set & SS_SET == 0 {
                        let annotation = Annotation::builder()
                            .endpoint(self.guard.tracer.0.local_endpoint.clone())
                            .build("ss");
                        span.annotation(annotation);
                    }
                }
                Kind::Local => {
                    if annotation_set & LC_SET == 0 {
                        let binary_annotation = BinaryAnnotation::builder()
                            .endpoint(self.guard.tracer.0.local_endpoint.clone())
                            .build("lc", "");
                        span.binary_annotation(binary_annotation);
                    }
                }
            }

            span.annotations(annotations);
            span.binary_annotations(binary_annotations);

            let span = span.build(self.context.trace_id(), name, self.context.span_id());
            self.guard.tracer.0.reporter.report(&span);
        }
    }
}

impl OpenSpan {
    /// Returns the context associated with this span.
    pub fn context(&self) -> TraceContext {
        self.context
    }

    /// Sets this span to be a server span.
    ///
    /// When completed, server spans have "sr" and "ss" annotations
    /// automatically attached.
    pub fn server(&mut self) {
        if let SpanState::Real { ref mut kind, .. } = self.state {
            *kind = Kind::Server;
        }
    }

    /// Sets this span to be a client span.
    ///
    /// When completed, client spans have "cs" and "cr" annotations
    /// automatically attached.
    pub fn client(&mut self) {
        if let SpanState::Real { ref mut kind, .. } = self.state {
            *kind = Kind::Client;
        }
    }

    /// Attaches an annotation to this span.
    pub fn annotate(&mut self, value: &str) {
        if let SpanState::Real {
            ref mut annotations,
            ref mut annotation_set,
            ..
        } = self.state
        {
            match value {
                "cs" => *annotation_set |= CS_SET,
                "cr" => *annotation_set |= CR_SET,
                "sr" => *annotation_set |= SR_SET,
                "ss" => *annotation_set |= SS_SET,
                _ => {}
            }

            let annotation = Annotation::builder()
                .endpoint(self.guard.tracer.0.local_endpoint.clone())
                .build(value);
            annotations.push(annotation);
        }
    }

    /// Attaches a binary annotation to this span.
    pub fn tag(&mut self, key: &str, value: &str) {
        if let SpanState::Real {
            ref mut binary_annotations,
            ref mut annotation_set,
            ..
        } = self.state
        {
            if key == "lc" {
                *annotation_set |= LC_SET;
            }
            let binary_annotation = BinaryAnnotation::builder()
                .endpoint(self.guard.tracer.0.local_endpoint.clone())
                .build(key, value);
            binary_annotations.push(binary_annotation);
        }
    }

    /// "Detaches" this span from the `Tracer`.
    ///
    /// The parent of this span is normally re-registered as the `Tracer`'s
    /// current span when the `OpenSpan` drops. This method will cause that to
    /// happen immediately. New child spans created from the `Tracer` afterwards
    /// will be parented to this span's parent rather than this span itself.
    ///
    /// This is intended to be used to enable the creation of multiple
    /// "parallel" spans.
    pub fn detach(&mut self) {
        self.guard.detach();
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
    pub fn new_trace(&self, name: &str) -> OpenSpan {
        self.ensure_sampled(name, self.next_context(None, None, false), false)
    }

    /// Joins an existing trace.
    ///
    /// The context can come from, for example, the headers of an HTTP request.
    pub fn join_trace(&self, name: &str, context: TraceContext) -> OpenSpan {
        self.ensure_sampled(name, context, true)
    }

    /// Starts a new span with the specified parent.
    pub fn new_child(&self, name: &str, parent: TraceContext) -> OpenSpan {
        if parent.sampled() == Some(false) {
            return self.new_span(parent, SpanState::Nop);
        }

        let context = self.next_context(Some(parent), parent.sampled(), parent.debug());
        self.ensure_sampled(name, context, false)
    }

    /// Starts a new trace parented to the current span if one exists.
    pub fn next_span(&self, name: &str) -> OpenSpan {
        match self.current() {
            Some(context) => self.new_child(name, context),
            None => self.new_trace(name),
        }
    }

    fn ensure_sampled(&self, name: &str, mut context: TraceContext, mut shared: bool) -> OpenSpan {
        if let None = context.sampled() {
            context.sampled = Some(self.0.sampler.sample(context.trace_id()));
            // since the thing we got this context from didn't indicate if it should be sampled
            // we can't assume they're recording the start/duration for us.
            shared = false;
        }

        let state = match context.sampled() {
            Some(false) => SpanState::Nop,
            _ => SpanState::Real {
                name: name.to_string(),
                start_time: SystemTime::now(),
                start_instant: Instant::now(),
                shared,
                kind: Kind::Local,
                annotations: vec![],
                binary_annotations: vec![],
                annotation_set: 0,
            },
        };

        self.new_span(context, state)
    }

    fn new_span(&self, context: TraceContext, state: SpanState) -> OpenSpan {
        let guard = self.set_current(context);

        OpenSpan {
            guard,
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
            done: false,
            _p: PhantomData,
        }
    }

    /// Returns this thread's current trace context.
    pub fn current(&self) -> Option<TraceContext> {
        self.0.current.get_cloned()
    }

    fn next_context(
        &self,
        parent: Option<TraceContext>,
        mut sampled: Option<bool>,
        mut debug: bool,
    ) -> TraceContext {
        let mut id = [0; 8];
        rand::thread_rng().fill_bytes(&mut id);

        let mut context = TraceContext::builder();
        let trace_id = match parent {
            Some(parent) => {
                context.parent_id(parent.span_id());
                sampled = parent.sampled();
                debug = parent.debug();

                parent.trace_id()
            }
            None => TraceId::from(id),
        };

        context.debug(debug);
        if let Some(sampled) = sampled {
            context.sampled(sampled);
        }

        let span_id = SpanId::from(id);
        context.build(trace_id, span_id)
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
        let reporter = self.reporter.take().unwrap_or_else(
            || Box::new(LoggingReporter),
        );

        let sampler = self.sampler.take().unwrap_or_else(
            || Box::new(AlwaysSampler),
        );

        let inner = Inner {
            current: ThreadLocal::new(),
            local_endpoint,
            reporter,
            sampler,
        };

        Tracer(Arc::new(inner))
    }
}
