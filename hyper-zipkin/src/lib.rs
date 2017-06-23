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

//! Hyper definitions for Zipkin headers.
#![warn(missing_docs)]
extern crate zipkin;

#[macro_use]
extern crate hyper;

use hyper::header::{Header, HeaderFormat, Headers};
use std::fmt;
use std::ops::{Deref, DerefMut};
use zipkin::{TraceId, SpanId, TraceContext};

header! {
    /// The `X-B3-TraceId` header.
    ///
    /// Its value is a hexadecimal-encoded 8 or 16 byte trace ID. It corresponds
    /// to the `trace_id` field of a `TraceContext`.
    #[derive(Copy)] (XB3TraceId, "X-B3-TraceId") => [TraceId]
}

header! {
    /// The `X-B3-SpanId` header.
    ///
    /// Its value is a hexadecimal-encoded 8 byte span ID. It corresponds to the
    /// `span_id` field of a `TraceContext`.
    #[derive(Copy)] (XB3SpanId, "X-B3-SpanId") => [SpanId]
}

header! {
    /// The `X-B3-ParentSpanID` header.
    ///
    /// Its value is a hexadecimal-encoded 8 byte span ID. It corresponds to the
    /// `parent_id` field of a `TraceContext`.
    #[derive(Copy)] (XB3ParentSpanId, "X-B3-ParentSpanId") => [SpanId]
}

/// The `X-B3-Flags` header.
///
/// Its value is always `1` if present, which indicates that the context is in
/// debug mode. It corresponds to the `debug` field of a `TraceContext.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct XB3Flags;

impl Header for XB3Flags {
    fn header_name() -> &'static str {
        "X-B3-Flags"
    }

    fn parse_header(raw: &[Vec<u8>]) -> hyper::Result<XB3Flags> {
        if raw.len() == 1 {
            let line = &raw[0];
            if line.len() == 1 {
                let byte = line[0];
                match byte {
                    b'1' => return Ok(XB3Flags),
                    _ => {}
                }
            }
        }
        Err(hyper::Error::Header)
    }
}

impl HeaderFormat for XB3Flags {
    fn fmt_header(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str("1")
    }
}

/// The `X-B3-Sampled` header.
///
/// Its value is either `0` or `1`, and indicates if the client has requested
/// that the context be sampled or not. It correponds to the `sampled` field of
/// a `TraceContext`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct XB3Sampled(pub bool);

impl Deref for XB3Sampled {
    type Target = bool;

    fn deref(&self) -> &bool {
        &self.0
    }
}

impl DerefMut for XB3Sampled {
    fn deref_mut(&mut self) -> &mut bool {
        &mut self.0
    }
}

impl Header for XB3Sampled {
    fn header_name() -> &'static str {
        "X-B3-Sampled"
    }

    fn parse_header(raw: &[Vec<u8>]) -> hyper::Result<XB3Sampled> {
        if raw.len() == 1 {
            let line = &raw[0];
            if line.len() == 1 {
                let byte = line[0];
                match byte {
                    b'0' => return Ok(XB3Sampled(false)),
                    b'1' => return Ok(XB3Sampled(true)),
                    _ => {}
                }
            }
        }
        Err(hyper::Error::Header)
    }
}

impl HeaderFormat for XB3Sampled {
    fn fmt_header(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        if self.0 {
            fmt.write_str("1")
        } else {
            fmt.write_str("0")
        }
    }
}

/// Constructs a `TraceContext` from a set of headers.
pub fn get_trace_context(headers: &Headers) -> Option<TraceContext> {
    let trace_id = match headers.get::<XB3TraceId>() {
        Some(trace_id) => trace_id,
        None => return None,
    };

    let span_id = match headers.get::<XB3SpanId>() {
        Some(span_id) => span_id,
        None => return None,
    };

    let mut context = TraceContext::builder();

    if let Some(parent_id) = headers.get::<XB3ParentSpanId>() {
        context.parent_id(parent_id.0);
    }

    if let Some(sampled) = headers.get::<XB3Sampled>() {
        context.sampled(sampled.0);
    }

    if let Some(&XB3Flags) = headers.get::<XB3Flags>() {
        context.debug(true);
    }

    Some(context.build(trace_id.0, span_id.0))
}

/// Serializes a `TraceContext` into a set of headers.
pub fn set_trace_context(context: TraceContext, headers: &mut Headers) {
    headers.set(XB3TraceId(context.trace_id()));
    headers.set(XB3SpanId(context.span_id()));

    if let Some(parent_id) = context.parent_id() {
        headers.set(XB3ParentSpanId(parent_id));
    }

    if context.debug() {
        headers.set(XB3Flags);
    } else if let Some(sampled) = context.sampled() {
        headers.set(XB3Sampled(sampled));
    }
}
