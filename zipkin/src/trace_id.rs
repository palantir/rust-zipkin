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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Inner {
    Short([u8; 8]),
    Long([u8; 16]),
}

/// The ID of a trace.
///
/// Trace IDs are either 8 or 16 bytes, and are serialized as hexadecimal
/// strings.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TraceId(Inner);

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
        let inner = match HEXLOWER_PERMISSIVE.decode_len(s.len()) {
            Ok(8) => {
                let mut buf = [0; 8];
                HEXLOWER_PERMISSIVE
                    .decode_mut(s.as_bytes(), &mut buf)
                    .map_err(|e| TraceIdParseError(Some(e.error)))?;
                Inner::Short(buf)
            }
            Ok(16) => {
                let mut buf = [0; 16];
                HEXLOWER_PERMISSIVE
                    .decode_mut(s.as_bytes(), &mut buf)
                    .map_err(|e| TraceIdParseError(Some(e.error)))?;
                Inner::Long(buf)
            }
            _ => return Err(TraceIdParseError(None)),
        };

        Ok(TraceId(inner))
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
        match self.0 {
            Inner::Short(ref buf) => buf,
            Inner::Long(ref buf) => buf,
        }
    }
}

impl From<[u8; 8]> for TraceId {
    fn from(bytes: [u8; 8]) -> TraceId {
        TraceId(Inner::Short(bytes))
    }
}

impl From<[u8; 16]> for TraceId {
    fn from(bytes: [u8; 16]) -> TraceId {
        TraceId(Inner::Long(bytes))
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
