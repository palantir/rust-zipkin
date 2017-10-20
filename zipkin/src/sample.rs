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

//! Span samplers.
use rand;

use TraceId;

/// A sampler decides whether or not a span should be recorded based on its
/// trace ID.
///
/// A trace context received from a remote service may already indicate if the
/// span should be recorded, but if it does not, a `Sample`r is responsible for
/// making that decision.
pub trait Sample {
    /// Returns `true` if the span associated with the trace ID should be
    /// recorded.
    fn sample(&self, trace_id: TraceId) -> bool;
}

/// A `Sample`r which always returns `true`.
pub struct AlwaysSampler;

impl Sample for AlwaysSampler {
    fn sample(&self, _: TraceId) -> bool {
        true
    }
}

/// A `Sample`r which always returns `false`.
pub struct NeverSampler;

impl Sample for NeverSampler {
    fn sample(&self, _: TraceId) -> bool {
        false
    }
}

/// A `Sample`r which randomly samples at a specific rate.
pub struct RandomSampler {
    rate: f32,
}

impl RandomSampler {
    /// Creates a new `RandomSampler` at the specified rate.
    ///
    /// # Panics
    ///
    /// Panics if `rate` is less than 0 or greater than 1.
    pub fn new(rate: f32) -> RandomSampler {
        assert!(rate >= 0. && rate <= 1.);
        RandomSampler { rate }
    }
}

impl Sample for RandomSampler {
    fn sample(&self, _: TraceId) -> bool {
        rand::random::<f32>() > self.rate
    }
}
