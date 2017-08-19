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

extern crate futures;
extern crate zipkin;

use futures::{Future, Poll};
use zipkin::{Tracer, OpenSpan};

pub struct SpannedFuture<F> {
    span: OpenSpan,
    tracer: Tracer,
    future: F,
}

impl<F> SpannedFuture<F>
where
    F: Future,
{
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
