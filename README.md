# rust-zipkin

[![CircleCI](https://circleci.com/gh/palantir/rust-zipkin.svg?style=shield)](https://circleci.com/gh/palantir/rust-zipkin)

A collection of crates to support the Zipkin distributed tracing system.

## zipkin-types

[Documentation](https://docs.rs/zipkin-types)

The zipkin-types crate defines Rust types corresponding to Zipkin's object
schema.

## zipkin

[Documentation](https://docs.rs/zipkin)

The zipkin crate defines a `Tracer` object which handles the heavy lifting of
creating and recording Zipkin spans.

## http-zipkin

[Documentation](https://docs.rs/http-zipkin)

The http-zipkin crate provides functions to serialize and deserialize zipkin
`TraceContext` and `SamplingFlags` values into and out of http `HeaderMap`
collections to propagate traces across HTTP requests.

## License

This repository is made available under the [Apache 2.0 License](http://www.apache.org/licenses/LICENSE-2.0).
