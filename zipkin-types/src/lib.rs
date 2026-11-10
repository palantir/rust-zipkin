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

//! Type definitions for Zipkin distributed trace information.
//!
//! This library corresponds to version 2 of the Zipkin [specification].
//!
//! # Serialization
//!
//! If the `serde` Cargo feature is enabled, `Annotation`, `Endpoint`, `Kind`, `Span`, `SpanId`, and
//! `TraceId` implement `Serialize` and `Deserialize` in the standard Zipkin format.
//!
//! [specification]: https://github.com/openzipkin/zipkin-api/blob/master/zipkin2-api.yaml
#![doc(html_root_url = "https://docs.rs/zipkin-types/0.1")]
#![warn(missing_docs)]

#[doc(inline)]
pub use crate::annotation::Annotation;
#[doc(inline)]
pub use crate::endpoint::Endpoint;
#[doc(inline)]
pub use crate::span::{Kind, Span};
#[doc(inline)]
pub use crate::span_id::SpanId;
#[doc(inline)]
pub use crate::trace_id::TraceId;

pub mod annotation;
pub mod endpoint;
pub mod span;
pub mod span_id;
pub mod trace_id;

fn encode_hex<'a>(bytes: &[u8], buf: &'a mut [u8; 32]) -> &'a str {
    let buf = &mut buf[..bytes.len() * 2];
    data_encoding::HEXLOWER.encode_mut(bytes, buf);
    std::str::from_utf8(buf).unwrap()
}

#[cfg(feature = "serde")]
mod time_micros {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn to_wire(time: &SystemTime) -> u64 {
        super::duration_micros::to_wire(
            &time
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0)),
        )
    }

    pub fn from_wire(time: u64) -> SystemTime {
        let duration = super::duration_micros::from_wire(time);
        UNIX_EPOCH + duration
    }

    pub fn serialize<S>(time: &SystemTime, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        to_wire(time).serialize(s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        u64::deserialize(d).map(from_wire)
    }
}

#[cfg(feature = "serde")]
mod duration_micros {
    use std::time::Duration;

    pub fn to_wire(duration: &Duration) -> u64 {
        let micros = duration.as_secs() * 1_000_000 + duration.subsec_nanos() as u64 / 1_000;
        micros.max(1)
    }

    pub fn from_wire(duration: u64) -> Duration {
        let seconds = duration / 1_000_000;
        let subsec_nanos = (duration % 1_000_000) * 1_000;
        Duration::new(seconds, subsec_nanos as u32)
    }
}

#[cfg(feature = "serde")]
mod opt_time_micros {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::SystemTime;

    pub fn serialize<S>(time: &Option<SystemTime>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *time {
            Some(ref time) => s.serialize_some(&super::time_micros::to_wire(time)),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Option<SystemTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<u64>::deserialize(d).map(|o| o.map(super::time_micros::from_wire))
    }
}

#[cfg(feature = "serde")]
mod opt_duration_micros {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Option<Duration>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *duration {
            Some(ref duration) => s.serialize_some(&super::duration_micros::to_wire(duration)),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<u64>::deserialize(d).map(|o| o.map(super::duration_micros::from_wire))
    }
}

#[cfg(test)]
mod test {
    use crate::{SpanId, TraceId};

    #[test]
    fn id_display() {
        let id = TraceId::from([0x00, 0x1f, 0xa0, 0xff, 0x12, 0x34, 0xab, 0xcd]);
        assert_eq!(id.to_string(), "001fa0ff1234abcd");

        let id = TraceId::from([0xab; 16]);
        assert_eq!(id.to_string(), "abababababababababababababababab");

        let id = SpanId::from([0x0f, 0xf0, 0, 1, 2, 3, 4, 5]);
        assert_eq!(id.to_string(), "0ff0000102030405");
    }

    #[test]
    #[cfg(feature = "serde")]
    fn missing_timestamp_and_duration() {
        let json = r#"{"traceId":"0101010101010101","id":"0202020202020202"}"#;
        let span = serde_json::from_str::<crate::Span>(json).unwrap();
        assert_eq!(span.timestamp(), None);
        assert_eq!(span.duration(), None);
        assert_eq!(serde_json::to_string(&span).unwrap(), json);
    }
}
