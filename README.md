# rust-zipkin

[![CircleCI](https://circleci.com/gh/palantir/rust-zipkin.svg?style=svg)](https://circleci.com/gh/palantir/rust-zipkin)

A collection of crates to support the Zipkin distributed tracing system.

## zipkin

[Documentation](https://docs.rs/zipkin)

The zipkin crate defines Rust types corresponding to Zipkin's object schema, as
well as a `Tracer` object which handles the heavy lifting of creating and
recording Zipkin spans.

## hyper-zipkin

[Documentation](https://docs.rs/hyper-zipkin)

The hyper-zipkin crate defines Hyper header types corresponding to the standard
headers used for propagation of Zipkin trace contexts thorough remote calls, as
well as functions to serialize and deserialize zipkin `TraceContext` values
into and out of Hyper `Headers` collections.

## License

This repository is made available under the [Apache 2.0 License](http://www.apache.org/licenses/LICENSE-2.0).
