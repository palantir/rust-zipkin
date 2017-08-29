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
//! This crate provides a `Spanned` wrapper type which ensures that the current
//! trace context is registered with a `Tracer` while a futures type is
//! processing. It can wrap `Future`s, `Sink`s, and `Stream`s.
#![doc(html_root_url="https://docs.rs/zipkin-futures/0.1")]
#![warn(missing_docs)]

extern crate futures;
extern crate zipkin;

use futures::{Future, Poll, Stream, Sink, StartSend};
use zipkin::{Tracer, OpenSpan};

#[deprecated(note = "renamed to Spanned", since = "0.1.1")]
pub use Spanned as SpannedFuture;

/// A wrapper type which ensures that a Zipkin span is active while its inner
/// value runs.
pub struct Spanned<T> {
    span: OpenSpan,
    tracer: Tracer,
    inner: T,
}

impl<T> Spanned<T> {
    /// Returns a new `Spanned`.
    pub fn new(span: OpenSpan, tracer: &Tracer, inner: T) -> Spanned<T> {
        Spanned {
            span,
            tracer: tracer.clone(),
            inner,
        }
    }
}

impl<F> Future for Spanned<F>
where
    F: Future,
{
    type Item = F::Item;
    type Error = F::Error;

    fn poll(&mut self) -> Poll<F::Item, F::Error> {
        let _guard = self.tracer.set_current(self.span.context());
        self.inner.poll()
    }
}

impl<S> Stream for Spanned<S>
where
    S: Stream,
{
    type Item = S::Item;
    type Error = S::Error;

    fn poll(&mut self) -> Poll<Option<S::Item>, S::Error> {
        let _guard = self.tracer.set_current(self.span.context());
        self.inner.poll()
    }
}

impl<S> Sink for Spanned<S>
where
    S: Sink,
{
    type SinkItem = S::SinkItem;
    type SinkError = S::SinkError;

    fn start_send(&mut self, item: S::SinkItem) -> StartSend<S::SinkItem, S::SinkError> {
        let _guard = self.tracer.set_current(self.span.context());
        self.inner.start_send(item)
    }

    fn poll_complete(&mut self) -> Poll<(), S::SinkError> {
        let _guard = self.tracer.set_current(self.span.context());
        self.inner.poll_complete()
    }

    fn close(&mut self) -> Poll<(), S::SinkError> {
        let _guard = self.tracer.set_current(self.span.context());
        self.inner.close()
    }
}