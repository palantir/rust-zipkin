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

//! Annotations.
use std::time::SystemTime;

/// Associates an event that explains latency with a timestamp.
///
/// Unlike log statements, annotations are often codes, e.g. "ws" for WireSend.
///
/// Zipkin v1 core annotations such as "cs" and "sr" have been replaced with
/// `Span::kind`, which interprets timestamp and duration.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Annotation {
    #[cfg_attr(feature = "serde", serde(with = "::time_micros"))]
    timestamp: SystemTime,
    value: String,
}

impl Annotation {
    /// Creates a new `Annotation`.
    #[inline]
    pub fn new(timestamp: SystemTime, value: &str) -> Annotation {
        Annotation {
            timestamp,
            value: value.to_string(),
        }
    }

    /// Creates a new `Annotation` at the current time.
    #[inline]
    pub fn now(value: &str) -> Annotation {
        Annotation::new(SystemTime::now(), value)
    }

    /// Returns the time at which the annotated event occurred.
    #[inline]
    pub fn timestamp(&self) -> SystemTime {
        self.timestamp
    }

    /// Returns the value of the annotation.
    #[inline]
    pub fn value(&self) -> &str {
        &self.value
    }
}
