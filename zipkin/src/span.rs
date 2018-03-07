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
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use {Annotation, Endpoint, SpanId, TraceId};

/// The "kind" of a span.
///
/// This has an impact on the relationship between the span's timestamp, duration, and local
/// endpoint.
#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
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

    #[doc(hidden)]
    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    __NonExhaustive,
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
#[cfg_attr(feature = "serde", derive(Serialize))]
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
    #[cfg_attr(feature = "serde",
               serde(skip_serializing_if = "Option::is_none",
                     serialize_with = "::opt_time_micros"))]
    timestamp: Option<SystemTime>,
    #[cfg_attr(feature = "serde",
               serde(skip_serializing_if = "Option::is_none",
                     serialize_with = "::opt_duration_micros"))]
    duration: Option<Duration>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "is_false"))]
    debug: bool,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "is_false"))]
    shared: bool,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    local_endpoint: Option<Endpoint>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    remote_endpoint: Option<Endpoint>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Vec::is_empty"))]
    annotations: Vec<Annotation>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "HashMap::is_empty"))]
    tags: HashMap<String, String>,
}

#[cfg(feature = "serde")]
fn is_false(v: &bool) -> bool {
    !*v
}

impl Span {
    /// Returns a builder used to construct a `Span`.
    pub fn builder() -> Builder {
        Builder {
            name: None,
            parent_id: None,
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
    pub fn trace_id(&self) -> TraceId {
        self.trace_id
    }

    /// The logical operation this span represents (e.g. an RPC method).
    ///
    /// Leave absent if unknown.
    ///
    /// These are lookup labels, so take care to ensure names are low cardinality. For example, do
    /// not embed variables into the name.
    pub fn name(&self) -> Option<&str> {
        self.name.as_ref().map(|s| &**s)
    }

    /// The parent span ID, or `None` if this is the root span in a trace.
    pub fn parent_id(&self) -> Option<SpanId> {
        self.parent_id
    }

    /// The unique 64 bit identifier for this operation within the trace.
    pub fn id(&self) -> SpanId {
        self.id
    }

    /// The "kind" of operation this span represents.
    ///
    /// When absent, the span is absent or incomplete.
    pub fn kind(&self) -> Option<Kind> {
        self.kind
    }

    /// The start of the span.
    pub fn timestamp(&self) -> Option<SystemTime> {
        self.timestamp
    }

    /// The duration of the critical path, if known.
    ///
    /// Durations are recorded in microseconds, and rounded up to a minimum of 1. Durations of
    /// children can be longer than their parents due to asynchronous operations.
    pub fn duration(&self) -> Option<Duration> {
        self.duration
    }

    /// Determines if this span is part of a normal or forcibly sampled span.
    ///
    /// If true, the span should always be sampled regardless of the sampling configuration.
    pub fn debug(&self) -> bool {
        self.debug
    }

    /// Determines if this span was started by another tracer (e.g. on a different host).
    pub fn shared(&self) -> bool {
        self.shared
    }

    /// Returns the host that recorded this span, primarily for query by service name.
    ///
    /// Instrumentation should always record this. The IP address is usually the site local or
    /// advertised service address. When present, the port indicates the listen port.
    pub fn local_endpoint(&self) -> Option<&Endpoint> {
        self.local_endpoint.as_ref()
    }

    /// Returns the other side of the connection for RPC or messaging spans.
    pub fn remote_endpoint(&self) -> Option<&Endpoint> {
        self.remote_endpoint.as_ref()
    }

    /// Returns the annotations associated with this span.
    pub fn annotations(&self) -> &[Annotation] {
        &self.annotations
    }

    /// Returns tags used to give spans context for search, viewing, and analysis.
    pub fn tags(&self) -> &HashMap<String, String> {
        &self.tags
    }
}

/// A builder for `Span`s.
pub struct Builder {
    name: Option<String>,
    parent_id: Option<SpanId>,
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

impl Builder {
    /// Sets the name of the span.
    ///
    /// Defaults to `None`.
    pub fn name(&mut self, name: &str) -> &mut Builder {
        self.name = Some(name.to_lowercase());
        self
    }

    /// Sets the ID of the span's parent.
    ///
    /// Defaults to `None`.
    pub fn parent_id(&mut self, parent_id: SpanId) -> &mut Builder {
        self.parent_id = Some(parent_id);
        self
    }

    /// Sets the kind of the span.
    ///
    /// Defaults to `None`.
    pub fn kind(&mut self, kind: Kind) -> &mut Builder {
        self.kind = Some(kind);
        self
    }

    /// Sets the time of the beginning of the span.
    ///
    /// Defaults to `None`.
    pub fn timestamp(&mut self, timestamp: SystemTime) -> &mut Builder {
        self.timestamp = Some(timestamp);
        self
    }

    /// Sets the duration of the span.
    ///
    /// Defaults to `None`.
    pub fn duration(&mut self, duration: Duration) -> &mut Builder {
        self.duration = Some(duration);
        self
    }

    /// Sets the debug state of the span.
    ///
    /// Defaults to `false`.
    pub fn debug(&mut self, debug: bool) -> &mut Builder {
        self.debug = debug;
        self
    }

    /// Sets the shared state of the span.
    ///
    /// Defaults to `false`.
    pub fn shared(&mut self, shared: bool) -> &mut Builder {
        self.shared = shared;
        self
    }

    /// Sets the local endpoint of the span.
    ///
    /// Defaults to `None`.
    pub fn local_endpoint(&mut self, local_endpoint: Endpoint) -> &mut Builder {
        self.local_endpoint = Some(local_endpoint);
        self
    }

    /// Sets the remote endpoint of the span.
    ///
    /// Defaults to `None`.
    pub fn remote_endpoint(&mut self, remote_endpoint: Endpoint) -> &mut Builder {
        self.remote_endpoint = Some(remote_endpoint);
        self
    }

    /// Adds an annotation to the span.
    pub fn annotation(&mut self, annotation: Annotation) -> &mut Builder {
        self.annotations.push(annotation);
        self
    }

    /// Adds multiple annotations to the span.
    pub fn annotations<I>(&mut self, annotations: I) -> &mut Builder
    where
        I: IntoIterator<Item = Annotation>,
    {
        self.annotations.extend(annotations);
        self
    }

    /// Adds a tag to the span.
    pub fn tag(&mut self, key: &str, value: &str) -> &mut Builder {
        self.tags.insert(key.to_string(), value.to_string());
        self
    }

    /// As multiple tags to the span.
    pub fn tags<I>(&mut self, tags: I) -> &mut Builder
    where
        I: IntoIterator<Item = (String, String)>,
    {
        self.tags.extend(tags);
        self
    }

    /// Constructs a `Span`.
    pub fn build(&self, trace_id: TraceId, id: SpanId) -> Span {
        Span {
            trace_id,
            name: self.name.clone(),
            id,
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
