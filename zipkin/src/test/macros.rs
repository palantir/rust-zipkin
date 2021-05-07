//  Copyright 2020 Palantir Technologies, Inc.
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
use crate as zipkin; // hack to get the macro codegen to work in the same crate
use crate::{spanned, test};
use futures::executor;

fn is_send<T>(_: T)
where
    T: Send,
{
}

#[test]
fn blocking_free_function() {
    #[spanned(name = "foobar")]
    fn foo() {
        zipkin::next_span().with_name("fizzbuzz");
    }

    test::init();

    let span = zipkin::next_span().with_name("root");
    foo();
    drop(span);

    let spans = test::take();
    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].name(), Some("fizzbuzz"));
    assert_eq!(spans[1].name(), Some("foobar"));
    assert_eq!(spans[2].name(), Some("root"));
    assert_eq!(spans[0].trace_id(), spans[2].trace_id());
    assert_eq!(spans[1].trace_id(), spans[2].trace_id());
    assert_eq!(spans[0].parent_id(), Some(spans[1].id()));
    assert_eq!(spans[1].parent_id(), Some(spans[2].id()));
    assert_eq!(spans[2].parent_id(), None);
}

#[test]
fn blocking_associated_function() {
    struct Foo;

    impl Foo {
        #[spanned(name = "foobar")]
        fn foo() {
            zipkin::next_span().with_name("fizzbuzz");
        }
    }

    test::init();

    let span = zipkin::next_span().with_name("root");
    Foo::foo();
    drop(span);

    let spans = test::take();
    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].name(), Some("fizzbuzz"));
    assert_eq!(spans[1].name(), Some("foobar"));
    assert_eq!(spans[2].name(), Some("root"));
    assert_eq!(spans[0].trace_id(), spans[2].trace_id());
    assert_eq!(spans[1].trace_id(), spans[2].trace_id());
    assert_eq!(spans[0].parent_id(), Some(spans[1].id()));
    assert_eq!(spans[1].parent_id(), Some(spans[2].id()));
    assert_eq!(spans[2].parent_id(), None);
}

#[test]
fn blocking_method() {
    struct Foo;

    impl Foo {
        #[spanned(name = "foobar")]
        fn foo(&self) {
            zipkin::next_span().with_name("fizzbuzz");
        }
    }

    test::init();

    let span = zipkin::next_span().with_name("root");
    Foo.foo();
    drop(span);

    let spans = test::take();
    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].name(), Some("fizzbuzz"));
    assert_eq!(spans[1].name(), Some("foobar"));
    assert_eq!(spans[2].name(), Some("root"));
    assert_eq!(spans[0].trace_id(), spans[2].trace_id());
    assert_eq!(spans[1].trace_id(), spans[2].trace_id());
    assert_eq!(spans[0].parent_id(), Some(spans[1].id()));
    assert_eq!(spans[1].parent_id(), Some(spans[2].id()));
    assert_eq!(spans[2].parent_id(), None);
}

#[test]
fn async_free_function() {
    #[spanned(name = "foobar")]
    async fn foo() {
        zipkin::next_span().with_name("fizzbuzz");
    }

    is_send(foo());

    test::init();

    let future = zipkin::next_span().with_name("root").detach().bind(foo());
    executor::block_on(future);

    let spans = test::take();
    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].name(), Some("fizzbuzz"));
    assert_eq!(spans[1].name(), Some("foobar"));
    assert_eq!(spans[2].name(), Some("root"));
    assert_eq!(spans[0].trace_id(), spans[2].trace_id());
    assert_eq!(spans[1].trace_id(), spans[2].trace_id());
    assert_eq!(spans[0].parent_id(), Some(spans[1].id()));
    assert_eq!(spans[1].parent_id(), Some(spans[2].id()));
    assert_eq!(spans[2].parent_id(), None);
}

#[test]
fn async_associated_function() {
    struct Foo;

    impl Foo {
        #[spanned(name = "foobar")]
        async fn foo() {
            zipkin::next_span().with_name("fizzbuzz");
        }
    }

    is_send(Foo::foo());

    test::init();

    let future = zipkin::next_span()
        .with_name("root")
        .detach()
        .bind(Foo::foo());
    executor::block_on(future);

    let spans = test::take();
    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].name(), Some("fizzbuzz"));
    assert_eq!(spans[1].name(), Some("foobar"));
    assert_eq!(spans[2].name(), Some("root"));
    assert_eq!(spans[0].trace_id(), spans[2].trace_id());
    assert_eq!(spans[1].trace_id(), spans[2].trace_id());
    assert_eq!(spans[0].parent_id(), Some(spans[1].id()));
    assert_eq!(spans[1].parent_id(), Some(spans[2].id()));
    assert_eq!(spans[2].parent_id(), None);
}

#[test]
fn async_method() {
    struct Foo;

    impl Foo {
        #[spanned(name = "foobar")]
        async fn foo(&self) {
            zipkin::next_span().with_name("fizzbuzz");
        }
    }

    is_send(Foo.foo());

    test::init();

    let future = zipkin::next_span()
        .with_name("root")
        .detach()
        .bind(Foo.foo());
    executor::block_on(future);

    let spans = test::take();
    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].name(), Some("fizzbuzz"));
    assert_eq!(spans[1].name(), Some("foobar"));
    assert_eq!(spans[2].name(), Some("root"));
    assert_eq!(spans[0].trace_id(), spans[2].trace_id());
    assert_eq!(spans[1].trace_id(), spans[2].trace_id());
    assert_eq!(spans[0].parent_id(), Some(spans[1].id()));
    assert_eq!(spans[1].parent_id(), Some(spans[2].id()));
    assert_eq!(spans[2].parent_id(), None);
}
