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
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// The network context of a node in the service graph.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Endpoint {
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    service_name: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    ipv4: Option<Ipv4Addr>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    ipv6: Option<Ipv6Addr>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    port: Option<u16>,
}

impl Endpoint {
    /// Returns a builder type used to construct an `Endpoint`.
    #[inline]
    pub fn builder() -> Builder {
        Builder {
            service_name: None,
            ipv4: None,
            ipv6: None,
            port: None,
        }
    }

    /// Returns the name of the service at this endpoint.
    #[inline]
    pub fn service_name(&self) -> Option<&str> {
        self.service_name.as_ref().map(|s| &**s)
    }

    /// Returns the IPv4 address of the service at this endpoint.
    #[inline]
    pub fn ipv4(&self) -> Option<Ipv4Addr> {
        self.ipv4
    }

    /// Returns the IPv6 address of the service at this endpoint.
    #[inline]
    pub fn ipv6(&self) -> Option<Ipv6Addr> {
        self.ipv6
    }

    /// Returns the port of the service at this endpoint.
    #[inline]
    pub fn port(&self) -> Option<u16> {
        self.port
    }
}

/// A builder type for `Endpoint`s.
pub struct Builder {
    service_name: Option<String>,
    ipv4: Option<Ipv4Addr>,
    ipv6: Option<Ipv6Addr>,
    port: Option<u16>,
}

impl From<Endpoint> for Builder {
    #[inline]
    fn from(e: Endpoint) -> Builder {
        Builder {
            service_name: e.service_name,
            ipv4: e.ipv4,
            ipv6: e.ipv6,
            port: e.port,
        }
    }
}

impl Builder {
    /// Sets the service name associated with the endpoint.
    ///
    /// Defaults to `None`.
    #[inline]
    pub fn service_name(&mut self, service_name: &str) -> &mut Builder {
        self.service_name = Some(service_name.to_string());
        self
    }

    /// Sets the IPv4 address associated with the endpoint.
    ///
    /// Defaults to `None`.
    #[inline]
    pub fn ipv4(&mut self, ipv4: Ipv4Addr) -> &mut Builder {
        self.ipv4 = Some(ipv4);
        self
    }

    /// Sets the IPv6 address associated with the endpoint.
    ///
    /// Defaults to `None`.
    #[inline]
    pub fn ipv6(&mut self, ipv6: Ipv6Addr) -> &mut Builder {
        self.ipv6 = Some(ipv6);
        self
    }

    /// Sets the IP address associated with the endpoint.
    ///
    /// This is simply a convenience function which delegates to `ipv4` and
    /// `ipv6`.
    #[inline]
    pub fn ip(&mut self, ip: IpAddr) -> &mut Builder {
        match ip {
            IpAddr::V4(addr) => self.ipv4(addr),
            IpAddr::V6(addr) => self.ipv6(addr),
        }
    }

    /// Sets the port associated with the endpoint.
    ///
    /// Defaults to `None`.
    #[inline]
    pub fn port(&mut self, port: u16) -> &mut Builder {
        self.port = Some(port);
        self
    }

    /// Constructs the `Endpoint`.
    #[inline]
    pub fn build(&self) -> Endpoint {
        Endpoint {
            service_name: self.service_name.clone(),
            ipv4: self.ipv4,
            ipv6: self.ipv6,
            port: self.port,
        }
    }
}
