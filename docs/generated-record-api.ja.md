# 生成レコードstruct API一覧

この文書は、現在の `0.1.0` 系コードで `#[fixed_record]` を付けたstructから利用できるAPIを一覧化したものです。`1.0.0` に向けた命名レビューの現状把握に使います。ここにある名前は変更前のものであり、推奨名や最終仕様を示すものではありません。

## 前提となる定義例

```rust
use fixed_record::prelude::*;

#[fixed_record]
pub struct Payment {
    pub account: Fixed<8>,
    pub amount: Fixed<10>,
}
```

この例では、元の `Payment` にメソッドとtrait実装が追加され、フィールド識別用の `PaymentField` が生成されます。default featureの `list` が有効な場合は、別の型として `PaymentList` も生成されます。

以降の `<field>` は元のフィールド名を表します。上の例では `account` または `amount` です。`<FIELD>` はフィールド名を大文字にした定数名部分です。

## 定数

| API | 内容 |
| --- | --- |
| `Payment::TOTAL_LEN` | 全フィールドを合計したレコードのバイト長 |
| `Payment::CLEAR_BYTE` | `set_field_*`や`cleared()`がクリアに使うバイト |
| `Payment::FIELD_SIZE_<FIELD>` | 各フィールドのバイト長。例: `FIELD_SIZE_ACCOUNT` |

## 生成と初期化

| API | 戻り値 | 現在の挙動 |
| --- | --- | --- |
| `Payment::zeroed()` | `Payment` | 全フィールドを`0x00`で初期化する |
| `Payment::spaced()` | `Payment` | 全フィールドをspace (`0x20`) で初期化する |
| `Payment::cleared()` | `Payment` | 全フィールドを`CLEAR_BYTE`で初期化する |
| `Payment::default()` | `Payment` | `cleared()`と同じ |
| `Payment::builder()` | `Payment` | `cleared()`で初期化した値を返す |
| `record.build()` | `Payment` | 値を変更せず、そのまま返す |

`builder()`は専用builder型を返すのではなく、`Payment`そのものを返します。`build()`も変換や検証をせず、`self`をそのまま返します。

## フィールド固有の読み取り

各フィールドについて次のメソッドが生成されます。

| パターン | 戻り値 | 現在の挙動 |
| --- | --- | --- |
| `record.<field>()` | `&[u8]` | フィールド全体を生バイト列として借用する |
| `record.<field>_str()` | `Result<&str, Error>` | UTF-8として検証して借用する |

定義例では、次のメソッドが使えます。

```rust
record.account();
record.account_str();
record.amount();
record.amount_str();
```

元のフィールドが `pub` の場合は `record.account` のように `Fixed<N>` 自体へ直接アクセスでき、`Fixed::as_bytes()`、`as_str()`、`write_bytes()`、`fill()`、`fill_zero()`、`fill_space()`なども利用できます。フィールドが非公開の場合、この直接アクセスは定義したmodule内に限られますが、生成accessorの可視性はrecord structの可視性に従います。

## フィールド固有のbuilder形式メソッド

各メソッドは `self` を受け取り、更新後の `Self` を返します。

| パターン | 入力 | 現在の挙動 |
| --- | --- | --- |
| `with_<field>(value)` | `&str` | 先頭から上書きする。事前クリアをせず、短い入力では後続バイトが残る |
| `with_<field>_int(value)` | `i64` | ゼロ埋め数値を書き込む。幅超過時は切り詰めてstderrへ警告する |
| `try_with_<field>_int(value)` | `i64` | ゼロ埋め数値を書き込む。幅超過時は`Error::FieldOverflow`を返す |
| `with_<field>_int_truncated(value)` | `i64` | `with_<field>_int`の実処理。幅超過時は先頭側だけ残してstderrへ警告する |
| `with_<field>_int_signed(value)` | `i64` | `+`または`-`付きのゼロ埋め数値を書き込む。幅超過時は切り詰める |
| `try_with_<field>_int_signed(value)` | `i64` | 符号付き数値を書き込む。幅超過時は`Error::FieldOverflow`を返す |
| `with_<field>_int_signed_truncated(value)` | `i64` | 符号付き数値を書き込み、幅超過時は先頭側だけ残してstderrへ警告する |

例:

```rust
let record = Payment::builder()
    .with_account("A0000123")
    .try_with_amount_int(1500)?
    .build();
```

## フィールド情報

| API | 戻り値 | 内容 |
| --- | --- | --- |
| `Payment::size_of(field)` | `usize` | 指定フィールドのバイト長 |
| `Payment::offset_of(field)` | `usize` | レコード先頭から指定フィールドまでのoffset |
| `Payment::name_of(field)` | `&'static str` | 元のフィールド名 |
| `Payment::all_fields()` | `&'static [PaymentField]` | 定義順の全フィールド |
| `Payment::byte_len_fields(fields)` | `usize` | 指定フィールド群の合計バイト長 |
| `record.byte_len()` | `usize` | レコード全体のバイト長。`TOTAL_LEN`と同じ |

