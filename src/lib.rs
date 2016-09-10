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
    #[test]
    fn it_works() {}
}
