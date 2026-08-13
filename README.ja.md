# fixed-record

`fixed-record` は、Rust の struct 定義から固定長レコード用のパース、生成、フィールド操作、Reader/Writer、検索用 List API を生成するライブラリです。

利用者は通常、`fixed-record` だけを依存に追加します。`fixed-record-macros` は `fixed-record` から再エクスポートされる内部実装用の proc macro crate です。

英語版は [README.md](README.md) にあります。

API ドキュメントは <https://tomi-912.github.io/fixed-record/> で公開しています。

## なぜ fixed-record?

固定長レコードは、銀行系ファイル、ホスト連携、レガシーなバッチ処理、行単位のデータ交換など、各フィールドのバイト幅が決まっている場面でよく出てきます。`fixed-record` は、そのレイアウトを Rust の struct に一度だけ書き、周辺の定型コードを生成します。

主なメリットは次のとおりです。

- `Fixed<N>` フィールドでバイトレイアウトを一度だけ定義できます。
- builder、getter、setter、parse、serialize、メタ情報が生成されます。
- フィールドを private にしたまま、生成メソッドを公開 API として使えます。
- `Reader` / `Writer` でレコード列を読み書きできます。
- default feature の `list` が有効な場合、生成された `{StructName}List` で検索やソートができます。
- unsafe を書かずに、`zerocopy` traits による安全な zerocopy view を使えます。

## レコード定義の形

`#[fixed_record]` は named fields の struct に対応し、全フィールドが `Fixed<N>` である必要があります。

```rust
#[fixed_record]
struct Payment {
    bank_code: Fixed<4>,
    account_no: Fixed<7>,
    amount: Fixed<8>,
}
```

`Fixed<N>` 以外のフィールド、tuple struct、0 byte のフィールド、負の幅、literal ではない幅は compile error になります。

## インストール

crates.io に公開する前は、Git repository 依存として追加します。

```toml
[dependencies]
fixed-record = { git = "https://github.com/tomi-912/fixed-record.git" }
```

crates.io 公開後は、version 指定の依存として追加できます。

```toml
[dependencies]
fixed-record = "0.1"
```

## クイックスタート