## レコードのparseと変換

| API | 戻り値 | 現在の挙動 |
| --- | --- | --- |
| `Payment::parse(bytes)` | `Result<Payment, Error>` | 先頭`TOTAL_LEN` byteをコピーする。短い場合は`TooShort`、余りは無視する |
| `Payment::parse_str(text)` | `Result<Payment, Error>` | `str`をbyte列として`parse`する |
| `record.to_bytes()` | `[u8; TOTAL_LEN]` | レコード全体を固定長arrayへコピーする |
| `record.as_str()` | `Result<&str, Error>` | レコード全体をUTF-8として借用する |

`FixedRecord` trait経由でも `parse` と `to_bytes` を利用できます。ただし、traitの `to_bytes` は `Vec<u8>` を返すため、同名のinherent methodとは戻り値が異なります。

### `FixedRecord` traitのメソッド

通常は生成structのinherent methodを使いますが、genericな処理では次のtrait APIを利用できます。

| API | 戻り値 | 内容 |
| --- | --- | --- |
| `<Payment as FixedRecord>::parse(bytes)` | `Result<Payment, Error>` | inherent methodの`Payment::parse`へ委譲する |
| `<Payment as FixedRecord>::to_bytes(&record)` | `Vec<u8>` | 固定長表現を`Vec<u8>`としてコピーする |
| `<Payment as FixedRecord>::field_name(field)` | `&'static str` | `Payment::name_of`へ委譲する |
| `<Payment as FixedRecord>::field_bytes(&record, field)` | `&[u8]` | `record.get_field_bytes`へ委譲する |

## zero-copy参照

### crate独自の補助メソッド

| API | 戻り値 | 現在の挙動 |
| --- | --- | --- |
| `Payment::ref_from_str(text)` | `Result<&Payment, Error>` | 長さが`TOTAL_LEN`と完全一致する文字列をzero-copyで借用する |
| `Payment::ref_from_bytes_prefix(bytes)` | `Result<&Payment, Error>` | 先頭`TOTAL_LEN` byteをzero-copyで借用し、後続byteを許容する |
| `Payment::ref_from_str_prefix(text)` | `Result<&Payment, Error>` | 文字列の先頭`TOTAL_LEN` byteをzero-copyで借用する |

### zerocopy traitから利用できるメソッド

生成structは `FromBytes`、`IntoBytes`、`Immutable`、`KnownLayout` をderiveします。traitがscopeにある場合、zerocopy 0.8.56では主に次を呼び出せます。

| trait | API群 | 内容 |
| --- | --- | --- |
| `FromBytes` | `ref_from_bytes`、`ref_from_prefix`、`ref_from_suffix` | byte sliceから共有参照を作る |
| `FromBytes` | `mut_from_bytes`、`mut_from_prefix`、`mut_from_suffix` | mutable byte sliceから可変参照を作る |
| `FromBytes` | `read_from_bytes`、`read_from_prefix`、`read_from_suffix` | byte sliceから所有値へコピーする |
| `FromBytes` | `read_from_io` | readerから所有値を読み取る |
| `IntoBytes` | `as_bytes`、`as_mut_bytes` | レコード全体をbyte sliceとして借用する |
| `IntoBytes` | `write_to`、`write_to_prefix`、`write_to_suffix` | 既存byte sliceへ書き込む |
| `IntoBytes` | `write_to_io` | writerへ書き込む |

`FromBytes`には旧形式の `ref_from`、`mut_from`、`read_from` も存在します。また、動的サイズ型向けの `*_with_elems` 系もありますが、通常の固定長生成structで使う中心的なAPIではありません。

## 動的なフィールド読み取り

`PaymentField`を引数にして、実行時に対象フィールドを選びます。

| API | 戻り値 | 現在の挙動 |
| --- | --- | --- |
| `record.get_field_bytes(field)` | `&[u8]` | 生バイト列を借用する |
| `record.get_field_str(field)` | `Result<&str, Error>` | UTF-8として借用する |
| `record.get_field_trimmed(field)` | `Result<&str, Error>` | 前後のspaceとNULを除いた文字列を借用する |
| `record.get_field_string_trimmed(field)` | `Result<String, Error>` | trim後の所有`String`を作る |
| `record.get_field_as::<T>(field)` | `Result<T, Error>` | trim後に`FromStr`で任意の型へparseする |

## 動的なフィールド変更

