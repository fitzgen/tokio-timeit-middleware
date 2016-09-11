# tokio-timeit-middleware

Time how long it takes your Tokio `Service` to send a `Service::Response` in
reply to a `Service::Request`.

## Usage

First, add this to your Cargo.toml:

```toml
[dependencies]
tokio-timeit-middleware = { git = "https://github.com/fitzgen/tokio-timeit-middleware" }
```

Next, add this to your crate:

```rust
extern crate tokio_timeit_middleware;
```

To time your Tokio `Service`'s request/response times, wrap it in
`tokio_timeit_middleware::Timeit`:

```rust
let timed_service = Timeit::new(my_tokio_service, Rc::new(|duration| {
    println!("Responded to request in {}", duration);
));
```
