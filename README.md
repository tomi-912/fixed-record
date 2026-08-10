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

- `#[repr(C)]` と基本 derive を付けたレコード構造体
- `{StructName}Field` enum
- `TOTAL_LEN`、フィールド長、オフセットなどのメタ情報
- `builder`、`with_*`、`with_*_int`、`with_*_int_signed`
- `parse` / `parse_str`
- `as_bytes` / `from_bytes` / `from_str`
- 動的フィールド取得・更新
- `apply_*` 系の一括流し込み
- `FixedRecord` トレイト
- `Reader` / `Writer`
- `{StructName}List` による挿入、検索、範囲検索、論理削除、`vacuum`、ソート
- `compare_all_fields` / `compare_by_fields` / `to_dump_string`


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

## 触るときの目安

proc macro 版を本命として整理したい場合は、このワークスペースを基準にするとよさそうです。

今後は、単体版 `fixed_record` にだけ追加されている機能が出てきたら、こちらの `fixed_record_main/` と `fixed_record_macros/` へ移植していく流れが自然です。

また、`target/` が残っているためファイル一覧が大きく見えます。ソースを確認するときは `target/` を除外して見ると追いやすいです。

## 現状確認メモ

2026-08-10 時点で、ソースをざっと読んで `cargo test` と `cargo clippy --all-targets --all-features` を実行した確認メモです。

### 確認結果

- `cargo test` は成功しています。
  - app 側の単体テスト: 12 件成功
  - `fixed_record_main` の doctest: 8 件成功
  - `fixed_record_main` / `fixed_record_macros` の lib test は 0 件
- `cargo clippy --all-targets --all-features` は成功扱いですが、警告があります。
  - `fixed_record_macros/src/helpers.rs`: ネストした `if` を畳めるという警告
  - `fixed_record_main/src/io.rs`: `loop` を `while let` にできるという警告
  - `app/src/main.rs`: test module の後ろに `main` があるという警告
- 実装は大きく、`#[fixed_record_main]` からレコード本体、フィールド enum、メタ情報、builder、parse、動的フィールド操作、Reader/Writer、List/Index 系まで生成されています。

### 良いところ

- workspace が `fixed_record_main`、`fixed_record_macros`、`app` に分かれていて、proc macro クレート分離の形は分かりやすいです。
- `Fixed<N>` は小さくまとまっており、固定長バイト列としての基本操作、UTF-8 参照、ゼロ埋め、スペース埋めが揃っています。
- サンプルアプリ内に、builder、apply、Reader/Writer、List の基本挙動を確認するテストがあります。
- `FixedRecord` trait があり、Reader/Writer 側は生成型に依存しすぎない形になっています。
- フィールドの doc comment を生成 enum や accessor に引き継ぐ作りになっていて、生成 API のドキュメント体験を意識しています。

### 問題点・注意点

#### 重要度高

- 生成コードの `as_bytes` / `parse` / `from_bytes` / `from_str` が `unsafe` なメモリ変換に依存しています。
  - `#[repr(C)]` は付いていますが、構造体に padding が入らないことまでは保証しません。
  - 今は `Fixed<N>` の alignment が 1 なので多くのケースで動きますが、将来フィールド型や実装を変えた時に壊れやすいです。
  - 公開 API として安全にするなら、各フィールドごとにスライスをコピー/連結する実装へ寄せるか、unsafe 前提条件を明文化してテストで守る必要があります。
- `from_bytes` / `from_str` は入力バイト列への参照をそのまま生成型 `&Self` として返します。
  - ゼロコピーとしては速い一方、型レイアウトへの依存が強いです。
  - 文字列や任意 slice から構造体参照を作る API は、利用者から見ると安全そうに見えるため危険度が高いです。
  - `parse` のように所有値へコピーする API を基本にし、ゼロコピー API は別名にして制約を強く書く方がよさそうです。
- `Reader::next` が I/O エラーを握りつぶします。
  - `read_exact` が `UnexpectedEof` 以外のエラーを返しても `None` になります。
  - `fill_buf` のエラーも無視されます。
  - `Iterator<Item = Result<T, Error>>` なのに I/O エラーを呼び出し側へ返せないため、途中の読み取り失敗が正常終了に見えます。
