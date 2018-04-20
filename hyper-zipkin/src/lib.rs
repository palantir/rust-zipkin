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
#![doc(html_root_url = "https://docs.rs/hyper-zipkin/0.3")]
#![warn(missing_docs)]
extern crate zipkin;

#[macro_use]
extern crate hyper;

use hyper::header::{Formatter, Header, Headers, Raw};
use std::fmt;
use std::ops::{Deref, DerefMut};
use zipkin::{SamplingFlags, SpanId, TraceContext, TraceId};

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
/// debug mode. It corresponds to the `debug` field of a `TraceContext`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct XB3Flags;

impl Header for XB3Flags {
    fn header_name() -> &'static str {
        "X-B3-Flags"
    }

    fn parse_header(raw: &Raw) -> hyper::Result<XB3Flags> {
        if let Some(line) = raw.one() {
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

    fn fmt_header(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.fmt_line(&"1")
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

    fn parse_header(raw: &Raw) -> hyper::Result<XB3Sampled> {
        if let Some(line) = raw.one() {
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

    fn fmt_header(&self, fmt: &mut Formatter) -> fmt::Result {
        let s = if self.0 { "1" } else { "0" };
        fmt.fmt_line(&s)
    }
}

/// Constructs `SamplingFlags` from a set of headers.
pub fn get_sampling_flags(headers: &Headers) -> SamplingFlags {
    let mut builder = SamplingFlags::builder();

    if let Some(sampled) = headers.get::<XB3Sampled>() {
        builder.sampled(sampled.0);
    }

    if let Some(&XB3Flags) = headers.get::<XB3Flags>() {
        builder.debug(true);
    }

    builder.build()
}

/// Serializes `SamplingFlags` into a set of headers.
pub fn set_sampling_flags(flags: SamplingFlags, headers: &mut Headers) {
    if flags.debug() {
        headers.set(XB3Flags);
    } else if let Some(sampled) = flags.sampled() {
        headers.set(XB3Sampled(sampled));
    }
}

/// Constructs a `TraceContext` from a set of headers.
pub fn get_trace_context(headers: &Headers) -> Option<TraceContext> {
    let trace_id = headers.get::<XB3TraceId>()?.0;
    let span_id = headers.get::<XB3SpanId>()?.0;

    let mut context = TraceContext::builder();
    context
        .trace_id(trace_id)
        .span_id(span_id)
        .sampling_flags(get_sampling_flags(headers));

    if let Some(parent_id) = headers.get::<XB3ParentSpanId>() {
        context.parent_id(parent_id.0);
    }

    Some(context.build())
}

/// Serializes a `TraceContext` into a set of headers.
pub fn set_trace_context(context: TraceContext, headers: &mut Headers) {
    headers.set(XB3TraceId(context.trace_id()));
    headers.set(XB3SpanId(context.span_id()));

    if let Some(parent_id) = context.parent_id() {
        headers.set(XB3ParentSpanId(parent_id));
    }

    set_sampling_flags(context.sampling_flags(), headers);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn flags_empty() {
        let mut headers = Headers::new();
        let flags = SamplingFlags::builder().build();
        set_sampling_flags(flags, &mut headers);

        let expected_headers = Headers::new();
        assert_eq!(headers, expected_headers);

        assert_eq!(get_sampling_flags(&headers), flags);
    }

    #[test]
    fn flags_debug() {
        let mut headers = Headers::new();
        let flags = SamplingFlags::builder().debug(true).build();
        set_sampling_flags(flags, &mut headers);

        let mut expected_headers = Headers::new();
        expected_headers.set_raw("X-B3-Flags", "1");
        assert_eq!(headers, expected_headers);

        assert_eq!(get_sampling_flags(&headers), flags);
    }

    #[test]
    fn flags_sampled() {
        let mut headers = Headers::new();
        let flags = SamplingFlags::builder().sampled(true).build();
        set_sampling_flags(flags, &mut headers);

        let mut expected_headers = Headers::new();
        expected_headers.set_raw("X-B3-Sampled", "1");
        assert_eq!(headers, expected_headers);

        assert_eq!(get_sampling_flags(&headers), flags);
    }

    #[test]
    fn flags_unsampled() {
        let mut headers = Headers::new();
        let flags = SamplingFlags::builder().sampled(false).build();
        set_sampling_flags(flags, &mut headers);

        let mut expected_headers = Headers::new();
        expected_headers.set_raw("X-B3-Sampled", "0");
        assert_eq!(headers, expected_headers);

        assert_eq!(get_sampling_flags(&headers), flags);
    }

    #[test]
    fn trace_context() {
        let mut headers = Headers::new();
        let context = TraceContext::builder()
            .trace_id([0, 1, 2, 3, 4, 5, 6, 7].into())
            .parent_id([1, 2, 3, 4, 5, 6, 7, 8].into())
            .span_id([2, 3, 4, 5, 6, 7, 8, 9].into())
            .sampled(true)
            .build();
        set_trace_context(context, &mut headers);

        let mut expected_headers = Headers::new();
        expected_headers.set_raw("X-B3-TraceId", "0001020304050607");
        expected_headers.set_raw("X-B3-SpanId", "0203040506070809");
        expected_headers.set_raw("X-B3-ParentSpanId", "0102030405060708");
        expected_headers.set_raw("X-B3-Sampled", "1");
        assert_eq!(headers, expected_headers);

        assert_eq!(get_trace_context(&headers), Some(context));
    }
}
