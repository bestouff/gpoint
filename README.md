# "G" Format for Floating Point

Prints floating-point exactly like a `printf("%g", value)`, using a `Display` implementation via `libc`'s `printf()`'s .

## Usage

Use the crates.io repository; add this to your `Cargo.toml` along
with the rest of your dependencies:

```toml
[dependencies]
gpoint = "0.2"
```

Then wrap your `f32` or `f64` with a `GPoint`:

```
use gpoint::GPoint;

println!("answer: {}", GPoint(42.));
```

See the [API documentation](https://docs.rs/gpoint) for further details.

