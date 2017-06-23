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

//! Binary annotations.
use Endpoint;

/// A `BinaryAnnotation` represents extra information about a `Span`.
///
/// It consists of a key/value pair of information, and an optional `Endpoint`
/// identifying the service in which the `BinaryAnnotation` was generated.
///
/// Zipkin defines a number of "standard" keys:
///
/// * `lc` - "Local Component": Used to identify local spans - those that do not
///     involve a remote request to another service.
///
/// Arbitrary user-defined keys can also be used.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct BinaryAnnotation {
    key: String,
    value: String,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    endpoint: Option<Endpoint>,
}

impl BinaryAnnotation {
    /// Returns a builder used to construct a `BinaryAnnotation`.
    pub fn builder() -> Builder {
        Builder { endpoint: None }
    }

    /// Returns the binary annotation's key.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Returns the binary annotation's value.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns the endpoint on which the binary annotation was generated.
    pub fn endpoint(&self) -> Option<&Endpoint> {
        self.endpoint.as_ref()
    }
}

/// A builder for `BinaryAnnotation`s.
pub struct Builder {
    endpoint: Option<Endpoint>,
}

impl Builder {
    /// Sets the endpoint associated with the binary annotation.
    ///
    /// Defaults to `None`.
    pub fn endpoint(&mut self, endpoint: Endpoint) -> &mut Builder {
        self.endpoint = Some(endpoint);
        self
    }

    /// Constructs the `BinaryAnnotation`.
    pub fn build(&mut self, key: &str, value: &str) -> BinaryAnnotation {
        BinaryAnnotation {
            key: key.to_string(),
            value: value.to_string(),
            endpoint: self.endpoint.take(),
        }
    }
}
