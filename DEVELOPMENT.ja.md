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
- license: MIT No Attribution License (`MIT-0`)

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
- 通常の作業完了時はコミットまでで止めます。
- push はユーザーから明示的に依頼されたときだけ行います。
- README は公開利用者が最初に読む内容として保ちます。
- 公開前の判断、設計メモ、調査メモ、今後の課題は DEVELOPMENT に追記します。
- doc comment を追加・更新するときは、必ず英語を先、日本語を後の順で併記します。
- README または DEVELOPMENT の内容を変更するときは、英語版と日本語版の両方を更新します。
- `target/` が残っているため、探索時は `rg --glob '!target/**'` などで除外すると追いやすいです。

## Current Status

2026-08-13 時点の確認メモです。

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

- `crates/fixed-record/tests/generated_api.rs`: builder / parse / to_bytes / Reader / Writer / List / zerocopy
- `crates/fixed-record/tests/compile_fail.rs`: proc macro の不正入力、List immutable API、sequence check の compile-fail
- `examples/no-list/tests/compile_fail.rs`: `default-features = false` 時に `{StructName}List` が生成されないことを確認

確認済み:

- `fixed-record` の generated API integration test: 通常 feature で 66 件成功
- `fixed-record` の doctest: 25 件成功

## うまくできている点

- 利用者向け入口が `fixed-record` にまとまっており、利用側は `fixed_record::prelude::*` だけで始められます。
- proc macro crate は `fixed-record-macros` として分離済みで、利用者に直接依存させない方針を取れます。
- `Fixed<N>` は固定長バイト列としての基本操作、UTF-8 参照、ゼロ埋め、スペース埋めが小さくまとまっています。
- `FixedRecord` trait によって、Reader/Writer は生成型に依存しすぎない形になっています。
- `#[fixed_record]` はレコード本体、field enum、メタ情報、builder、parse、フィールド操作、Reader/Writer 連携、optional な List API まで生成できます。
- 生成される field enum と List の可視性は入力レコードの可視性に揃えており、private record から public な補助型が漏れません。
- 生成レコードは常に `#[repr(C)]` になり、`zerocopy::FromBytes`、`IntoBytes`、`Immutable`、`KnownLayout` を derive します。以前の独自 unchecked pointer cast API を、安全な zerocopy trait API に置き換えています。
- `Fixed<N>` 自体も zerocopy 対応済みで、生成レコードは byte array backed なフィールドだけで構成されます。
- ゼロコピー参照 helper は手書き unsafe ではなく `zerocopy` の上に重ねています。`ref_from_bytes_prefix`、`ref_from_str`、`ref_from_str_prefix` は crate 側のエラーに揃えつつ cast は zerocopy に任せます。
- コピーする API (`parse`、`parse_str`、`to_bytes`) と、借用 byte view API (`ref_from_bytes`、`as_bytes`、`as_mut_bytes`) を分けており、所有権の意味が明確です。
- compile-fail test で不正な struct 形状や生成型の可視性を確認し、integration test で byte、string、UTF-8、Reader/Writer、List、zerocopy の挙動を確認しています。
- フィールドの doc comment を生成 enum や accessor に引き継ぐため、生成 API の rustdoc 体験が比較的よいです。
- `fixed-record` の crate-level rustdoc には、レコード定義、builder、parse、動的フィールド操作、一括流し込み helper、Reader/Writer、sequence check、検索可能な List、range 検索、`clear_byte`、`FixedRecord`、feature flag、zerocopy まで、具体例付きの英日ガイドを追加済みです。

## Important Notes

### zerocopy support

以前の `unchecked` feature は削除済みです。生成レコードは常に zerocopy traits を derive し、trait method 経由で安全な zerocopy API を使えます。

重要な挙動:

- `ref_from_bytes` は `zerocopy::FromBytes` の exact-size zerocopy API です。
- `ref_from_bytes_prefix` は後続バイトを許容し、短い入力を `Error::TooShort` に変換します。
- `ref_from_str` / `ref_from_str_prefix` は文字数ではなくバイト幅で判定します。
- `as_bytes` / `as_mut_bytes` は `zerocopy::IntoBytes` の借用 byte view です。`to_bytes` はコピーする API として残します。
- `&mut str` は常に valid UTF-8 である必要がある一方、固定長レコードは byte-oriented なので、mutable string API は意図的に生成していません。

### generated List API

`{StructName}List` の生成は default feature の `list` で制御します。

- default では List API を生成します。
- 生成される List は `Vec<Box<Record>>` としてレコードを保持し、`BTreeMap<Field, BTreeMap<Vec<u8>, Vec<usize>>>` 索引を管理します。内側のキーはフィールドの実バイト列、値は重複値を含む現在の vector index です。完全一致は索引を直接参照し、prefix、padding を考慮した検索、range、ソート済み参照には順序付き索引の範囲を使います。`push` は末尾へ1件追加して索引登録します。位置指定 `insert` は挿入位置以降の既存索引IDを1つ繰り上げ、フィールドキーを再構築せずに新レコードを索引登録します。`update` は対象項目を更新します。`remove` は1件を索引から外して後ろのIDを繰り下げ、フィールドキーは再構築しません。全体の順序が変わる `sort` と `sort_by` は索引を再構築します。`pop` は残る ID が変わらないため、削除した末尾レコードだけを索引から除外します。ソートと途中挿入では Box が移動し、レコード本体は移動しません。
- 選択フィールドの異なる値が `u` 件、一致件数が `k` 件の場合、完全一致検索は全 `n` レコードの走査ではなく `O(log u + k)` です。prefix・range 検索は `O(log u + m + k log k)` で、`m` は走査した異なる索引キー数、`k log k` は現在の List 順序を維持するための ID ソートです。代わりに、フィールドバイト列のコピーと、レコードごと・フィールドごとに1つの `usize` を索引用メモリとして使います。
- `fixed-record` を `default-features = false` で依存すると、レコード本体とフィールド操作だけを生成します。

### Reader separators

`Reader::new` と `Writer::new` はどちらもデフォルトで LF (`\n`) をレコード区切りにします。入力フォーマットが別の区切り、または区切りなしの場合は、呼び出し側が `Reader::with_separator` で指定します。区切りを設定した場合は、最終レコードを含むすべてのレコード後ろにその区切りが必要です。

### test placement

主要な挙動は `crates/fixed-record/tests/` に寄せています。`examples/basic` は利用者が読んで動かせる実行サンプルとして薄く保ち、API 保証は library crate 側で持ちます。

`default-features = false` の compile-fail は、同一 package 内の integration test では依存 feature を切り替えにくいため、専用 fixture の `examples/no-list` で検証します。

## Publish Checklist

crates.io 公開前にやること:

1. GitHub repository を `fixed-record` にリネームする。
2. `Cargo.toml` の package metadata を埋める。
3. `MIT-0` の license metadata と LICENSE 本文を確認する。
4. README の install 文言を実際の公開 version に合わせる。
5. README と rustdoc の zerocopy API 表現を最終確認する。
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

1. `Cargo.toml` の残りの公開 metadata を追加する。
2. GitHub repository 名を `fixed-record` に変える。
3. `crates/fixed-record/tests/generated_api.rs` を挙動別ファイルへ分割する。
4. `cargo package --dry-run` で公開パッケージ内容を確認する。
