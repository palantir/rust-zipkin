use std::sync::{Arc, Mutex};

use {Endpoint, Report, Span, Tracer};

struct TestReporter(Arc<Mutex<Vec<Span>>>);

impl Report for TestReporter {
    fn report(&self, span: &Span) {
        self.0.lock().unwrap().push(span.clone());
    }
}

#[test]
fn detach() {
    let spans = Arc::new(Mutex::new(vec![]));
    let tracer = Tracer::builder()
        .reporter(Box::new(TestReporter(spans.clone())))
        .build(Endpoint::builder().service_name("test").build());

    let parent = tracer.new_trace().with_name("parent");
    let parent_trace = parent.context().trace_id();
    let parent_id = parent.context().span_id();

    let detached = tracer.next_span().with_name("detached").detach();
    let detached_id = detached.context().span_id();

    let child2 = tracer.next_span().with_name("child2");
    let child2_trace = child2.context().trace_id();
    let child2_id = child2.context().span_id();

    drop(detached);
    drop(child2);
    drop(parent);

    let spans = spans.lock().unwrap();
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
