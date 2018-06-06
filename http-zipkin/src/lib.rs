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

//! HTTP header propagation for Zipkin trace information.
#![doc(html_root_url = "https://docs.rs/http-zipkin/0.1")]
#![warn(missing_docs)]

extern crate http;
extern crate zipkin;

use http::header::{HeaderMap, HeaderValue};
use std::str::FromStr;
use zipkin::{SamplingFlags, TraceContext};

const X_B3_SAMPLED: &'static str = "X-B3-Sampled";
const X_B3_FLAGS: &'static str = "X-B3-Flags";
const X_B3_TRACEID: &'static str = "X-B3-TraceId";
const X_B3_PARENTSPANID: &'static str = "X-B3-ParentSpanId";
const X_B3_SPANID: &'static str = "X-B3-SpanId";

/// Serializes sampling flags into a set of HTTP headers.
pub fn set_sampling_flags(flags: SamplingFlags, headers: &mut HeaderMap) {
    if flags.debug() {
        headers.insert(X_B3_FLAGS, HeaderValue::from_static("1"));
        headers.remove(X_B3_SAMPLED);
    } else {
        headers.remove(X_B3_FLAGS);
        match flags.sampled() {
            Some(true) => {
                headers.insert(X_B3_SAMPLED, HeaderValue::from_static("1"));
            }
            Some(false) => {
                headers.insert(X_B3_SAMPLED, HeaderValue::from_static("0"));
            }
            None => {
                headers.remove(X_B3_SAMPLED);
            }
        }
    }
}

/// Deserializes sampling flags from a set of HTTP headers.
pub fn get_sampling_flags(headers: &HeaderMap) -> SamplingFlags {
    let mut builder = SamplingFlags::builder();

    if let Some(flags) = headers.get(X_B3_FLAGS) {
        if flags == "1" {
            builder.debug(true);
        }
    } else if let Some(sampled) = headers.get(X_B3_SAMPLED) {
        if sampled == "1" {
            builder.sampled(true);
        } else if sampled == "0" {
            builder.sampled(false);
        }
    }

    builder.build()
}

/// Serializes a trace context into a set of HTTP headers.
pub fn set_trace_context(context: TraceContext, headers: &mut HeaderMap) {
    set_sampling_flags(context.sampling_flags(), headers);

    headers.insert(
        X_B3_TRACEID,
        HeaderValue::from_str(&context.trace_id().to_string()).unwrap(),
    );
    match context.parent_id() {
        Some(parent_id) => {
            headers.insert(
                X_B3_PARENTSPANID,
                HeaderValue::from_str(&parent_id.to_string()).unwrap(),
            );
        }
        None => {
            headers.remove(X_B3_PARENTSPANID);
        }
    }
    headers.insert(
        X_B3_SPANID,
        HeaderValue::from_str(&context.span_id().to_string()).unwrap(),
    );
}

/// Deserializes a trace context from a set of HTTP headers.
pub fn get_trace_context(headers: &HeaderMap) -> Option<TraceContext> {
    let trace_id = parse_header(headers, X_B3_TRACEID)?;
    let span_id = parse_header(headers, X_B3_SPANID)?;

    let mut builder = TraceContext::builder();
    builder
        .trace_id(trace_id)
        .span_id(span_id)
        .sampling_flags(get_sampling_flags(headers));

    if let Some(parent_id) = parse_header(headers, X_B3_PARENTSPANID) {
        builder.parent_id(parent_id);
    }

    Some(builder.build())
}

fn parse_header<T>(headers: &HeaderMap, name: &str) -> Option<T>
where
    T: FromStr,
{
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn flags_empty() {
        let mut headers = HeaderMap::new();
        let flags = SamplingFlags::builder().build();
        set_sampling_flags(flags, &mut headers);

        let expected_headers = HeaderMap::new();
        assert_eq!(headers, expected_headers);

        assert_eq!(get_sampling_flags(&headers), flags);
    }

    #[test]
    fn flags_debug() {
        let mut headers = HeaderMap::new();
        let flags = SamplingFlags::builder().debug(true).build();
        set_sampling_flags(flags, &mut headers);

        let mut expected_headers = HeaderMap::new();
        expected_headers.insert("X-B3-Flags", HeaderValue::from_static("1"));
        assert_eq!(headers, expected_headers);

        assert_eq!(get_sampling_flags(&headers), flags);
    }

    #[test]
    fn flags_sampled() {
        let mut headers = HeaderMap::new();
        let flags = SamplingFlags::builder().sampled(true).build();
        set_sampling_flags(flags, &mut headers);

        let mut expected_headers = HeaderMap::new();
        expected_headers.insert("X-B3-Sampled", HeaderValue::from_static("1"));
        assert_eq!(headers, expected_headers);

        assert_eq!(get_sampling_flags(&headers), flags);
    }

    #[test]
    fn flags_unsampled() {
        let mut headers = HeaderMap::new();
        let flags = SamplingFlags::builder().sampled(false).build();
        set_sampling_flags(flags, &mut headers);

        let mut expected_headers = HeaderMap::new();
        expected_headers.insert("X-B3-Sampled", HeaderValue::from_static("0"));
        assert_eq!(headers, expected_headers);

        assert_eq!(get_sampling_flags(&headers), flags);
    }

    #[test]
    fn trace_context() {
        let mut headers = HeaderMap::new();
        let context = TraceContext::builder()
            .trace_id([0, 1, 2, 3, 4, 5, 6, 7].into())
            .parent_id([1, 2, 3, 4, 5, 6, 7, 8].into())
            .span_id([2, 3, 4, 5, 6, 7, 8, 9].into())
            .sampled(true)
            .build();
        set_trace_context(context, &mut headers);

        let mut expected_headers = HeaderMap::new();
        expected_headers.insert("X-B3-TraceId", HeaderValue::from_static("0001020304050607"));
        expected_headers.insert("X-B3-SpanId", HeaderValue::from_static("0203040506070809"));
        expected_headers.insert(
            "X-B3-ParentSpanId",
            HeaderValue::from_static("0102030405060708"),
        );
        expected_headers.insert("X-B3-Sampled", HeaderValue::from_static("1"));
        assert_eq!(headers, expected_headers);

        assert_eq!(get_trace_context(&headers), Some(context));
    }
}
