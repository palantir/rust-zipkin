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

//! Spans.
use crate::{Annotation, Endpoint, SpanId, TraceId};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// The "kind" of a span.
///
/// This has an impact on the relationship between the span's timestamp, duration, and local
/// endpoint.
#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[non_exhaustive]
pub enum Kind {
    /// The client side of an RPC.
    ///
    /// * Timestamp - The moment a request was sent (formerly "cs")
    /// * Duration - When present, indicates when a response was received (formerly "cr")
    /// * Remote Endpoint - Represents the server.
    Client,

    /// The server side of an RPC.
    ///
    /// * Timestamp - The moment a request was received (formerly "sr")
    /// * Duration - When present, indicates when a response was received (formerly "ss")
    /// * Remote Endpoint - Represents the client.
    Server,

    /// A message sent to a message broker.
    ///
    /// * Timestamp - The moment a message was sent to a destination (formerly "ms")
    /// * Duration - When present, represents the delay sending the message, such as batching.
    /// * Remote Endpoint - Represents the broker.
    Producer,

    /// A message received from a message broker.
    ///
    /// * Timestamp - The moment a message was received from an origin (formerly "mr")
    /// * Duration - When present, represents the delay consuming the message, such as from a
    ///     backlog.
    /// * Remote Endpoint - Represents the broker.
    Consumer,
}

/// A `Span` represents a single operation over some range of time.
///
/// Multiple spans make up a single "trace" of a distributed computation, and
/// spans can be nested. A new trace is created with a "root" span, and
/// subsections of that computation are recorded in individual spans.
///
/// For spans tracing a remote service call, two records will typically be
/// generated, one from the client and the other from the server. The client is
/// responsible for recording the timestamp and duration associated with the
/// span, and the server span should omit that information. The client and
/// server may both add their own annotations and binary annotations the span -
/// they will be merged.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Span {
    trace_id: TraceId,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    name: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    parent_id: Option<SpanId>,
    id: SpanId,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    kind: Option<Kind>,
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "Option::is_none",
            with = "crate::opt_time_micros"
        )
    )]
    timestamp: Option<SystemTime>,
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "Option::is_none",
            with = "crate::opt_duration_micros"
        )
    )]
    duration: Option<Duration>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "is_false", default = "value_false")
    )]
    debug: bool,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "is_false", default = "value_false")
    )]
    shared: bool,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    local_endpoint: Option<Endpoint>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    remote_endpoint: Option<Endpoint>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Vec::is_empty", default)
    )]
    annotations: Vec<Annotation>,
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "HashMap::is_empty", default)
    )]
    tags: HashMap<String, String>,
}

#[cfg(feature = "serde")]
#[inline]
fn is_false(v: &bool) -> bool {
    !*v
}

#[cfg(feature = "serde")]
#[inline]
fn value_false() -> bool {
    false
}

impl Span {
    /// Returns a builder used to construct a `Span`.
    #[inline]
    pub fn builder() -> Builder {
        Builder {
            trace_id: None,
            name: None,
            parent_id: None,
            id: None,
            kind: None,
            timestamp: None,
            duration: None,
            debug: false,
            shared: false,
            local_endpoint: None,
            remote_endpoint: None,
            annotations: vec![],
            tags: HashMap::new(),
        }
    }

    /// The randomly generated, unique identifier for a trace, set on all spans within it.
    #[inline]
    pub fn trace_id(&self) -> TraceId {
        self.trace_id
    }

    /// The logical operation this span represents (e.g. an RPC method).
    ///
    /// Leave absent if unknown.
    ///
    /// These are lookup labels, so take care to ensure names are low cardinality. For example, do
    /// not embed variables into the name.
    #[inline]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The parent span ID, or `None` if this is the root span in a trace.
    #[inline]
    pub fn parent_id(&self) -> Option<SpanId> {
        self.parent_id
    }

    /// The unique 64 bit identifier for this operation within the trace.
    #[inline]
    pub fn id(&self) -> SpanId {
        self.id
    }

    /// The "kind" of operation this span represents.
    ///
    /// When absent, the span is local or incomplete.
    #[inline]
    pub fn kind(&self) -> Option<Kind> {
        self.kind
    }

    /// The start of the span.
    #[inline]
    pub fn timestamp(&self) -> Option<SystemTime> {
        self.timestamp
    }

    /// The duration of the critical path, if known.
    ///
    /// Durations are recorded in microseconds, and rounded up to a minimum of 1. Durations of
    /// children can be longer than their parents due to asynchronous operations.
    #[inline]
    pub fn duration(&self) -> Option<Duration> {
        self.duration
    }

    /// Determines if this span is part of a normal or forcibly sampled span.
    ///
    /// If true, the span should always be sampled regardless of the sampling configuration.
    #[inline]
    pub fn debug(&self) -> bool {
        self.debug
    }

    /// Determines if this span was started by another tracer (e.g. on a different host).
    #[inline]
    pub fn shared(&self) -> bool {
        self.shared
    }

    /// Returns the host that recorded this span, primarily for query by service name.
    ///
    /// Instrumentation should always record this. The IP address is usually the site local or
    /// advertised service address. When present, the port indicates the listen port.
    #[inline]
    pub fn local_endpoint(&self) -> Option<&Endpoint> {
        self.local_endpoint.as_ref()
    }

    /// Returns the other side of the connection for RPC or messaging spans.
    #[inline]
    pub fn remote_endpoint(&self) -> Option<&Endpoint> {
        self.remote_endpoint.as_ref()
    }

