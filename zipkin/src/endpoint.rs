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

//! Endpoints.
use std::net::{Ipv4Addr, Ipv6Addr, IpAddr};

/// An `Endpoint` represents information about a service recording trace
/// information.
///
/// It consists of a service name, an optional IPv4 and/or IPv6 address, and an
/// optional port.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Endpoint {
    service_name: String,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    ipv4: Option<Ipv4Addr>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    ipv6: Option<Ipv6Addr>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    port: Option<u16>,
}

impl Endpoint {
    /// Returns a builder type used to construct an `Endpoint`.
    pub fn builder() -> Builder {
        Builder {
            ipv4: None,
            ipv6: None,
            port: None,
        }
    }

    /// Returns the name of the service at this endpoint.
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Returns the name of the service at this endpoint.
    #[deprecated(since = "0.1.5", note = "renamed to service_name")]
    pub fn name(&self) -> &str {
        self.service_name()
    }

    /// Returns the IPv4 address of the service at this endpoint.
    pub fn ipv4(&self) -> Option<Ipv4Addr> {
        self.ipv4
    }

    /// Returns the IPv6 address of the service at this endpoint.
    pub fn ipv6(&self) -> Option<Ipv6Addr> {
        self.ipv6
    }

    /// Returns the port of the service at this endpoint.
    pub fn port(&self) -> Option<u16> {
        self.port
    }
}

/// A builder type for `Endpoint`s.
pub struct Builder {
    ipv4: Option<Ipv4Addr>,
    ipv6: Option<Ipv6Addr>,
    port: Option<u16>,
}

impl Builder {
    /// Sets the IPv4 address associated with the endpoint.
    ///
    /// Defaults to `None`.
    pub fn ipv4(&mut self, ipv4: Ipv4Addr) -> &mut Builder {
        self.ipv4 = Some(ipv4);
        self
    }

    /// Sets the IPv6 address associated with the endpoint.
    ///
    /// Defaults to `None`.
    pub fn ipv6(&mut self, ipv6: Ipv6Addr) -> &mut Builder {
        self.ipv6 = Some(ipv6);
        self
    }

    /// Sets the IP address associated with the endpoint.
    ///
    /// This is simply a convenience function which delegates to `ipv4` and
    /// `ipv6`.
    pub fn ip(&mut self, ip: IpAddr) -> &mut Builder {
        match ip {
            IpAddr::V4(addr) => self.ipv4(addr),
            IpAddr::V6(addr) => self.ipv6(addr),
        }
    }

    /// Sets the port associated with the endpoint.
    ///
    /// Defaults to `None`.
    pub fn port(&mut self, port: u16) -> &mut Builder {
        self.port = Some(port);
        self
    }

    /// Constructs the `Endpoint`.
    pub fn build(&mut self, service_name: &str) -> Endpoint {
        Endpoint {
            service_name: service_name.to_string(),
            ipv4: self.ipv4.take(),
            ipv6: self.ipv6.take(),
            port: self.port.take(),
        }
    }
}
