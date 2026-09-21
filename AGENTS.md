# Repository Instructions

This file defines the standing rules and technical constraints for work in this repository.
The Japanese translation is [AGENTS.ja.md](AGENTS.ja.md).

## Documentation Synchronization

- Update `AGENTS.md` and `AGENTS.ja.md` together in the same change. Neither file may be changed alone.
- Keep both files semantically equivalent. English is the canonical wording when resolving an ambiguity.
- Apply the same rule to `README.md` / `README.ja.md` and `DEVELOPMENT.md` / `DEVELOPMENT.ja.md`.
- Write public Rust documentation bilingually, with English first and Japanese second.
- Write public development content, including GitHub Issue titles and bodies, bilingually with English first and Japanese second. Apply the same rule to Pull Request descriptions, release notes, and other public planning or decision records.
- Keep README files focused on installation and user-facing usage.
- Keep long-lived repository rules and technical constraints in AGENTS.
- Keep proposed improvements, design questions, and release planning in DEVELOPMENT or their linked GitHub Issues.

## Project Direction

- The current release is `0.1.0`.
- The next major target is `1.0.0`.
- Public API naming and consistency are the primary review goals before `1.0.0`.
- Backward compatibility with `0.1.0` is not required while preparing `1.0.0`. Prefer a coherent API over compatibility shims, but document intentional breaking changes.
- Track actionable improvements as GitHub Issues. Use one issue per independently reviewable change and link it from DEVELOPMENT while it affects the `1.0.0` roadmap.
- Use a GitHub Project when prioritization or progress across multiple issues needs a board or roadmap view.

## Naming

- Published crate: `fixed-record`
- Rust crate name: `fixed_record`
- Proc-macro crate: `fixed-record-macros`
- Attribute macro: `#[fixed_record]`
- Repository: <https://github.com/tomi-912/fixed-record>
- License: MIT No Attribution License (`MIT-0`)

Users should depend only on `fixed-record`. Treat `fixed-record-macros` as an implementation crate re-exported by `fixed-record`. Generated code must use `::fixed_record::...` paths.

## Workspace Layout

```text
crates/
  fixed-record/          # Public library crate
  fixed-record-macros/   # Proc-macro implementation
examples/
  basic/                 # Small runnable examples
  no-list/               # default-features = false test fixture
```

Keep API guarantees and behavioral tests in `crates/fixed-record/`. Keep `examples/basic` small and user-oriented. The no-list compile-fail test belongs in `examples/no-list` because an integration test in the library package cannot conveniently change the package's own dependency features.

## Working Rules

- Preserve unrelated user changes in a dirty worktree.
- Commit source and documentation changes by default, then stop. Push only when the user explicitly requests it.
- Exclude generated `target/` content from searches, reviews, and commits.
- Keep changes focused and add tests in proportion to behavioral risk.
- Run formatting, tests, and Clippy before completing a substantive change:

```bash
cargo fmt --all --check
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

- Also run the relevant examples when changing generated APIs or feature behavior:

```bash
cargo run -p fixed-record-basic-example --bin fixed_record_usage
cargo run -p fixed-record-basic-example --bin macro_reexport
cargo run -p fixed-record-no-list-example
```

## Public API Constraints

### Fixed records and zerocopy

- Generated records are `#[repr(C)]` and derive `zerocopy::FromBytes`, `IntoBytes`, `Immutable`, and `KnownLayout`.
- Keep copying APIs (`parse`, `parse_str`, `to_bytes`) distinct from borrowed byte-view APIs (`ref_from_bytes`, `as_bytes`, `as_mut_bytes`).
- `ref_from_bytes` requires an exact-size input. Prefix variants may accept trailing bytes and map short input to `Error::TooShort`.
- String widths are byte widths, not character counts.
- Do not generate mutable string views: fixed records are byte-oriented, while `&mut str` must remain valid UTF-8.
- Do not reintroduce handwritten unchecked pointer-cast APIs when zerocopy provides the operation safely.

### Generated List API

- The default `list` feature controls generation of `{StructName}List`. With `default-features = false`, generate only the record and field operations.
- Keep generated helper visibility aligned with the input record. Internal index types must remain private.
- List field indexes use typed `BTreeMap<Fixed<N>, Vec<usize>>` entries. Mutations must keep indexes consistent, including during unwinding.
- Fallible search names use the `try_` prefix. Exact lookup validates field width; padded and prefix lookup semantics must remain distinct.
- Position-changing operations must update or rebuild indexes as required. Preserve current List order in search results.
- Do not expose internal vector indexes as stable record identifiers.

### Reader and Writer

- `Reader::new` and `Writer::new` default to LF (`\n`) separators.
- A configured separator is required after every record, including the final record.
- `Writer` replaces NUL bytes with spaces when writing records.
- Reader/Writer changes must cover separator boundaries, incomplete records, I/O errors, and sequence checks.

## Generated API Testing

- Integration behavior lives under `crates/fixed-record/tests/`.
- Invalid macro input, visibility, borrowing restrictions, and unavailable generated APIs belong in compile-fail tests.
- When renaming public APIs for `1.0.0`, update generated code, integration tests, compile-fail fixtures, examples, README, and rustdoc in the same change.
- Treat generated rustdoc as part of the public API review.

## Publishing

- Both publishable crates must retain matching `MIT-0` license files and complete crates.io metadata.
- `fixed-record` must use a versioned path dependency on `fixed-record-macros` so workspace builds use the local crate and published builds resolve the released version.
- Publish `fixed-record-macros` first, wait for its version to reach the crates.io index, then publish `fixed-record`.
- Inspect package contents and run `cargo publish --dry-run` for both crates before a release.
- GitHub Actions generates rustdoc from `main` and publishes it to the generated `gh-pages` branch. Do not edit `gh-pages` manually.
