# fixed-record

`fixed-record` generates parsers, builders, field accessors, Reader/Writer support, and searchable List APIs from Rust struct definitions for fixed-width records.

Most users only need to depend on `fixed-record`. The `fixed-record-macros` crate is an implementation detail and is re-exported by `fixed-record`.

Japanese documentation is available in [README.ja.md](README.ja.md).

API documentation is published at <https://tomi-912.github.io/fixed-record/>.

## Installation

Before the crate is published to crates.io, depend on the Git repository.

```toml
[dependencies]
fixed-record = { git = "https://github.com/tomi-912/fixed-record.git" }
```

After publishing to crates.io, use the versioned dependency.

```toml
[dependencies]
fixed-record = "0.1"
```

## Quick Start

```rust
use fixed_record::prelude::*;

#[fixed_record]
pub struct User {
    pub id: Fixed<8>,
    pub name: Fixed<16>,
    pub age: Fixed<3>,
}

let user = User::builder()
    .with_id("00000001")
    .with_name("Tanaka")
    .with_age_int(25)
    .build();

assert_eq!(User::TOTAL_LEN, 27);
assert_eq!(user.id(), b"00000001");
assert_eq!(user.age(), b"025");
assert_eq!(user.get_field_trimmed(UserField::Name).unwrap(), "Tanaka");
```

## Generated API

Applying `#[fixed_record]` to a struct mainly generates:

- the record struct with basic derives
- a `{StructName}Field` enum
- metadata such as `TOTAL_LEN`, field lengths, and offsets
- `builder`, `with_*`, `try_with_*_int`, and `with_*_int_truncated`
- `parse` / `parse_str` / `to_bytes`
- dynamic field operations such as `get_field_*` and `set_field_*`
- bulk application helpers such as `apply_*`
- a `FixedRecord` trait implementation
- `Reader` / `Writer` interoperability
- `{StructName}List` insertion, lookup, range search, removal, `vacuum`, and sorting
- `compare_all_fields` / `compare_by_fields` / `to_dump_string`

## Field Initialization

`set_field_*` clears the target field with `CLEAR_BYTE` before writing. When unspecified, `CLEAR_BYTE` is a space byte (`0x20`).

```rust
#[fixed_record(clear_byte = ZERO)]
pub struct User {
    pub id: Fixed<8>,
}
```

Use `clear_byte = ZERO` or `clear_byte = 0` when `set_field_*`, `builder()`, `default()`, and `cleared()` should initialize fields with `0x00`.

For explicit initialization, use `zeroed()`, `spaced()`, or `cleared()`. `zeroed()` always uses `0x00`, `spaced()` always uses spaces, and `cleared()` uses `CLEAR_BYTE`.

Use `set_field_bytes_no_clear` / `set_field_str_no_clear` when you want to preserve existing trailing bytes. The builder-style `with_*` methods also perform partial overwrites without clearing first.

## List Search

When the default `list` feature is enabled, `{StructName}List` is generated.

```rust
let mut list = UserList::new();
let id = list.insert(user);

let found = list.try_find_by(UserField::Id, b"00000001")?;
let first = list.try_first_by(UserField::Id, b"00000001")?;
let by_id = list.get(id);
```

If you want to specify the field width at the call site, compatibility APIs `find_by<const N: usize>` and `first_by<const N: usize>` are also available. In most cases, prefer `try_find_by` / `try_first_by`, which infer the width from the field enum.

Use `try_find_by_prefix` / `try_first_by_prefix` for prefix searches.

## Reader / Writer

`Reader` reads fixed-width records sequentially. A trailing `\n`, `\r`, `\r\n`, or `,` immediately after each record is skipped automatically.

```rust
let mut reader = Reader::<_, User>::new(source)
    .with_sequence_check([UserField::Id]);

let mut reader = Reader::<_, User>::new(source)
    .with_sequence_check_options([UserField::Id], false);
```

Sequence checks return `Error::SequenceError` when the current record is smaller than the previous record. Equal keys are allowed by default.

`Writer` writes `to_bytes()` output and appends a separator after each record.

Use `RecordSeparator` to choose the separator written after each record.

```rust
let mut writer = Writer::new(output)
    .with_separator(RecordSeparator::Crlf);

let mut csv_like_writer = Writer::new(output)
    .with_separator(RecordSeparator::Comma);

let mut cr_writer = Writer::new(output)
    .with_separator(RecordSeparator::Cr);
```

## Feature Flags

- `list`: enabled by default. Generates `{StructName}List` and search index APIs.
- `unchecked`: generates unsafe zero-copy APIs.

Disable default features when you only want the record type and field operations.

```toml
[dependencies]
fixed-record = { version = "0.1", default-features = false }
```

The `unchecked` feature generates `as_bytes_unchecked` / `parse_unchecked` / `from_bytes_unchecked` / `from_str_unchecked`. Callers must guarantee that the struct memory layout exactly matches the fixed-width record byte layout.

## Examples

```bash
cargo run -p fixed-record-basic-example --bin fixed_record_usage
cargo run -p fixed-record-basic-example --bin macro_reexport
cargo run -p fixed-record-no-list-example
```

## Workspace Layout

```text
crates/
  fixed-record/
  fixed-record-macros/
examples/
  basic/
  no-list/
```

The public entry point is `fixed-record`. `fixed-record-macros` is the proc macro implementation crate.

## License

This project is licensed under the MIT No Attribution License (MIT-0). See [LICENSE](LICENSE).
