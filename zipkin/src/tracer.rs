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
use crate::trace_context;
use crate::{
    Attached, Endpoint, OpenSpan, Report, Sample, SamplingFlags, Span, SpanId, SpanState,
    TraceContext, TraceId,
};
use lazycell::AtomicLazyCell;
use rand::Rng;
use std::cell::Cell;
use std::error::Error;
use std::fmt;
use std::marker::PhantomData;
use std::time::{Instant, SystemTime};

thread_local! {
    static CURRENT: Cell<Option<TraceContext>> = Cell::new(None);
}

pub(crate) static TRACER: AtomicLazyCell<Tracer> = AtomicLazyCell::NONE;

/// A guard object for the thread-local current trace context.
///
/// It will restore the previous trace context when it drops.
pub struct CurrentGuard {
    prev: Option<TraceContext>,
    // make sure this type is !Send since it pokes at thread locals
    _p: PhantomData<*const ()>,
}

unsafe impl Sync for CurrentGuard {}

impl Drop for CurrentGuard {
    fn drop(&mut self) {
        CURRENT.with(|c| c.set(self.prev));
    }
}

/// Sets this thread's current trace context.
///
/// This method does not start a span. It is designed to be used when
/// propagating the trace of an existing span to a new thread.
///
/// A guard object is returned which will restore the previous trace context
/// when it falls out of scope.
pub fn set_current(context: TraceContext) -> CurrentGuard {
    CurrentGuard {
        prev: CURRENT.with(|c| c.replace(Some(context))),
        _p: PhantomData,
    }
}

/// Returns this thread's current trace context.
pub fn current() -> Option<TraceContext> {
    CURRENT.with(|c| c.get())
}

pub(crate) struct Tracer {
    pub sampler: Box<dyn Sample + Sync + Send>,
    pub reporter: Box<dyn Report + Sync + Send>,
    pub local_endpoint: Endpoint,
}

/// Initializes the global tracer.
///
/// The tracer can only be initialized once in the lifetime of a program. Spans created before this function is called
/// will be no-ops.
///
/// Returns an error if the tracer is already initialized.
pub fn set_tracer<S, R>(
    sampler: S,
    reporter: R,
    local_endpoint: Endpoint,
) -> Result<(), SetTracerError>
where
    S: Sample + 'static + Sync + Send,
    R: Report + 'static + Sync + Send,
{
    TRACER
        .fill(Tracer {
            sampler: Box::new(sampler),
            reporter: Box::new(reporter),
            local_endpoint,
        })
        .map_err(|_| SetTracerError(()))
}

/// The error returned when attempting to set a tracer when one is already installed.
#[derive(Debug)]
pub struct SetTracerError(());

impl fmt::Display for SetTracerError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("")
    }
}

impl Error for SetTracerError {}

/// Starts a new trace.
pub fn new_trace() -> OpenSpan<Attached> {
    new_trace_from(SamplingFlags::default())
}

/// Stats a new trace with specific sampling flags.
pub fn new_trace_from(flags: SamplingFlags) -> OpenSpan<Attached> {
    let id = next_id();
    let context = TraceContext::builder()
        .trace_id(TraceId::from(id))
        .span_id(SpanId::from(id))
        .sampling_flags(flags)
        .build();
    make_span(context, false)
}

/// Joins an existing trace.
///
/// The context can come from, for example, the headers of an HTTP request.
pub fn join_trace(context: TraceContext) -> OpenSpan<Attached> {
    make_span(context, true)
}

/// Stats a new span with the specified parent.
pub fn new_child(parent: TraceContext) -> OpenSpan<Attached> {
    let id = next_id();
    let context = TraceContext::builder()
        .trace_id(parent.trace_id())
        .parent_id(parent.span_id())
        .span_id(SpanId::from(id))
        .sampling_flags(parent.sampling_flags())
        .build();
    make_span(context, false)
}

/// Creates a new span parented to the current one if it exists, or starting a new trace otherwise.
pub fn next_span() -> OpenSpan<Attached> {
    match current() {
        Some(context) => new_child(context),
        None => new_trace(),
    }
}

fn next_id() -> [u8; 8] {
    let mut id = [0; 8];
    rand::thread_rng().fill(&mut id);
    id
}

fn make_span(mut context: TraceContext, mut shared: bool) -> OpenSpan<Attached> {
    let tracer = match TRACER.borrow() {
        Some(tracer) => tracer,
        None => return OpenSpan::new(context, SpanState::Nop),
    };

    if context.sampled().is_none() {
        context = trace_context::Builder::from(context)
            .sampled(tracer.sampler.sample(context.trace_id()))
            .build();
        // since the thing we got the context from didn't indicate if it should be sampled,
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
                .local_endpoint(tracer.local_endpoint.clone());

            if let Some(parent_id) = context.parent_id() {
                span.parent_id(parent_id);
            }

            SpanState::Real {
                span,
                start_instant: Instant::now(),
            }
        }
    };

    OpenSpan::new(context, state)
}
