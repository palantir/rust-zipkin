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

//! Sampling flags.

/// Flags used to control sampling.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SamplingFlags {
    sampled: Option<bool>,
    debug: bool,
}

impl Default for SamplingFlags {
    fn default() -> SamplingFlags {
        SamplingFlags::builder().build()
    }
}

impl SamplingFlags {
    /// Returns a builder used to construct `SamplingFlags`.
    pub fn builder() -> Builder {
        Builder {
            sampled: None,
            debug: false,
        }
    }

    /// Determines if sampling has been requested for this context.
    ///
    /// A value of `None` indicates that the service working in the context is
    /// responsible for determining if it should be sampled.
    pub fn sampled(&self) -> Option<bool> {
        self.sampled
    }

    /// Determines if this context is in debug mode.
    ///
    /// Debug contexts should always be sampled, regardless of the value of
    /// `sampled()`.
    pub fn debug(&self) -> bool {
        self.debug
    }
}

/// A builder type for `SamplingFlags`.
pub struct Builder {
    sampled: Option<bool>,
    debug: bool,
}

impl From<SamplingFlags> for Builder {
    fn from(flags: SamplingFlags) -> Builder {
        Builder {
            sampled: flags.sampled,
            debug: flags.debug,
        }
    }
}

impl Builder {
    /// Sets the sampling request for this context.
    ///
    /// Defaults to `None`.
    pub fn sampled(&mut self, sampled: bool) -> &mut Builder {
        self.sampled = Some(sampled);
        self
    }

    /// Sets the debug flag for this request.
    ///
    /// Defaults to `false`.
    pub fn debug(&mut self, debug: bool) -> &mut Builder {
        self.debug = debug;
        self
    }

    /// Constructs `SamplingFlags`.
    pub fn build(&self) -> SamplingFlags {
        SamplingFlags {
            sampled: if self.debug {
                Some(true)
            } else {
                self.sampled
            },
            debug: self.debug,
        }
    }
}
