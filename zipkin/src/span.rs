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
use std::time::{SystemTime, Duration};
use std::mem;

use {TraceId, SpanId, Annotation, BinaryAnnotation};

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
    name: String,
    id: SpanId,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    parent_id: Option<SpanId>,
    #[cfg_attr(feature = "serde",
               serde(skip_serializing_if = "Option::is_none",
                       serialize_with = "::opt_time_micros"))]
    timestamp: Option<SystemTime>,
    #[cfg_attr(feature = "serde",
               serde(skip_serializing_if = "Option::is_none",
                       serialize_with = "::opt_duration_micros"))]
    duration: Option<Duration>,
    annotations: Vec<Annotation>,
    binary_annotations: Vec<BinaryAnnotation>,
}

impl Span {
    /// Returns a builder used to construct a `Span`.
    pub fn builder() -> Builder {
        Builder {
            parent_id: None,
            timestamp: None,
            duration: None,
            annotations: vec![],
            binary_annotations: vec![],
        }
    }

    /// Returns the trace ID associated with this span.
    pub fn trace_id(&self) -> TraceId {
        self.trace_id
    }

    /// Returns the name of this span.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the ID of this span.
    pub fn id(&self) -> SpanId {
        self.id
    }

    /// Returns the ID of the parent of this span, if one exists.
    pub fn parent_id(&self) -> Option<SpanId> {
        self.parent_id
    }

    /// Returns the time of the beginning of this span, if known.
    pub fn timestamp(&self) -> Option<SystemTime> {
        self.timestamp
    }

    /// Returns the duration of this span, if known.
    pub fn duration(&self) -> Option<Duration> {
        self.duration
    }

    /// Returns the annotations associated with this span.
    pub fn annotations(&self) -> &[Annotation] {
        &self.annotations
    }

    /// Returns the binary annotations associated with this span.
    pub fn binary_annotations(&self) -> &[BinaryAnnotation] {
        &self.binary_annotations
    }
}

/// A builder for `Span`s.
pub struct Builder {
    parent_id: Option<SpanId>,
    timestamp: Option<SystemTime>,
    duration: Option<Duration>,
    annotations: Vec<Annotation>,
    binary_annotations: Vec<BinaryAnnotation>,
}

impl Builder {
    /// Sets the ID of the span's parent.
    ///
    /// Defaults to `None`.
    pub fn parent_id(&mut self, parent_id: SpanId) -> &mut Builder {
        self.parent_id = Some(parent_id);
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

    /// Adds a binary annotation to the span.
    pub fn binary_annotation(&mut self, binary_annotation: BinaryAnnotation) -> &mut Builder {
        self.binary_annotations.push(binary_annotation);
        self
    }

    /// Adds multiple binary annotations to the span.
    pub fn binary_annotations<I>(&mut self, binary_annotations: I) -> &mut Builder
    where
        I: IntoIterator<Item = BinaryAnnotation>,
    {
        self.binary_annotations.extend(binary_annotations);
        self
    }

    /// Constructs a `Span`.
    pub fn build(&mut self, trace_id: TraceId, name: String, id: SpanId) -> Span {
        Span {
            trace_id,
            name,
            id,
            parent_id: self.parent_id.take(),
            timestamp: self.timestamp.take(),
            duration: self.duration.take(),
            annotations: mem::replace(&mut self.annotations, vec![]),
            binary_annotations: mem::replace(&mut self.binary_annotations, vec![]),
        }
    }
}
