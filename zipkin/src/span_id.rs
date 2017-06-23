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

//! Span IDs.
use data_encoding::{HEXLOWER_PERMISSIVE, DecodeError};
#[cfg(feature = "serde")]
use serde::{Serialize, Serializer};
use std::error::Error;
use std::fmt;
use std::str::FromStr;

/// The ID of a span.
///
/// Span IDs are 8 bytes, and are serialized as hexadecimal strings.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SpanId {
    buf: [u8; 8],
}

impl FromStr for SpanId {
    type Err = SpanIdParseError;

    fn from_str(s: &str) -> Result<SpanId, SpanIdParseError> {
        let mut buf = [0; 8];
        match HEXLOWER_PERMISSIVE.decode_len(s.len()) {
            Ok(8) => {
                HEXLOWER_PERMISSIVE
                    .decode_mut(s.as_bytes(), &mut buf)
                    .map_err(|e| SpanIdParseError(Some(e)))?
            }
            _ => return Err(SpanIdParseError(None)),
        }

        Ok(SpanId { buf })
    }
}

impl fmt::Display for SpanId {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        for b in self.bytes() {
            write!(fmt, "{:02x}", b)?;
        }
        Ok(())
    }
}

#[cfg(feature = "serde")]
impl Serialize for SpanId {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.collect_str(self)
    }
}

impl SpanId {
    /// Returns the bytes of the span ID.
    pub fn bytes(&self) -> &[u8] {
        &self.buf
    }
}

impl From<[u8; 8]> for SpanId {
    fn from(bytes: [u8; 8]) -> SpanId {
        SpanId { buf: bytes }
    }
}

/// The error returned when parsing a `SpanId` from a string.
#[derive(Debug)]
pub struct SpanIdParseError(Option<DecodeError>);

impl fmt::Display for SpanIdParseError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}: ", self.description())?;
        match self.0 {
            Some(ref err) => write!(fmt, "{}", err),
            None => fmt.write_str("invalid length"),
        }
    }
}

impl Error for SpanIdParseError {
    fn description(&self) -> &str {
        "error parsing span ID"
    }

    fn cause(&self) -> Option<&Error> {
        match self.0 {
            Some(ref e) => Some(e),
            None => None,
        }
    }
}
