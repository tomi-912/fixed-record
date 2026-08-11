# fixed-record

`fixed-record` は、Rust の struct 定義から固定長レコード用のパース、生成、フィールド操作、Reader/Writer、検索用 List API を生成するライブラリです。

利用者は通常、`fixed-record` だけを依存に追加します。`fixed-record-macros` は `fixed-record` から再エクスポートされる内部実装用の proc macro crate です。

英語版は [README.md](README.md) にあります。

API ドキュメントは <https://tomi-912.github.io/fixed-record/> で公開しています。

## インストール

```toml
[dependencies]
fixed-record = "0.1"
```

## クイックスタート

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

## 生成される API

`#[fixed_record]` を付けた struct から、主に次の API を生成します。

- 基本 derive を付けたレコード struct
- `{StructName}Field` enum
- `TOTAL_LEN`、フィールド長、オフセットなどのメタ情報
- `builder`、`with_*`、`try_with_*_int`、`with_*_int_truncated`
- `parse` / `parse_str` / `to_bytes`
- `get_field_*` / `set_field_*` などの動的フィールド操作
- `apply_*` 系の一括流し込み
- `FixedRecord` trait 実装
- `Reader` / `Writer` との連携
- `{StructName}List` による挿入、検索、範囲検索、削除、`vacuum`、ソート
- `compare_all_fields` / `compare_by_fields` / `to_dump_string`

## フィールド初期化

`set_field_*` は、書き込み前に `CLEAR_BYTE` で対象フィールドをクリアします。未指定時の `CLEAR_BYTE` は半角スペース (`0x20`) です。

```rust
#[fixed_record(clear_byte = ZERO)]
pub struct User {
    pub id: Fixed<8>,
}
```

`set_field_*`、`builder()`、`default()`、`cleared()` で `0x00` 初期化したい場合は、`clear_byte = ZERO` または `clear_byte = 0` を指定します。

明示的な初期化には、`zeroed()`、`spaced()`、`cleared()` を使えます。`zeroed()` は常に `0x00`、`spaced()` は常に半角スペース、`cleared()` は `CLEAR_BYTE` で初期化します。

既存の後続バイトを残したい場合は、`set_field_bytes_no_clear` / `set_field_str_no_clear` を使います。`with_*` はメソッドチェーン用の部分上書き API で、書き込み前のクリアを行いません。

## List 検索

default feature の `list` が有効な場合、`{StructName}List` が生成されます。

```rust
let mut list = UserList::new();
let id = list.insert(user);

let found = list.try_find_by(UserField::Id, b"00000001")?;
let first = list.try_first_by(UserField::Id, b"00000001")?;
let by_id = list.get(id);
```

フィールド幅を呼び出し側で指定したい場合は、互換 API として `find_by<const N: usize>` / `first_by<const N: usize>` も使えます。通常は、フィールド enum から幅を判断する `try_find_by` / `try_first_by` を優先します。

先頭一致で検索したい場合は `try_find_by_prefix` / `try_first_by_prefix` を使います。

## Reader / Writer

`Reader` は固定長レコードを順に読み込みます。レコード直後の `\n`、`\r`、`\r\n`、`,` は読み飛ばします。

```rust
let mut reader = Reader::<_, User>::new(source)
    .with_sequence_check([UserField::Id]);

let mut reader = Reader::<_, User>::new(source)
    .with_sequence_check_options([UserField::Id], false);
```

シーケンスチェックでは、前回レコードより今回レコードが小さい場合に `Error::SequenceError` を返します。同一キーはデフォルトで許可されます。

`Writer` は `to_bytes` したレコードを書き出し、レコード末尾に区切りを付けます。

`RecordSeparator` で、レコードごとに書き出す区切りを選べます。

```rust
let mut writer = Writer::new(output)
    .with_separator(RecordSeparator::Crlf);

let mut csv_like_writer = Writer::new(output)
    .with_separator(RecordSeparator::Comma);

let mut cr_writer = Writer::new(output)
    .with_separator(RecordSeparator::Cr);
```

## Feature Flags

- `list`: default feature。`{StructName}List` と検索インデックス API を生成します。
- `unchecked`: unsafe なゼロコピー系 API を追加生成します。

レコード本体とフィールド操作だけを生成したい場合は、default feature を外します。

```toml
[dependencies]
fixed-record = { version = "0.1", default-features = false }
```

`unchecked` feature では、`as_bytes_unchecked` / `parse_unchecked` / `from_bytes_unchecked` / `from_str_unchecked` が生成されます。これらは構造体のメモリレイアウトが固定長レコードのバイト配置と一致していることを呼び出し側が保証する必要があります。

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

公開時の利用者向け入口は `fixed-record` です。`fixed-record-macros` は proc macro 実装用 crate として扱います。

## ライセンス

このプロジェクトは MIT No Attribution License (MIT-0) で公開します。詳細は [LICENSE](LICENSE) を参照してください。
