//! TODO

#![deny(missing_docs)]

extern crate futures;
extern crate time;
extern crate tokio_service;

use futures::{Async, Future, Poll};
use tokio_service::Service;

/// TODO
pub struct TimeitService<S> {
    downstream: S,
}

impl<S> TimeitService<S> {
    /// TODO
    pub fn new(service: S) -> TimeitService<S> {
        TimeitService {
            downstream: service
        }
    }
}

impl<S> Service for TimeitService<S>
    where S: Service
{
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Future = EndTimeit<S::Future>;

    fn call(&self, request: Self::Request) -> Self::Future {
        EndTimeit {
            start: Some(time::now()),
            future: self.downstream.call(request),
        }
    }

    fn poll_ready(&self) -> Async<()> {
        self.downstream.poll_ready()
    }
}

/// TODO
pub struct EndTimeit<F> {
    start: Option<time::Tm>,
    future: F,
}

impl<F> Future for EndTimeit<F>
    where F: Future
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
                println!("call took {}", end - start);
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

    use futures::Async;
    use tokio_service::Service;

    struct StubService<P> {
        p: P,
    }

    impl<P> StubService<P> {
        fn new(p: P) -> StubService<P> {
            StubService {
                p: p
            }
        }
    }

    impl<P> Service for StubService<P> where P: Fn() -> Async<()> {
        type Request = ();
        type Response = ();
        type Error = ();
        type Future = futures::Done<(), ()>;

        fn call(&self, _: Self::Request) -> Self::Future {
            futures::done(Ok(()))
        }

        fn poll_ready(&self) -> Async<()> {
            (self.p)()
        }
    }

    #[test]
    fn if_downstream_not_ready_neither_are_we() {
        let stub = StubService::new(|| Async::NotReady);
        let wrapped = TimeitService::new(stub);
        assert_eq!(wrapped.poll_ready(), Async::NotReady);
    }

    #[test]
    fn if_downstream_ready_so_are_we() {
        let stub = StubService::new(|| Async::Ready(()));
        let wrapped = TimeitService::new(stub);
        assert_eq!(wrapped.poll_ready(), Async::Ready(()));
    }
}
