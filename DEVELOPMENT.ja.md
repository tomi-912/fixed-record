# Development Notes

このファイルは、`fixed-record` の開発方針、公開準備、現状確認、今後の修正候補を置く場所です。

README は利用者向けの導入と使い方に絞ります。内部設計、作業ルール、調査メモ、公開前タスクはこのファイルへ寄せます。

英語版は [DEVELOPMENT.md](DEVELOPMENT.md) にあります。

## Naming

公開向けの名前は次で統一します。

- 公開 crate: `fixed-record`
- コード上の crate 名: `fixed_record`
- proc macro crate: `fixed-record-macros`
- attribute macro: `#[fixed_record]`
- repository name: `fixed-record` へ変更予定

利用者は `fixed-record` だけを `[dependencies]` に追加します。`fixed-record-macros` は `fixed-record` から再エクスポートされる内部実装用 crate として扱います。

生成コード内の参照は `::fixed_record::...` に統一済みです。

## Workspace Layout

```text
crates/
  fixed-record/
  fixed-record-macros/
examples/
  basic/
  no-list/
```

- `crates/fixed-record/`: 利用者向けの本体 crate。`Fixed<N>`、`Error`、`Reader`、`Writer`、`FixedRecord`、`prelude`、`#[fixed_record]` の再エクスポートを提供します。
- `crates/fixed-record-macros/`: `#[fixed_record]` を実装する proc macro crate です。
- `examples/basic/`: 利用者向けの薄い実行サンプルです。
- `examples/no-list/`: `default-features = false` で List 生成を外す構成を検証する fixture です。

## Working Rules

- ソースやドキュメントを変更したら、原則としてその変更をコミットします。
- push は明示的に依頼されたときだけ行います。
- README は公開利用者が最初に読む内容として保ちます。
- 公開前の判断、設計メモ、調査メモ、今後の課題は DEVELOPMENT に追記します。
- `target/` が残っているため、探索時は `rg --glob '!target/**'` などで除外すると追いやすいです。

## Current Status

2026-08-11 時点の確認メモです。

成功確認済み:

```bash
cargo fmt --all --check
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo run -p fixed-record-basic-example --bin fixed_record_usage
cargo run -p fixed-record-basic-example --bin macro_reexport
cargo run -p fixed-record-no-list-example
```

テスト配置:

- `crates/fixed-record/tests/generated_api.rs`: builder / parse / to_bytes / Reader / Writer / List / Index / `unchecked`
- `crates/fixed-record/tests/compile_fail.rs`: proc macro の不正入力、List immutable API、sequence check の compile-fail
- `examples/no-list/tests/compile_fail.rs`: `default-features = false` 時に `{StructName}List` が生成されないことを確認

確認済み:

- `fixed-record` の integration test: 通常 feature で 41 件成功
- `cargo test --all-features` では `unchecked` feature 用テストを含めて 42 件成功
- `fixed-record` の doctest: 8 件成功

## What Works Well

- 利用者向け入口が `fixed-record` にまとまっており、利用側は `fixed_record::prelude::*` だけで始められます。
- proc macro crate は `fixed-record-macros` として分離済みで、利用者に直接依存させない方針を取れます。
- `Fixed<N>` は固定長バイト列としての基本操作、UTF-8 参照、ゼロ埋め、スペース埋めが小さくまとまっています。
- `FixedRecord` trait によって、Reader/Writer は生成型に依存しすぎない形になっています。
- `#[fixed_record]` はレコード本体、field enum、メタ情報、builder、parse、フィールド操作、Reader/Writer 連携、List/Index 系まで生成できます。
- フィールドの doc comment を生成 enum や accessor に引き継ぐため、生成 API の rustdoc 体験が比較的よいです。

## Important Notes

### unchecked feature

`unchecked` feature で生成される `as_bytes_unchecked` / `parse_unchecked` / `from_bytes_unchecked` / `from_str_unchecked` は、構造体のメモリレイアウトに依存します。

現在の `Fixed<N>` は中身が `[u8; N]` なので alignment が 1 です。そのため、全フィールドが `Fixed<N>` の現在の制約下では期待通り動きやすいです。ただし、Rust の構造体は一般には padding や alignment の影響を受けます。

公開時は次のどちらかに寄せます。

- `unchecked` を明示 feature かつ上級者向けとして残し、README と rustdoc に安全条件を強く書く。
- 初回公開では `unchecked` を外す、または experimental 扱いにする。

今のおすすめは、通常 API を主役にして、`unchecked` は明示 feature の上級者向けとして扱うことです。

### generated List API

`{StructName}List` / Index 系の生成は default feature の `list` で制御します。

- default では従来どおり生成します。
- `fixed-record` を `default-features = false` で依存すると、レコード本体とフィールド操作だけを生成します。

### test placement

主要な挙動は `crates/fixed-record/tests/` に寄せています。`examples/basic` は利用者が読んで動かせる実行サンプルとして薄く保ち、API 保証は library crate 側で持ちます。

`default-features = false` の compile-fail は、同一 package 内の integration test では依存 feature を切り替えにくいため、専用 fixture の `examples/no-list` で検証します。

## Publish Checklist

crates.io 公開前にやること:

1. GitHub repository を `fixed-record` にリネームする。
2. `Cargo.toml` の package metadata を埋める。
3. LICENSE を追加する。
4. README の install 文言を実際の公開 version に合わせる。
5. `unchecked` feature の扱いを確定する。
6. `cargo package --dry-run -p fixed-record-macros` を通す。
7. `cargo package --dry-run -p fixed-record` を通す。
8. CI で次を必須にする。

```bash
cargo fmt --all --check
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

metadata 候補:

- `description`
- `license` または `license-file`
- `repository`
- `readme`
- `keywords`
- `categories`
- `exclude` / `include`

`fixed-record-macros` の description には、利用者が直接依存する crate ではなく `fixed-record` から使われる proc macro 実装用 crate であることを明記します。

## Next Recommended Work

1. `Cargo.toml` の公開 metadata と LICENSE を追加する。
2. GitHub repository 名を `fixed-record` に変える。
3. `unchecked` feature の安全条件を rustdoc と README にさらに明記する。
4. `crates/fixed-record/tests/generated_api.rs` を挙動別ファイルへ分割する。
5. `cargo package --dry-run` で公開パッケージ内容を確認する。