    /// Returns the annotations associated with this span.
    #[inline]
    pub fn annotations(&self) -> &[Annotation] {
        &self.annotations
    }

    /// Returns tags used to give spans context for search, viewing, and analysis.
    #[inline]
    pub fn tags(&self) -> &HashMap<String, String> {
        &self.tags
    }
}

/// A builder for `Span`s.
pub struct Builder {
    trace_id: Option<TraceId>,
    name: Option<String>,
    parent_id: Option<SpanId>,
    id: Option<SpanId>,
    kind: Option<Kind>,
    timestamp: Option<SystemTime>,
    duration: Option<Duration>,
    debug: bool,
    shared: bool,
    local_endpoint: Option<Endpoint>,
    remote_endpoint: Option<Endpoint>,
    annotations: Vec<Annotation>,
    tags: HashMap<String, String>,
}

impl From<Span> for Builder {
    #[inline]
    fn from(s: Span) -> Builder {
        Builder {
            trace_id: Some(s.trace_id),
            name: s.name,
            parent_id: s.parent_id,
            id: Some(s.id),
            kind: s.kind,
            timestamp: s.timestamp,
            duration: s.duration,
            debug: s.debug,
            shared: s.shared,
            local_endpoint: s.local_endpoint,
            remote_endpoint: s.remote_endpoint,
            annotations: s.annotations,
            tags: s.tags,
        }
    }
}

impl Builder {
    /// Sets the trace ID of the span.
    #[inline]
    pub fn trace_id(&mut self, trace_id: TraceId) -> &mut Builder {
        self.trace_id = Some(trace_id);
        self
    }

    /// Sets the name of the span.
    ///
    /// Defaults to `None`.
    #[inline]
    pub fn name(&mut self, name: &str) -> &mut Builder {
        self.name = Some(name.to_lowercase());
        self
    }

    /// Sets the ID of the span's parent.
    ///
    /// Defaults to `None`.
    #[inline]
    pub fn parent_id(&mut self, parent_id: SpanId) -> &mut Builder {
        self.parent_id = Some(parent_id);
        self
    }

    /// Sets the ID of the span.
    #[inline]
    pub fn id(&mut self, id: SpanId) -> &mut Builder {
        self.id = Some(id);
        self
    }

    /// Sets the kind of the span.
    ///
    /// Defaults to `None`.
    #[inline]
    pub fn kind(&mut self, kind: Kind) -> &mut Builder {
        self.kind = Some(kind);
        self
    }

    /// Sets the time of the beginning of the span.
    ///
    /// Defaults to `None`.
    #[inline]
    pub fn timestamp(&mut self, timestamp: SystemTime) -> &mut Builder {
        self.timestamp = Some(timestamp);
        self
    }

    /// Sets the duration of the span.
    ///
    /// Defaults to `None`.
    #[inline]
    pub fn duration(&mut self, duration: Duration) -> &mut Builder {
        self.duration = Some(duration);
        self
    }

    /// Sets the debug state of the span.
    ///
    /// Defaults to `false`.
    #[inline]
    pub fn debug(&mut self, debug: bool) -> &mut Builder {
        self.debug = debug;
        self
    }

    /// Sets the shared state of the span.
    ///
    /// Defaults to `false`.
    #[inline]
    pub fn shared(&mut self, shared: bool) -> &mut Builder {
        self.shared = shared;
        self
    }

    /// Sets the local endpoint of the span.
    ///
    /// Defaults to `None`.
    #[inline]
    pub fn local_endpoint(&mut self, local_endpoint: Endpoint) -> &mut Builder {
        self.local_endpoint = Some(local_endpoint);
        self
    }

    /// Sets the remote endpoint of the span.
    ///
    /// Defaults to `None`.
    #[inline]
    pub fn remote_endpoint(&mut self, remote_endpoint: Endpoint) -> &mut Builder {
        self.remote_endpoint = Some(remote_endpoint);
        self
    }

    /// Adds an annotation to the span.
    #[inline]
    pub fn annotation(&mut self, annotation: Annotation) -> &mut Builder {
        self.annotations.push(annotation);
        self
    }

    /// Adds multiple annotations to the span.
    #[inline]
    pub fn annotations<I>(&mut self, annotations: I) -> &mut Builder
    where
        I: IntoIterator<Item = Annotation>,
    {
        self.annotations.extend(annotations);
        self
    }

    /// Adds a tag to the span.
    #[inline]
    pub fn tag(&mut self, key: &str, value: &str) -> &mut Builder {
        self.tags.insert(key.to_string(), value.to_string());
        self
    }

    /// As multiple tags to the span.
    #[inline]
    pub fn tags<I>(&mut self, tags: I) -> &mut Builder
    where
        I: IntoIterator<Item = (String, String)>,
    {
        self.tags.extend(tags);
        self
    }

    /// Constructs a `Span`.
    ///
    /// # Panics
    ///
    /// Panics if `trace_id` or `id` was not set.
    #[inline]
    pub fn build(&self) -> Span {
        Span {
            trace_id: self.trace_id.expect("trace ID not set"),
            name: self.name.clone(),
            id: self.id.expect("span ID not set"),
            kind: self.kind,
            parent_id: self.parent_id,
            timestamp: self.timestamp,
            duration: self.duration,
            debug: self.debug,
            shared: self.shared,
            local_endpoint: self.local_endpoint.clone(),
            remote_endpoint: self.remote_endpoint.clone(),
            annotations: self.annotations.clone(),
            tags: self.tags.clone(),
        }
    }
}
