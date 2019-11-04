use crate::TraceContext;
use std::cell::Cell;
use std::marker::PhantomData;

thread_local! {
    static CURRENT: Cell<Option<TraceContext>> = Cell::new(None);
}

/// A guard object for the thread-local current trace context.
///
/// It will restore the previous trace context when it drops.
pub struct CurrentGuard {
    prev: Option<TraceContext>,
    // make sure this type is !Send since it pokes at thread locals
    _p: PhantomData<*const ()>,
}

unsafe impl Sync for CurrentGuard {}

impl Drop for CurrentGuard {
    fn drop(&mut self) {
        CURRENT.with(|c| c.set(self.prev));
    }
}

/// Sets this thread's current trace context.
///
/// This method does not start a span. It is designed to be used when
/// propagating the trace of an existing span to a new thread.
///
/// A guard object is returned which will restore the previous trace context
/// when it falls out of scope.
pub fn set_current(context: TraceContext) -> CurrentGuard {
    CurrentGuard {
        prev: CURRENT.with(|c| c.replace(Some(context))),
        _p: PhantomData,
    }
}

/// Returns this thread's current trace context.
pub fn current() -> Option<TraceContext> {
    CURRENT.with(|c| c.get())
}