- `Reader::next` は短い末尾レコードを `None` として扱います。
  - ファイル末尾に不完全な固定長レコードがあっても検知できません。
  - 固定長ファイル用途では、これはデータ欠損を見逃す可能性があります。
- proc macro の入力エラーが `panic!` / `expect` 中心です。
  - named struct 以外、`Fixed<N>` 以外、`N` がリテラルでない場合などで、利用者に優しい compile error になりにくいです。
  - `syn::Error::new_spanned(...).to_compile_error()` を返す形にした方が使いやすいです。

#### 重要度中

- `with_*_int_signed` はフィールドサイズが 0 の場合に `#size - 1` で underflow します。
  - `Fixed<0>` を許可するか禁止するか決め、proc macro 側で検査した方がよさそうです。
- `with_*_int` / `with_*_int_signed` は値がフィールド幅を超えてもエラーにならず、最終的に `write_bytes` で先頭側から切り捨てられます。
  - 例: 幅 3 に `12345` を入れると `"123"` になります。
  - 固定長レコードでは桁あふれを検知したい場面が多いので、Result を返す setter も欲しいです。
- `set_field_bytes` は書き込み前にフィールドをクリアしません。
  - 短いデータを書いた場合、残りのバイトは以前の値が残ります。
  - `set_field_str` や `with_*` はスペース埋めしてから書くため、同じ「set」でも挙動が違います。
- `apply_bytes_from` も内部で `set_field_bytes` を使うため、短い入力を既存レコードに流した時に古い値が残ります。
  - `spaced()` から使うテストでは問題が見えにくいです。
- `List` の `find_by<const N: usize>` / `first_by<const N: usize>` は、呼び出し側がフィールド幅と同じ `N` を指定する必要があります。
  - 間違った `N` でもコンパイルは通り、結果が空になるだけです。
  - フィールド enum からサイズを型レベルに持てないためですが、API としては誤用しやすいです。
- `List::remove` は論理削除だけで、index からは `vacuum` まで消えません。
  - 検索結果では `is_deleted` を見て除外しているので基本動作は合っています。
  - ただし大量削除時は index に削除済み ID が残り続け、性能やメモリ効率に影響します。
- 生成される `{StructName}Entry` は private ですが、`{StructName}List` の公開 API は生成量がかなり多いです。
  - レコード定義だけしたい利用者にも List/Index 実装が常に付いてきます。
  - 将来的には feature flag や別 macro に分ける余地があります。

#### 重要度低

- `fixed_record_main` が `fixed_record_macros` に依存して再エクスポートしています。
  - 利用者は `fixed_record_main::prelude::*` だけで使えるので便利です。
  - 一方で通常ライブラリと proc macro の依存関係が常にセットになるため、最小依存にしたい場合は feature 化を検討できます。
- `app` にサンプルとテストがかなり入っています。
  - workspace の検証としては便利ですが、ライブラリの保証としては `fixed_record_main` / `fixed_record_macros` 側にテストが少なく見えます。
  - 主要な挙動は library crate の unit/integration test へ移すと保守しやすいです。
- `Error::AlignmentError` と `Error::ParseError` の doc comment が薄く、利用者がどの API で発生するか追いづらいです。
- README の機能一覧は現在の生成内容とおおむね合っていますが、unsafe ゼロコピーやエラー握りつぶしなどの制約はまだ書かれていませんでした。

### 次に直すなら

1. `Reader` が I/O エラーと不完全レコードを返せるように `Error` を拡張する。
2. 生成コードの `unsafe` を減らし、`parse` / `to_bytes` をフィールド単位の安全なコピー実装にする。
3. `from_bytes` / `from_str` の扱いを見直し、残すなら unsafe 前提条件と API 名を明確にする。
4. proc macro の `panic!` を `syn::Error` に置き換えて compile error を改善する。
5. 桁あふれや `Fixed<0>` の扱いを決め、Result 版 setter または入力検証を追加する。
6. app 側のテストを library crate の integration test へ移す。
7. Clippy 警告を潰して、`cargo clippy -- -D warnings` でも通る状態にする。
