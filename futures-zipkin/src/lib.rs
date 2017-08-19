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

//! Futures support for Zipkin tracing.
//!
//! The `Tracer` type uses thread local storage to track the current span. This
//! works well in blocking applications where a unit of work "owns" a thread
//! while it's running. However, it is less appropriate for futures-based
//! applications where multiple distinct futures are being evaluated on the same
//! thread in an interleaved fashion.
//!
//! This crate provides a wrapper `Future` which ensures that the thread-local
//! current span is set appropriately whenever the inner `Future` is running.
#![warn(missing_docs)]

extern crate futures;
extern crate zipkin;

use futures::{Future, Poll};
use zipkin::{Tracer, OpenSpan};

/// A wrapping `Future` which ensures that a Zipkin span is active while its
/// inner future runs.
pub struct SpannedFuture<F> {
    span: OpenSpan,
    tracer: Tracer,
    future: F,
}

impl<F> SpannedFuture<F>
where
    F: Future,
{
    /// Returns a new `SpannedFuture`.
    pub fn new(span: OpenSpan, tracer: &Tracer, future: F) -> SpannedFuture<F> {
        SpannedFuture {
            span,
            tracer: tracer.clone(),
            future,
        }
    }
}

impl<F> Future for SpannedFuture<F>
where
    F: Future,
{
    type Item = F::Item;
    type Error = F::Error;

    fn poll(&mut self) -> Poll<F::Item, F::Error> {
        let _guard = self.tracer.set_current(self.span.context());
        self.future.poll()
    }
}
