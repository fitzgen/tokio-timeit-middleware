//! `tokio-timeit-middleware` provides a middleware Tokio `Service` called
//! [`Timeit`](./struct.Timeit.html) to time how long it takes to reply to a
//! `Service::Request` with a `Service::Response`.
//!
//! The recorded timings are sent to a `TimeSink` which may be any smart pointer
//! type that `Deref`s to a function that takes a
//! [`time::Duration`](https://doc.rust-lang.org/time/time/struct.Duration.html)
//! and is `Clone`.
//!
//! # Example
//!
//! ```
//! # extern crate time;
//! # extern crate tokio_service;
//! # extern crate tokio_timeit_middleware;
//! # use std::rc::Rc;
//! # fn foo<S>(my_tokio_service: S) where S: tokio_service::Service {
//! // Send recorded timings to metrics or logging or whatever...
//! let time_sink = Rc::new(|timing: time::Duration| {
//!     println!("Replied to a request with a response in {}", timing);
//! });
//!
//! // Wrap a service in `Timeit`!
//! let my_timed_service = tokio_timeit_middleware::Timeit::new(my_tokio_service, time_sink);
//! # }
//! ```

#![deny(missing_docs)]

extern crate futures;
extern crate time;
extern crate tokio_service;

use futures::{Async, Future, Poll};
use std::ops;
use tokio_service::Service;

/// A middleware that times how long it takes the downstream service `S` to
/// respond to a request with a response. The recorded `time::Duration`s are
/// passed to the `TimeSink`.
pub struct Timeit<S, TimeSink> {
    downstream: S,
    time_sink: TimeSink,
}

impl<S, TimeSink> Timeit<S, TimeSink> {
    /// Wrap the given `service` for timing.
    pub fn new(service: S, time_sink: TimeSink) -> Timeit<S, TimeSink> {
        Timeit {
            downstream: service,
            time_sink: time_sink,
        }
    }
}

impl<S, TimeSink, TimeSinkFn> Service for Timeit<S, TimeSink>
    where S: Service,
          TimeSink: ops::Deref<Target = TimeSinkFn> + Clone,
          TimeSinkFn: Fn(time::Duration)
{
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Future = EndTimeit<S::Future, TimeSink>;

    fn call(&self, request: Self::Request) -> Self::Future {
        EndTimeit {
            start: Some(time::now()),
            time_sink: self.time_sink.clone(),
            future: self.downstream.call(request),
        }
    }

    fn poll_ready(&self) -> Async<()> {
        self.downstream.poll_ready()
    }
}

/// A future that ends a time recording upon resolution.
#[doc(hidden)]
pub struct EndTimeit<F, TimeSink> {
    start: Option<time::Tm>,
    time_sink: TimeSink,
    future: F,
}

impl<F, TimeSink, TimeSinkFn> Future for EndTimeit<F, TimeSink>
    where F: Future,
          TimeSink: ops::Deref<Target = TimeSinkFn> + Clone,
          TimeSinkFn: Fn(time::Duration)
{
    type Item = F::Item;
    type Error = F::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.future.poll() {
            Ok(Async::NotReady) => return Ok(Async::NotReady),
            Err(e) => Err(e),
            Ok(Async::Ready(r)) => {
                let start =
                    self.start.take().expect("Should not call poll on EndTimeit more than once");
                let end = time::now();
                (*self.time_sink)(end - start);
                Ok(Async::Ready(r))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate futures;
    extern crate tokio_service;

    use super::*;

    use futures::{Async, Future};
    use std::cell::Cell;
    use std::rc::Rc;
    use tokio_service::Service;

    struct StubService<C, P> {
        c: C,
        p: P,
    }

    impl<C, P> StubService<C, P> {
        fn new(c: C, p: P) -> StubService<C, P> {
            StubService { c: c, p: p }
        }
    }

    impl<C, T, F, P> Service for StubService<C, P>
        where C: Fn() -> F,
              F: Future<Item = T, Error = ()> + 'static,
              P: Fn() -> Async<()>
    {
        type Request = ();
        type Response = T;
        type Error = ();
        type Future = F;

        fn call(&self, _: Self::Request) -> Self::Future {
            (self.c)()
        }

        fn poll_ready(&self) -> Async<()> {
            (self.p)()
        }
    }

    #[test]
    fn if_downstream_not_ready_neither_are_we() {
        let stub = StubService::new(|| futures::done(Ok(())), || Async::NotReady);
        let wrapped = Timeit::new(stub, Rc::new(|_| unreachable!()));
        assert_eq!(wrapped.poll_ready(), Async::NotReady);
    }

    #[test]
    fn if_downstream_ready_so_are_we() {
        let stub = StubService::new(|| futures::done(Ok(())), || Async::Ready(()));
        let wrapped = Timeit::new(stub, Rc::new(|_| unreachable!()));
        assert_eq!(wrapped.poll_ready(), Async::Ready(()));
    }

    #[test]
    fn if_downstream_returns_value_so_do_we() {
        let stub = StubService::new(|| futures::done(Ok(5)), || Async::NotReady);
        let wrapped = Timeit::new(stub, Rc::new(|_| {}));
        let mut future = wrapped.call(());
        assert_eq!(future.poll(), Ok(Async::Ready(5)));
    }

    #[test]
    fn if_downstream_does_not_return_value_time_sink_not_called() {
        let stub = StubService::new(|| futures::empty::<(), ()>(), || Async::NotReady);
        let wrapped = Timeit::new(stub, Rc::new(|_| unreachable!()));
        let mut future = wrapped.call(());
        assert_eq!(future.poll(), Ok(Async::NotReady));
    }

    #[test]
    fn if_downstream_returns_value_time_sink_is_called() {
        let stub = StubService::new(|| futures::done(Ok(())), || Async::NotReady);

        let times_called = Cell::new(0);
        let wrapped = Timeit::new(stub,
                                  Rc::new(|_| {
                                      times_called.set(times_called.get() + 1);
                                  }));

        let expected_times_called = 10;

        for _ in 0..expected_times_called {
            let mut future = wrapped.call(());
            assert_eq!(future.poll(), Ok(Async::Ready(())));
        }

        assert_eq!(times_called.get(), expected_times_called);
    }
}
