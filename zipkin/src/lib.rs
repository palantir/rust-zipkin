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

//! Zipkin is a library for collecting timing information about computations in
//! distributed systems. This information is collected into [Zipkin][] spans.
//!
//! This library corresponds to version 2 of the Zipkin [specification].
//!
//! # Serialization
//!
//! If the `serde` Cargo feature is enabled, `Annotation`, `Endpoint`, `Kind`, `Span`, `SpanId`, and
//! `TraceId` implement `Serialize` and `Deserialize` in the standard Zipkin format.
//!
//! [Zipkin]: http://zipkin.io/
//! [specification]: https://github.com/openzipkin/zipkin-api/blob/master/zipkin2-api.yaml
#![doc(html_root_url = "https://docs.rs/zipkin/0.3")]
#![warn(missing_docs)]

#[doc(inline)]
pub use zipkin_types::{
    annotation, endpoint, span, span_id, trace_id, Annotation, Endpoint, Kind, Span, SpanId,
    TraceId,
};

#[doc(inline)]
pub use crate::open_span::*;
#[doc(inline)]
pub use crate::report::Report;
#[doc(inline)]
pub use crate::sample::Sample;
#[doc(inline)]
pub use crate::sampling_flags::SamplingFlags;
#[doc(inline)]
pub use crate::trace_context::TraceContext;
#[doc(inline)]
pub use crate::tracer::*;

mod open_span;
pub mod report;
pub mod sample;
pub mod sampling_flags;
pub mod trace_context;
mod tracer;

#[cfg(test)]
mod test;
