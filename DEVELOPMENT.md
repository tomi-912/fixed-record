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
- Repository: <https://github.com/tomi-912/fixed-record>
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

Status note as of 2026-08-14.

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
- `crates/fixed-record/tests/compile_fail.rs`: compile-fail coverage for invalid proc macro input, immutable List APIs, private search indexes, mutable-reference escape attempts, iterators held across mutable sorting, and sequence checks
- `examples/no-list/tests/compile_fail.rs`: verifies that `{StructName}List` is not generated with `default-features = false`

Verified test results:

- `fixed-record` generated API integration tests: 83 tests pass with default features
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
- The generated List stores records as `Vec<Box<Record>>`. A private `{StructName}ListIndices` struct stores one `BTreeMap<Fixed<N>, Vec<usize>>` per record field, so differently sized `Fixed<N>` keys remain statically typed and no `Vec<u8>` key allocation is needed. Dynamic field APIs dispatch to these typed maps with generated `match` arms. The index type is always private, including for public records. Fallible searches consistently use `try_`; methods without that prefix return `Option`, `Vec`, or an iterator directly. Exact `try_find_by` / `try_first_by` lookup requires the selected field width and distinguishes `Error::TooShort` from `Error::FieldOverflow`. `try_find_by_padded` / `try_first_by_padded` accept short values whose remaining bytes are padding, while the prefix variants accept arbitrary trailing bytes. `try_find_range_by` uses `ByteRangeBounds` to accept standard ranges over `AsRef<[u8]>` values; short start/end bounds are extended with `0x00`/`0xFF`, oversized bounds return `FieldOverflow`, and reversed bounds return `InvalidRange`. Prefix, padded-value, and sorted lookup use ordered field-index ranges. `from_records` consumes a `Vec<Record>`, preserves its order while boxing the records, and builds all field indexes once. `From<Vec<Record>>` delegates to `from_records` for idiomatic conversion without duplicating index construction. `push` appends and indexes one record. Position-based `insert` increments existing indexed IDs at or after the insertion point, then indexes the new record without rebuilding field keys. `update` maintains affected entries. `remove` unindexes one record and decrements later IDs without rebuilding field keys. `sort` and `sort_by` rebuild indexes because the complete order can change. `pop` only unindexes the removed last record because remaining IDs do not change. `clear` removes all records and replaces all field indexes with empty maps. Sorting and insertion move boxes instead of moving record values themselves.
- Read and edit APIs share private `Vec<usize>` / `Option<usize>` lookup helpers, so `try_edit_by*`, `try_edit_range_by`, and `try_edit_first_by*` never expose current indexes. Filtered edits snapshot only matching records and repair affected index entries through a drop guard. `for_each_mut` confines mutable references to its callback and uses a rebuild guard because every record may change. Both guards preserve index consistency during unwinding.
- For `u` distinct values in a selected field and `k` matches, exact lookup is `O(log u + k)` instead of scanning all `n` records. Prefix and range lookup are `O(log u + m + k log k)`, where `m` is the number of distinct indexed keys visited; matching IDs are sorted to preserve current List order. The tradeoff is index memory for copied field bytes and one `usize` per record per field.
- Predicate-based `find`, `find_all`, and `retain` remain available for compatibility but are deprecated. They require an `O(n)` scan; `retain` additionally rebuilds every field index. Public guidance should lead users to the indexed `try_first_by*`, `try_find_by*`, and `try_find_range_by` APIs.
- The `RecordWithList` trait associates each generated record with its generated List type. With the `list` feature, `Reader::collect_list` collects all remaining records into a `Vec<Record>`, returns the first Reader error without a partial List, and uses `From<Vec<Record>>` to build the indexed List. Generated `List::read_from` consumes a configured `Reader<R, Record>` and delegates to `collect_list`, preserving separator and sequence-check settings.
- Depending on `fixed-record` with `default-features = false` generates only the record body and field operations.

### Reader separators

`Reader::new` and `Writer::new` both default to LF (`\n`) record separators. If an input format
uses a different separator or no separator, callers must configure it with `Reader::with_separator`.
A configured separator is required after every record, including the final record.
`Writer::write_all` accepts an iterator of borrowed records, delegates each item to `write_record`,
and therefore preserves iterator order, NUL replacement, and separator settings.

### test placement

Core behavior is covered under `crates/fixed-record/tests/`. `examples/basic` remains a small runnable sample, while library guarantees live in the library crate.

The `default-features = false` compile-fail test remains in the dedicated `examples/no-list` fixture because integration tests inside the same package cannot easily switch dependency features for the package under test.

## Publish Checklist

Release setup already completed:

- The GitHub repository and manifest `repository` URLs use `tomi-912/fixed-record`.
- Both package manifests intended for publication define descriptions, documentation, README, keywords, and categories.
- Both packages retain the SPDX `MIT-0` metadata and include a package-local LICENSE copy matching the repository root.
- `fixed-record` uses a versioned path dependency on `fixed-record-macros`, so local builds use the workspace crate and published builds use the crates.io release.
- README installation instructions use the crates.io version dependency.
- Both package archives have been inspected, and `cargo publish --dry-run -p fixed-record-macros` passes without metadata warnings.

Before publishing to crates.io:

1. Confirm the public zerocopy API wording in README and rustdoc.
2. Publish `fixed-record-macros` and wait for version `0.1.0` to appear in the crates.io index.
3. Run `cargo publish --dry-run -p fixed-record`, then publish `fixed-record`.
4. Require the following checks in CI.

```bash
cargo fmt --all --check
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

Both manifests use the repository root README through `readme = "../../README.md"`; Cargo copies it into each package archive as `README.md`. The proc-macro description identifies it as the implementation crate for `fixed-record`, rather than a user-facing dependency.

## Next Recommended Work

1. Publish `fixed-record-macros`, then complete the `fixed-record` dry-run and publish sequence.
2. Split `crates/fixed-record/tests/generated_api.rs` into behavior-focused files.
