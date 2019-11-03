use crate::sample::AlwaysSampler;
use crate::{Endpoint, Report, Span};
use std::cell::RefCell;

thread_local! {
    static SPANS: RefCell<Vec<Span>> = RefCell::new(vec![]);
}

struct TestReporter;

impl Report for TestReporter {
    fn report2(&self, span: Span) {
        SPANS.with(|s| s.borrow_mut().push(span));
    }
}

fn init() {
    let _ = crate::set_tracer(AlwaysSampler, TestReporter, Endpoint::builder().build());
    SPANS.with(|s| s.borrow_mut().clear());
}

#[test]
fn detach() {
    init();

    let parent = crate::new_trace().with_name("parent");
    let parent_trace = parent.context().trace_id();
    let parent_id = parent.context().span_id();

    let detached = crate::next_span().with_name("detached").detach();
    let detached_id = detached.context().span_id();

    let child2 = crate::next_span().with_name("child2");
    let child2_trace = child2.context().trace_id();
    let child2_id = child2.context().span_id();

    drop(detached);
    drop(child2);
    drop(parent);

    let spans = SPANS.with(|s| s.borrow().clone());
    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].id(), detached_id);
    assert_eq!(spans[0].parent_id(), Some(parent_id));
    assert_eq!(spans[1].trace_id(), child2_trace);
    assert_eq!(spans[1].id(), child2_id);
    assert_eq!(spans[1].parent_id(), Some(parent_id));
    assert_eq!(spans[2].trace_id(), parent_trace);
    assert_eq!(spans[2].id(), parent_id);
    assert_eq!(spans[2].parent_id(), None);
}
