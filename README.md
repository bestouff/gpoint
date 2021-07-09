# "G" Format for Floating Point

Prints floating-point exactly like a `printf("%g", value)`, using a `Display` implementation via `libc`'s `printf()`'s .

## Usage

Use the crates.io repository; add this to your `Cargo.toml` along
with the rest of your dependencies:

```toml
[dependencies]
gpoint = "0.1"
```

Then wrap your `f64` with a `GPoint`:

```
use gpoint::GPoint;

println!("answer: {:.3}", GPoint(42.));
```

See the [API documentation](https://docs.rs/gpoint) for further details.

## TODO

- [ ] be available for `f32` (for now it's `f64` only)

