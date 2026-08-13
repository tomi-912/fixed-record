# Development Notes

This file tracks development direction, release preparation, current status, and future work for `fixed-record`.

README files should stay focused on user-facing installation and usage. Internal design notes, working rules, investigation notes, and pre-release tasks belong here.

Japanese documentation is available in [DEVELOPMENT.ja.md](DEVELOPMENT.ja.md).

## Naming

Public-facing names are standardized as follows.

- Published crate: `fixed-record`
- Rust crate name: `fixed_record`
- Proc macro crate: `fixed-record-macros`
- Attribute macro: `#[fixed_record]`
- Repository name: planned rename to `fixed-record`
- License: MIT No Attribution License (`MIT-0`)

Users add only `fixed-record` to `[dependencies]`. `fixed-record-macros` is treated as an internal implementation crate re-exported by `fixed-record`.

Generated code references are standardized to `::fixed_record::...`.

## Workspace Layout

```text
crates/
  fixed-record/
  fixed-record-macros/
examples/
  basic/
  no-list/
```

- `crates/fixed-record/`: the user-facing crate. It provides `Fixed<N>`, `Error`, `Reader`, `Writer`, `FixedRecord`, `prelude`, and the `#[fixed_record]` re-export.
- `crates/fixed-record-macros/`: the proc macro crate that implements `#[fixed_record]`.
- `examples/basic/`: a small user-facing runnable example.
- `examples/no-list/`: a fixture that verifies the `default-features = false` configuration where List generation is disabled.

## Working Rules

- Commit source or documentation changes by default.
- Stop after committing during normal task completion.
- Push only when the user explicitly asks for a push.
- Keep README files focused on public users.
- Put release decisions, design notes, investigation notes, and future tasks in DEVELOPMENT.
- Add or update doc comments in bilingual order: English first, then Japanese.
- When README or DEVELOPMENT content changes, update both the English file and the Japanese companion file.
- Because `target/` may exist locally, use filters such as `rg --glob '!target/**'` while searching.

## Current Status

Status note as of 2026-08-13.

Verified commands:

```bash
cargo fmt --all --check
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo run -p fixed-record-basic-example --bin fixed_record_usage
cargo run -p fixed-record-basic-example --bin macro_reexport
cargo run -p fixed-record-no-list-example
```

Test layout:

- `crates/fixed-record/tests/generated_api.rs`: builder / parse / to_bytes / Reader / Writer / List / zerocopy
- `crates/fixed-record/tests/compile_fail.rs`: compile-fail coverage for invalid proc macro input, immutable List APIs, and sequence checks
- `examples/no-list/tests/compile_fail.rs`: verifies that `{StructName}List` is not generated with `default-features = false`

Verified test results:

- `fixed-record` generated API integration tests: 60 tests pass with default features
- `fixed-record` doctests: 25 tests pass

## What Works Well

- The user-facing entry point is consolidated in `fixed-record`, so users can start with `fixed_record::prelude::*`.
- The proc macro implementation is separated as `fixed-record-macros`, so users do not need to depend on it directly.
- `Fixed<N>` keeps basic fixed-width byte operations, UTF-8 access, zero padding, and space padding compact.
- The `FixedRecord` trait lets `Reader` and `Writer` work without depending too tightly on generated record types.
- `#[fixed_record]` can generate the record body, field enum, metadata, builder, parsing, field operations, Reader/Writer interoperability, and optional List APIs.
- Generated field enum and List visibility now follows the input record visibility, so private records do not leak public generated helper types.
- Generated records are always `#[repr(C)]` and derive `zerocopy::FromBytes`, `IntoBytes`, `Immutable`, and `KnownLayout`, replacing the earlier custom unchecked pointer-cast API with safe zerocopy trait APIs.
- `Fixed<N>` is also zerocopy-compatible, which keeps the generated record layout composed from byte-array-backed fields.
- Zero-copy reference helpers are layered on top of `zerocopy` rather than handwritten unsafe code. `ref_from_bytes_prefix`, `ref_from_str`, and `ref_from_str_prefix` keep crate-level error behavior while using zerocopy for the cast.
- Copying APIs (`parse`, `parse_str`, `to_bytes`) remain separate from borrowed byte-view APIs (`ref_from_bytes`, `as_bytes`, `as_mut_bytes`), which keeps the ownership model explicit.
- Compile-fail tests cover invalid struct shapes and generated type visibility, while integration tests cover byte, string, UTF-8, Reader/Writer, List, and zerocopy behavior.
- Field doc comments are propagated to generated enums and accessors, giving generated APIs a better rustdoc experience.
- The `fixed-record` crate-level rustdoc now includes a bilingual English/Japanese guide with concrete examples for record definition, builders, parsing, dynamic field operations, bulk application helpers, Reader/Writer, sequence checks, searchable Lists, range searches, `clear_byte`, `FixedRecord`, feature flags, and zerocopy.

## Important Notes

### zerocopy support

The old `unchecked` feature was removed. Generated records now always derive zerocopy traits and expose safe zerocopy APIs through trait methods.

Important behavioral points:

- `ref_from_bytes` is the exact-size zerocopy API from `zerocopy::FromBytes`.
- `ref_from_bytes_prefix` accepts trailing bytes and maps short input to `Error::TooShort`.
- `ref_from_str` and `ref_from_str_prefix` treat string widths as byte widths, not character counts.
- `as_bytes` / `as_mut_bytes` are borrowed byte views from `zerocopy::IntoBytes`; `to_bytes` remains a copying API.
- Mutable string-based APIs are intentionally not generated because `&mut str` must remain valid UTF-8 while fixed records are byte-oriented.

### generated List API

`{StructName}List` generation is controlled by the default `list` feature.

- Default features generate List APIs.
- The generated List stores records as `Vec<Box<Record>>`; List IDs are current vector indexes, `remove` physically removes entries, and searching is linear over stored records. Sorting moves the boxes in the vector instead of moving record values themselves.
- Depending on `fixed-record` with `default-features = false` generates only the record body and field operations.

### Reader separators

`Reader::new` and `Writer::new` both default to LF (`\n`) record separators. If an input format
uses a different separator or no separator, callers must configure it with `Reader::with_separator`.
A configured separator is required after every record, including the final record.

### test placement

Core behavior is covered under `crates/fixed-record/tests/`. `examples/basic` remains a small runnable sample, while library guarantees live in the library crate.

The `default-features = false` compile-fail test remains in the dedicated `examples/no-list` fixture because integration tests inside the same package cannot easily switch dependency features for the package under test.

## Publish Checklist

Before publishing to crates.io:

1. Rename the GitHub repository to `fixed-record`.
2. Fill package metadata in `Cargo.toml`.
3. Verify the `MIT-0` license metadata and LICENSE text.
4. Align README installation text with the actual published version.
5. Confirm the public zerocopy API wording in README and rustdoc.
6. Run `cargo package --dry-run -p fixed-record-macros`.
7. Run `cargo package --dry-run -p fixed-record`.
8. Require the following checks in CI.

```bash
cargo fmt --all --check
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

Metadata candidates:

- `description`
- `license` or `license-file`
- `repository`
- `readme`
- `keywords`
- `categories`
- `exclude` / `include`

The `fixed-record-macros` description should state that it is the proc macro implementation crate used through `fixed-record`, not the crate users normally depend on directly.

## Next Recommended Work

1. Add remaining public package metadata.
2. Rename the GitHub repository to `fixed-record`.
3. Split `crates/fixed-record/tests/generated_api.rs` into behavior-focused files.
4. Run `cargo package --dry-run` and inspect the package contents.