```rust
use fixed_record::prelude::*;

#[fixed_record]
struct User {
    id: Fixed<8>,
    name: Fixed<16>,
    age: Fixed<3>,
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
- レコード struct と同じ可視性の `{StructName}Field` enum
- `TOTAL_LEN`、フィールド長、オフセットなどのメタ情報
- `builder`、`with_*`、`try_with_*_int`、`with_*_int_truncated`
- `parse` / `parse_str` / `to_bytes`
- `get_field_*` / `set_field_*` などの動的フィールド操作
- `apply_*` 系の一括流し込み
- `FixedRecord` trait 実装
- `Reader` / `Writer` との連携
- `list` feature が有効な場合、レコード struct と同じ可視性の `{StructName}List` による末尾追加、位置指定挿入、検索、範囲検索、索引対応の変更、削除、`pop`、ソート
- `compare_all_fields` / `compare_by_fields` / `to_dump_string`

## List 検索と変更

`{StructName}List` は default feature の `list` で生成される補助機能です。レコードを `Vec<Box<Record>>` として保持します。マクロは、`BTreeMap<Fixed<8>, Vec<usize>>` や `BTreeMap<Fixed<16>, Vec<usize>>` のような型付き索引をフィールドごとに持つ、非公開の `{StructName}ListIndices` 型も生成します。この内部型は定義モジュールの外から名前を指定できません。キーはレコードのフィールドから直接コピーされ、`Vec<u8>` の割り当ては行いません。値には一致する全レコードの現在の index が入ります。ソート時は vector 内の Box が移動し、Box の先にあるレコード本体は移動しません。

失敗する可能性がある検索メソッドには一貫して `try_` prefix を付け、付かない検索メソッドは `Option`、`Vec`、または iterator を直接返します。完全一致検索は全レコードを走査せず、フィールド索引を直接参照します。`try_find_by` は選択フィールドと同じ幅の入力だけを受け付け、短い場合は `Error::TooShort`、長い場合は `Error::FieldOverflow` を返します。prefix、padding を考慮した検索、range、ソート済み参照には順序付き索引の範囲を使います。`try_find_range_by` は境界が `AsRef<[u8]>` を実装する標準 Rust range を受け取ります。短い開始境界は末尾を `0x00`、短い終了境界は末尾を `0xFF` で補うため、後続バイトは任意になります。いずれかの境界が長すぎる場合は `FieldOverflow`、開始が終了より大きい場合は `InvalidRange` を返します。`push(record)` は末尾へ1件追加して索引登録します。`insert(index, record)` は指定位置へ挿入し、その位置以降の既存索引IDを1つ繰り上げます。`index > len()` の場合は変更せず `false` を返します。`update` は対象項目だけを変更します。`remove` は削除対象を索引から外し、後ろのIDを1つ繰り下げます。全体の順序が変わる `sort` と `sort_by` だけが索引を再構築します。`pop` は他の index が変化しないため、末尾レコードの索引項目だけを削除します。フィールドキーのコピーとレコード index を保持する追加メモリと引き換えに、繰り返し検索を高速化する設計です。

通常の `iter_mut` で mutable なレコード参照を公開すると、任意の変更によってフィールド索引が古くなるため生成しません。`for_each_mut` は mutable 参照を callback 内に限定し、終了後に全索引を再構築します。`try_edit_by*`、`try_edit_range_by`、`try_edit_first_by*` は、対応する `try_find*` と同じ検索処理から非公開の現在 index を取得し、一致レコードだけを変更して影響する索引項目を修復します。index は公開せず、変更件数または変更有無を返します。callback が unwind した場合も drop guard が索引を修復します。

List の ID は現在の vector index です。補助機能としては素直ですが、`remove`、`sort`、`sort_by` の後は index が変わる可能性があります。

record parsing、field access、`Reader`、`Writer` だけでよい場合は default feature を外すと、List 型は生成されません。

```rust
let mut list = UserList::new();
list.push(user);

let exact = list.try_find_by(UserField::Id, b"00000001")?;
let first_exact = list.try_first_by(UserField::Id, b"00000001")?;
let padded = list.try_find_by_padded(UserField::Id, b"00000001")?;
let ages_in_20s_and_30s = list.try_find_range_by(UserField::Age, b"02"..=b"03")?;
let first_padded = list.try_first_by_padded(UserField::Id, b"00000001")?;

let edited = list.try_edit_by(UserField::Id, b"00000001", |user| {
    user.set_field_str(UserField::Name, "Sato");
})?;

list.for_each_mut(|user| user.set_field_str(UserField::Age, "026"));
```

`try_find_by` / `try_first_by` はフィールドと同じ幅を要求します。`try_find_by_padded` / `try_first_by_padded` は、残りのフィールドバイトがスペースまたは `0x00` なら短い入力を受け付けます。`Result` を使わず索引上で最小のフィールド値を取得する場合は `first_sorted_by(field)` を使います。

短い先頭一致で検索したい場合は `try_find_by_prefix` / `try_first_by_prefix` を使います。

変更用には `try_edit_by`、`try_edit_by_padded`、`try_edit_by_prefix`、`try_edit_range_by`、`try_edit_first_by`、`try_edit_first_by_padded`、`try_edit_first_by_prefix` があります。複数件用は変更件数、先頭1件用は変更できたかを返します。

## Reader / Writer

`Reader` は固定長レコードを順に読み込みます。`Reader::new` は `Writer::new` と同じく、レコード後ろに LF (`\n`) がある入力を想定します。別の区切り、または区切りなしの入力を読む場合は明示的に指定します。区切りを設定した場合は、最終レコードの後ろにもその区切りが必要です。

```rust
let mut reader = Reader::<_, User>::new(source)
    .with_sequence_check([UserField::Id]);

