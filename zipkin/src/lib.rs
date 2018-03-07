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
//! # Serialization
//!
//! If the `serde` Cargo feature is enabled, `Annotation`, `BinaryAnnotation`,
//! `Endpoint`, `SpanId`, and `TraceId` implement `Serialize` in the standard
//! Zipkin format.
//!
//! [Zipkin]: http://zipkin.io/
#![doc(html_root_url = "https://docs.rs/zipkin/0.1")]
#![warn(missing_docs)]

extern crate data_encoding;
extern crate rand;
extern crate thread_local_object;

#[macro_use]
extern crate log;

#[macro_use]
#[cfg(feature = "serde")]
extern crate serde;

#[cfg(test)]
extern crate antidote;

#[cfg(feature = "serde")]
use serde::{Serialize, Serializer};
#[cfg(feature = "serde")]
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[doc(inline)]
pub use annotation::Annotation;
#[doc(inline)]
pub use endpoint::Endpoint;
#[doc(inline)]
pub use report::Report;
#[doc(inline)]
pub use sample::Sample;
#[doc(inline)]
pub use span::{Kind, Span};
#[doc(inline)]
pub use span_id::SpanId;
#[doc(inline)]
pub use trace_context::TraceContext;
#[doc(inline)]
pub use trace_id::TraceId;
#[doc(inline)]
pub use tracer::{OpenSpan, Tracer};

pub mod annotation;
pub mod endpoint;
pub mod report;
pub mod sample;
pub mod span;
pub mod span_id;
pub mod trace_context;
pub mod trace_id;
pub mod tracer;

#[cfg(test)]
mod test;

#[cfg(feature = "serde")]
fn time_micros<S>(time: &SystemTime, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    duration_micros(
        &time.duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0)),
        s,
    )
}

#[cfg(feature = "serde")]
fn duration_micros<S>(duration: &Duration, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let micros = duration.as_secs() * 1_000_000 + duration.subsec_nanos() as u64 / 1_000;
    micros.serialize(s)
}

#[cfg(feature = "serde")]
fn opt_time_micros<S>(time: &Option<SystemTime>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match *time {
        Some(ref time) => time_micros(time, s),
        None => s.serialize_none(),
    }
}

#[cfg(feature = "serde")]
fn opt_duration_micros<S>(duration: &Option<Duration>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match *duration {
        Some(ref duration) => duration_micros(duration, s),
        None => s.serialize_none(),
    }
}
