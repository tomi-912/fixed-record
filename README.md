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

## 開発メモ

修正方針、作業ルール、現状確認メモは [`DEVELOPMENT.md`](DEVELOPMENT.md) に分けています。

README は利用者向けの使い方を中心に置きます。API や実行方法など、使い方が変わる変更を入れた場合は README も更新します。
