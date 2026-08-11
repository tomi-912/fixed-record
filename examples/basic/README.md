# fixed-record basic example

This example shows how application code uses the public `fixed-record` crate.

The main API guarantees live in `crates/fixed-record/tests/`. This example stays small so users can read and run it as a practical sample.

Japanese documentation is available in [README.ja.md](README.ja.md).

## Crate Roles

- `fixed-record`: the main crate users depend on. It re-exports `Fixed<N>`, `Error`, `Reader`, `Writer`, `FixedRecord`, `prelude`, and `#[fixed_record]`.
- `fixed-record-macros`: the internal proc macro crate that implements `#[fixed_record]`.

## Which Crate Should Users Import?

Application code should normally depend only on `fixed-record`.

```rust
use fixed_record::prelude::*;
```

The prelude includes `Fixed`, `Reader`, `Writer`, and the re-exported `#[fixed_record]` macro.

Users do not normally depend on `fixed-record-macros` directly.

## Run

User-facing API example:

```bash
cargo run -p fixed-record-basic-example --bin fixed_record_usage
```

Explicit import of the re-exported proc macro:

```bash
cargo run -p fixed-record-basic-example --bin macro_reexport
```

## Relationship

`fixed-record-macros` generates code that refers to items such as `::fixed_record::Fixed` and `::fixed_record::Error`.

In short, `fixed-record-macros` generates code, while `fixed-record` provides the public entry point and runtime types used by that generated code.
