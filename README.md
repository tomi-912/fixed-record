# fixed_record_project

固定長レコードライブラリを、通常ライブラリ・proc macro・サンプルアプリに分けたワークスペース版です。

`fixed_record_main` と `fixed_record_macros` と `app` の3メンバーで構成されています。

## 構成

- `fixed_record_main/`: 利用者が依存する通常ライブラリです。`Fixed<N>`、`Error`、`prelude`、マクロの再エクスポートを持ちます。
- `fixed_record_macros/`: `#[fixed_record_main]` attribute macro を定義する proc macro クレートです。
- `app/`: ライブラリの使い方と挙動確認用のサンプルアプリです。

## 役割

このワークスペースは、proc macro を別クレートに切り出した設計を試すための版です。

`#[fixed_record_main]` を付けた構造体から、次のような機能を生成します。

- 基本 derive を付けたレコード構造体
- `{StructName}Field` enum
- `TOTAL_LEN`、フィールド長、オフセットなどのメタ情報
- `builder`、`with_*`、`with_*_int`、`with_*_int_signed`
- `parse` / `parse_str` / `to_bytes`
- 動的フィールド取得・更新
- `apply_*` 系の一括流し込み
- `FixedRecord` トレイト
- `Reader` / `Writer`
- `{StructName}List` による挿入、検索、範囲検索、論理削除、`vacuum`、ソート
- `compare_all_fields` / `compare_by_fields` / `to_dump_string`

`set_field_*` が書き込み前にフィールドをクリアするときの値は、デフォルトでは `0x00` です。
`builder()`、`Default`、`cleared()` もこの値で初期化します。

```rust
#[fixed_record_main(clear_byte = SPACE)]
pub struct User {
    pub id: Fixed<8>,
}
```

`unchecked` feature を有効にした場合だけ、追加で次のものを生成します。

- `#[repr(C)]` を付けたレコード構造体
- `as_bytes_unchecked` / `parse_unchecked` / `from_bytes_unchecked` / `from_str_unchecked`


## 使い方

```rust
use fixed_record_main::prelude::*;

#[fixed_record_main]
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
assert_eq!(user.get_field_trimmed(UserField::Name).unwrap(), "Tanaka");
```

数値フィールドで桁あふれを検知したい場合は、`try_with_*_int` / `try_with_*_int_signed` を使います。

```rust
let user = User::builder()
    .try_with_age_int(25)?
    .build();
```

桁あふれを許容して先頭側だけ残したい場合は、`with_*_int_truncated` / `with_*_int_signed_truncated` を使います。切り捨てが発生した場合は stderr に警告を出します。

`set_field_bytes` / `set_field_str` は、書き込み前に `CLEAR_BYTE` でフィールドをクリアします。既存の後続バイトを残したい場合は、`set_field_bytes_no_clear` / `set_field_str_no_clear` を使います。

`with_*` はメソッドチェーン用の部分上書き API です。書き込み前にクリアしないため、短い文字列を書いた場合は後続バイトが残ります。

明示的に初期化したい場合は、`zeroed()`、`spaced()`、`cleared()` を使い分けます。`zeroed()` は常に `0x00`、`spaced()` は常に半角スペース、`cleared()` は `CLEAR_BYTE` で初期化します。

List 検索でフィールド幅を手で指定したくない場合は、`try_find_by` や `try_first_by` を使います。検索値がフィールド幅より短い場合、後続バイトが `0x00` または半角スペースのレコードも一致します。

```rust
let found = list.try_find_by(UserField::Id, b"0001")?;
let first = list.try_first_by(UserField::Id);
```

## 開発メモ

修正方針、作業ルール、現状確認メモは [`DEVELOPMENT.md`](DEVELOPMENT.md) に分けています。

README は利用者向けの使い方を中心に置きます。API や実行方法など、使い方が変わる変更を入れた場合は README も更新します。