| API | 現在の挙動 |
| --- | --- |
| `record.fill_field_zero(field)` | 指定フィールドを`0x00`で埋める |
| `record.fill_field_space(field)` | 指定フィールドをspaceで埋める |
| `record.fill_field_clear(field)` | 指定フィールドを`CLEAR_BYTE`で埋める |
| `record.set_field_bytes(field, data)` | `CLEAR_BYTE`でクリアしてからbyte列を書き込む |
| `record.set_field_bytes_no_clear(field, data)` | クリアせず先頭からbyte列を上書きする |
| `record.set_field_str(field, value)` | `CLEAR_BYTE`でクリアしてから文字列を書き込む |
| `record.set_field_str_no_clear(field, value)` | クリアせず先頭から文字列を上書きする |
| `record.fill_zero()` | 全フィールドを`0x00`で埋める |
| `record.fill_space()` | 全フィールドをspaceで埋める |

入力がフィールド幅を超えた場合、`set_field_*`系は先頭のフィールド幅分だけを書き込み、エラーを返しません。

## 一括適用

| API | 現在の挙動 |
| --- | --- |
| `record.apply(closure)` | `&mut Self`をclosureへ渡し、変更後の`Self`を返す |
| `record.apply_bytes(data)` | 先頭フィールドから定義順にbyte列を流し込む |
| `record.apply_str(value)` | 文字列をbyte列として先頭フィールドから流し込む |
| `record.apply_bytes_from(field, data)` | 指定フィールドから定義順にbyte列を流し込む |
| `record.apply_str_from(field, value)` | 指定フィールドから定義順に文字列を流し込む |

`apply_bytes*`と`apply_str*`は各フィールドへ書き込む前に`CLEAR_BYTE`でクリアします。入力が途中で終わった後のフィールドは変更しません。

## 比較とdump

| API | 戻り値 | 内容 |
| --- | --- | --- |
| `record.compare_all_fields(other)` | `Ordering` | 全フィールドを定義順に比較する |
| `record.compare_by_fields(other, fields)` | `Ordering` | 指定されたフィールドの優先順で比較する |
| `record.dump()` | `()` | 全フィールドを標準出力へ表示する |
| `record.to_dump_string()` | `String` | 全フィールドをdump形式の文字列にする |

## `PaymentField`で使えるメソッド

| API | 戻り値 | 内容 |
| --- | --- | --- |
| `field.as_str()` | `&'static str` | 元のフィールド名 |
| `field.size()` | `usize` | フィールドの定義バイト長 |

`PaymentField`は `Debug`、`Clone`、`Copy`、`PartialEq`、`Eq`、`PartialOrd`、`Ord`、`Hash` を実装します。

## 自動実装されるtrait

生成structには次が実装されます。

- `Debug`
- `Clone`
- `Copy`
- `PartialEq`
- `Eq`
- `Default`
- `FixedRecord`
- `zerocopy::FromBytes`
- `zerocopy::IntoBytes`
- `zerocopy::Immutable`
- `zerocopy::KnownLayout`
- `RecordWithList` (`list` feature有効時)

`PartialOrd`、`Ord`、`Hash` は生成struct自体には自動実装されません。

## struct本体ではなく関連型に属するAPI

次は `Payment` のメソッドではありません。

- `PaymentField`: フィールドを指定するenum
- `PaymentList`: レコード集合の検索、編集、sort、I/Oを提供する型
- `Reader<R, Payment>`: 複数レコードを読み取るiterator
- `Writer<W>`: レコードを書き込む型

特にList検索の `try_find_by*`、編集の `try_edit_by*`、`sort_by`、`read_from`などは `PaymentList` のAPIです。生成structの命名レビューとは分けて扱います。

## 命名レビューで確認したい点

この一覧から、少なくとも次の重複・曖昧さをIssue #1〜#7で検討します。

- `zeroed`、`spaced`、`cleared`、`default`、`builder`の役割が直感的に区別できるか。
- `builder()`が専用builder型ではなくrecord本体を返すことが名前から分かるか。
- `build()`が何も検証しないidentity操作として必要か。
- `parse`が余りbyteを許容する一方、`ref_from_bytes`は完全一致を要求することが名前から分かるか。
- inherent methodとtrait methodに、同名で戻り値が違う`to_bytes`が存在してよいか。
- field固有の`<field>()`と動的な`get_field_bytes()`で動詞が統一されていない点をどうするか。
- `with_<field>()`だけが事前クリアをせず、`set_field_str()`は事前クリアする違いが伝わるか。
- `int`、`int_signed`、`truncated`、`try_`の並びが予測しやすいか。
- `apply`がclosure操作とbyte列の流し込みの両方を表してよいか。
- `dump()`のようなdebug用APIを安定した公開APIに含めるか。
