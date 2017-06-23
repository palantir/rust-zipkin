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

//! Trace contexts.
use {TraceId, SpanId};

/// A `TraceContext` represents a distributed trace request.
///
/// It consists of a trace ID, the ID of the parent span, the ID of the
/// context's span, and flags dealing with the sampling of the span.
///
/// The trace context is sent to remote services on requests. For example,
/// it is included in a standard set of headers in HTTP requests.
#[derive(Copy, Clone)]
pub struct TraceContext {
    trace_id: TraceId,
    parent_id: Option<SpanId>,
    span_id: SpanId,
    pub(crate) sampled: Option<bool>,
    debug: bool,
}

impl TraceContext {
    /// Returns a builder used to construct a `TraceContext`.
    pub fn builder() -> Builder {
        Builder {
            parent_id: None,
            sampled: None,
            debug: false,
        }
    }

    /// Returns the ID of the trace associated with this context.
    pub fn trace_id(&self) -> TraceId {
        self.trace_id
    }

    /// Returns the ID of the parent of the span associated with this context.
    pub fn parent_id(&self) -> Option<SpanId> {
        self.parent_id
    }

    /// Returns the ID of the span associated with this context.
    pub fn span_id(&self) -> SpanId {
        self.span_id
    }

    /// Determines if sampling has been requested for this context.
    ///
    /// A value of `None` indicates that the service working in the context is
    /// responsible for determining if it should be sampled.
    pub fn sampled(&self) -> Option<bool> {
        self.sampled
    }

    /// Determines if this context is in debug mode.
    ///
    /// Debug contexts should always be sampled, regardless of the value of
    /// `sampled()`.
    pub fn debug(&self) -> bool {
        self.debug
    }
}

/// A builder type for `TraceContext`s.
pub struct Builder {
    parent_id: Option<SpanId>,
    sampled: Option<bool>,
    debug: bool,
}

impl Builder {
    /// Sets the ID of the parent span of this context.
    ///
    /// Defaults to `None`.
    pub fn parent_id(&mut self, parent_id: SpanId) -> &mut Builder {
        self.parent_id = Some(parent_id);
        self
    }

    /// Sets the sampling request for this context.
    ///
    /// Defaults to `None`.
    pub fn sampled(&mut self, sampled: bool) -> &mut Builder {
        self.sampled = Some(sampled);
        self
    }

    /// Sets the debug flag for this request.
    ///
    /// Defaults to `false`.
    pub fn debug(&mut self, debug: bool) -> &mut Builder {
        self.debug = debug;
        self
    }

    /// Constructs a `TraceContext`.
    pub fn build(&self, trace_id: TraceId, span_id: SpanId) -> TraceContext {
        TraceContext {
            trace_id,
            parent_id: self.parent_id,
            span_id,
            sampled: if self.debug { Some(true) } else { self.sampled },
            debug: self.debug,
        }
    }
}
