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
use data_encoding::{DecodeError, HEXLOWER_PERMISSIVE};
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
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
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
mod serde {
    use crate::trace_id::TraceId;
    use serde::de::{Error, Unexpected, Visitor};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::fmt;

    impl Serialize for TraceId {
        fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            s.collect_str(self)
        }
    }

    impl<'de> Deserialize<'de> for TraceId {
        fn deserialize<D>(d: D) -> Result<TraceId, D::Error>
        where
            D: Deserializer<'de>,
        {
            d.deserialize_str(V)
        }
    }

    struct V;

    impl<'de> Visitor<'de> for V {
        type Value = TraceId;

        fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
            fmt.write_str("a hex-encoded trace ID")
        }

        fn visit_str<E>(self, v: &str) -> Result<TraceId, E>
        where
            E: Error,
        {
            v.parse()
                .map_err(|_| Error::invalid_value(Unexpected::Str(v), &self))
        }
    }
}

impl TraceId {
    /// Returns the byte representation of the trace ID.
    #[inline]
    pub fn bytes(&self) -> &[u8] {
        match self.0 {
            Inner::Short(ref buf) => buf,
            Inner::Long(ref buf) => buf,
        }
    }
}

impl From<[u8; 8]> for TraceId {
    #[inline]
    fn from(bytes: [u8; 8]) -> TraceId {
        TraceId(Inner::Short(bytes))
    }
}

impl From<[u8; 16]> for TraceId {
    #[inline]
    fn from(bytes: [u8; 16]) -> TraceId {
        TraceId(Inner::Long(bytes))
    }
}

/// The error returned when parsing a `TraceId` from a string.
#[derive(Debug)]
pub struct TraceIdParseError(Option<DecodeError>);

impl fmt::Display for TraceIdParseError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("error parsing trace ID: ")?;
        match self.0 {
            Some(ref err) => write!(fmt, "{}", err),
            None => fmt.write_str("invalid length"),
        }
    }
}

impl Error for TraceIdParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.as_ref().map(|e| e as _)
    }
}