let mut reader = Reader::<_, User>::new(source)
    .with_separator(RecordSeparator::None)
    .with_sequence_check_options([UserField::Id], false);
```

シーケンスチェックでは、前回レコードより今回レコードが小さい場合に `Error::SequenceError` を返します。同一キーはデフォルトで許可されます。

区切りなしでレコードが連続する入力には `RecordSeparator::None` を使います。`Writer::new` は `to_bytes` したレコードを書き出し、デフォルトでは LF をレコード末尾に付けます。

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

- `list`: default feature。補助機能の `{StructName}List` を生成します。

レコード本体、フィールド操作、parse、`Reader`、`Writer` だけを使いたい場合は、default feature を外します。

```toml
[dependencies]
fixed-record = { version = "0.1", default-features = false }
```

## Zerocopy Support

`#[fixed_record]` のレコードは常に `#[repr(C)]` が付き、`zerocopy::FromBytes`、`zerocopy::IntoBytes`、`zerocopy::Immutable`、`zerocopy::KnownLayout` を derive します。zerocopy traits は `fixed_record::prelude` から再エクスポートされるため、利用側で `zerocopy` を直接 dependency に追加しなくても安全な zerocopy API を使えます。

```rust
use fixed_record::prelude::*;

#[fixed_record]
struct User {
    id: Fixed<8>,
    name: Fixed<16>,
    age: Fixed<3>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw = b"00000001Tanaka          025";
    let user = User::ref_from_bytes(raw)?;

    assert_eq!(user.id(), b"00000001");
    assert_eq!(user.as_bytes(), raw);
    assert_eq!(user.as_str()?, "00000001Tanaka          025");
    Ok(())
}
```

`ref_from_bytes` は入力長がちょうど1レコード分である必要があります。先頭1レコードの後ろに余りバイトが続く可能性がある場合は、`ref_from_bytes_prefix` を使います。

```rust
use fixed_record::prelude::*;

#[fixed_record]
struct User {
    id: Fixed<8>,
    name: Fixed<16>,
    age: Fixed<3>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw = "00000001Tanaka          025rest";
    let user = User::ref_from_str_prefix(raw)?;

    assert_eq!(user.id(), b"00000001");
    Ok(())
}
```

入力がUTF-8文字列で、バイト長がちょうど1レコード分の場合は `ref_from_str` を使えます。

```rust
use fixed_record::prelude::*;

#[fixed_record]
struct User {
    id: Fixed<8>,
    name: Fixed<16>,
    age: Fixed<3>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let user = User::ref_from_str("00000001Tanaka          025")?;

    assert_eq!(user.name_str()?, "Tanaka          ");
    Ok(())
}
```

## フィールド初期化

`set_field_*` は、書き込み前に `CLEAR_BYTE` で対象フィールドをクリアします。未指定時の `CLEAR_BYTE` は半角スペース (`0x20`) です。

```rust
#[fixed_record(clear_byte = ZERO)]
struct User {
    id: Fixed<8>,
}
```

`set_field_*`、`builder()`、`default()`、`cleared()` で `0x00` 初期化したい場合は、`clear_byte = ZERO` または `clear_byte = 0` を指定します。

明示的な初期化には、`zeroed()`、`spaced()`、`cleared()` を使えます。`zeroed()` は常に `0x00`、`spaced()` は常に半角スペース、`cleared()` は `CLEAR_BYTE` で初期化します。

既存の後続バイトを残したい場合は、`set_field_bytes_no_clear` / `set_field_str_no_clear` を使います。`with_*` はメソッドチェーン用の部分上書き API で、書き込み前のクリアを行いません。

## Examples

```bash
cargo run -p fixed-record-basic-example --bin fixed_record_usage
cargo run -p fixed-record-basic-example --bin macro_reexport
cargo run -p fixed-record-no-list-example
```

## ライセンス

このプロジェクトは MIT No Attribution License (MIT-0) で公開します。詳細は [LICENSE](LICENSE) を参照してください。
