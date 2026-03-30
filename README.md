# "G" Format for Floating Point

Prints floating-point numbers exactly like C's `printf("%g", value)`, using a `Display` implementation.

By default, formatting is done in pure Rust (no dependencies, works on all platforms including Windows and WASM). Enable the `libc` feature to delegate to your system's `snprintf()` instead (only available on Unix).

## Usage

Use the crates.io repository; add this to your `Cargo.toml` along
with the rest of your dependencies:

```toml
[dependencies]
gpoint = "0.3"
```

To use the libc-based implementation on Unix:

```toml
[dependencies]
gpoint = { version = "0.3", features = ["libc"] }
```

Then wrap your `f32` or `f64` with a `GPoint`:

```
use gpoint::GPoint;

println!("answer: {}", GPoint(42.));
```

See the [API documentation](https://docs.rs/gpoint) for further details.
