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

//! Trace IDs.
use data_encoding::{HEXLOWER_PERMISSIVE, DecodeError};
#[cfg(feature = "serde")]
use serde::{Serialize, Serializer};
use std::error::Error;
use std::fmt;
use std::str::FromStr;

/// The ID of a trace.
///
/// Span IDs are either 8 or 16 bytes, and are serialized as hexadecimal
/// strings.
// NB Eq impls are only derivable because non-extended values have zeroed buffer tails
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TraceId {
    buf: [u8; 16],
    extended: bool,
}

impl fmt::Display for TraceId {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        for b in self.bytes() {
            write!(fmt, "{:02x}", b)?;
        }
        Ok(())
    }
}

impl FromStr for TraceId {
    type Err = TraceIdParseError;

    fn from_str(s: &str) -> Result<TraceId, TraceIdParseError> {
        let mut buf = [0; 16];
        let extended = match HEXLOWER_PERMISSIVE.decode_len(s.len()) {
            Ok(8) => {
                HEXLOWER_PERMISSIVE
                    .decode_mut(s.as_bytes(), &mut buf[..8])
                    .map_err(|e| TraceIdParseError(Some(e)))?;
                false
            }
            Ok(16) => {
                HEXLOWER_PERMISSIVE
                    .decode_mut(s.as_bytes(), &mut buf)
                    .map_err(|e| TraceIdParseError(Some(e)))?;
                true
            }
            _ => return Err(TraceIdParseError(None)),
        };

        Ok(TraceId { buf, extended })
    }
}

#[cfg(feature = "serde")]
impl Serialize for TraceId {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.collect_str(self)
    }
}

impl TraceId {
    /// Returns the byte representation of the trace ID.
    pub fn bytes(&self) -> &[u8] {
        if self.extended {
            &self.buf
        } else {
            &self.buf[..8]
        }
    }
}

impl From<[u8; 8]> for TraceId {
    fn from(bytes: [u8; 8]) -> TraceId {
        let mut buf = [0; 16];
        buf[..8].copy_from_slice(&bytes);
        TraceId {
            buf,
            extended: false,
        }
    }
}

impl From<[u8; 16]> for TraceId {
    fn from(bytes: [u8; 16]) -> TraceId {
        TraceId {
            buf: bytes,
            extended: true,
        }
    }
}

/// The error returned when parsing a `TraceId` from a string.
#[derive(Debug)]
pub struct TraceIdParseError(Option<DecodeError>);

impl fmt::Display for TraceIdParseError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}: ", self.description())?;
        match self.0 {
            Some(ref err) => write!(fmt, "{}", err),
            None => fmt.write_str("invalid length"),
        }
    }
}

impl Error for TraceIdParseError {
    fn description(&self) -> &str {
        "error parsing trace ID"
    }

    fn cause(&self) -> Option<&Error> {
        match self.0 {
            Some(ref e) => Some(e),
            None => None,
        }
    }
}
