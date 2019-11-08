use crate::sample::AlwaysSampler;
use crate::{Endpoint, Report, Span};
use futures::executor;
use std::cell::RefCell;

thread_local! {
    static SPANS: RefCell<Vec<Span>> = RefCell::new(vec![]);
}

struct TestReporter;

impl Report for TestReporter {
    fn report(&self, span: Span) {
        SPANS.with(|s| s.borrow_mut().push(span));
    }
}

fn init() {
    let _ = crate::set_tracer(AlwaysSampler, TestReporter, Endpoint::builder().build());
    SPANS.with(|s| s.borrow_mut().clear());
}

#[test]
fn detach_attach() {
    init();

    let parent = crate::new_trace();
    let parent_trace = parent.context().trace_id();
    let parent_id = parent.context().span_id();

    let detached = crate::next_span().detach();
    let detached_id = detached.context().span_id();

    let child2 = crate::next_span();
    let child2_trace = child2.context().trace_id();
    let child2_id = child2.context().span_id();

    let attached = detached.attach();
    let child3 = crate::next_span();
    let child3_id = child3.context().span_id();

    drop(child3);
    drop(attached);
    drop(child2);
    drop(parent);

    let spans = SPANS.with(|s| s.borrow().clone());
    assert_eq!(spans.len(), 4);
    assert_eq!(spans[0].id(), child3_id);
    assert_eq!(spans[0].parent_id(), Some(detached_id));
    assert_eq!(spans[1].id(), detached_id);
    assert_eq!(spans[1].parent_id(), Some(parent_id));
    assert_eq!(spans[2].trace_id(), child2_trace);
    assert_eq!(spans[2].id(), child2_id);
    assert_eq!(spans[2].parent_id(), Some(parent_id));
    assert_eq!(spans[3].trace_id(), parent_trace);
    assert_eq!(spans[3].id(), parent_id);
    assert_eq!(spans[3].parent_id(), None);
}

#[test]
fn bind() {
    init();

    let future_root = crate::next_span();
    let future_root_context = future_root.context();

    let future = async {
        let span = crate::next_span();
        span.context()
    };

    let future = future_root.detach().bind(future);

    let other_root = crate::next_span();
    let other_root_context = other_root.context();

    let future_context = executor::block_on(future);

    drop(other_root);

    let spans = SPANS.with(|s| s.borrow().clone());
    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].id(), future_context.span_id());
    assert_eq!(spans[0].parent_id(), Some(future_root_context.span_id()));
    assert_eq!(spans[1].id(), future_root_context.span_id());
    assert_eq!(spans[1].parent_id(), None);
    assert_eq!(spans[2].id(), other_root_context.span_id());
    assert_eq!(spans[2].parent_id(), None);
}
