use std::panic;
use zipkin::sample::AlwaysSampler;
use zipkin::{Endpoint, Report, Span};

struct PanickingReporter;

impl Report for PanickingReporter {
    fn report(&self, _: Span) {
        panic!("reporter panic");
    }
}

#[test]
fn reporter_panic_while_unwinding() {
    zipkin::set_tracer(
        AlwaysSampler,
        PanickingReporter,
        Endpoint::builder().build(),
    )
    .unwrap();

    let result = panic::catch_unwind(|| {
        let _span = zipkin::new_trace();
        panic!("user panic");
    });
    assert!(result.is_err());
}
