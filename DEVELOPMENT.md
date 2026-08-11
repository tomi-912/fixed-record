# Development Notes

このファイルは、開発方針、作業ルール、現状確認メモ、今後の修正候補を置く場所です。

README は利用者向けの概要と使い方を中心に保ちます。API や実行方法など、利用者の使い方が変わる変更を入れた場合は README も更新します。

## 作業ルール

- ソースやドキュメントを変更したら、原則としてその変更をコミットします。
- push は明示的に依頼されたときだけ行います。
- 修正方針、設計メモ、調査メモ、今後の課題は README ではなくこのファイルに追記します。
- README は使い方、導入、実行例、利用者が最初に知るべき内容に絞ります。
- `target/` が残っているためファイル一覧が大きく見えます。ソースを確認するときは `target/` を除外して見ると追いやすいです。

## 触るときの目安

proc macro 版を本命として整理したい場合は、このワークスペースを基準にするとよさそうです。

今後は、単体版 `fixed_record` にだけ追加されている機能が出てきたら、こちらの `crates/fixed-record/` と `crates/fixed-record-macros/` へ移植していく流れが自然です。

## 現状確認メモ

2026-08-11 時点で、ソースをざっと読んで `cargo test`、`cargo test --all-features`、`cargo clippy --all-targets --all-features` を実行した確認メモです。

### 確認結果

- `cargo test` は成功しています。
  - `fixed-record-basic-example` 側の単体テスト: 41 件成功
  - `fixed-record` の doctest: 8 件成功
  - `fixed-record-no-list-example` の compile-fail test で、`default-features = false` 時に `{StructName}List` が生成されないことを確認しています。
  - `fixed-record` / `fixed-record-macros` の lib test は少なめです。
- `cargo test --all-features` は成功しています。
  - `unchecked` feature 有効時に unsafe 系メソッドが生成されることも example 側で確認しています。
- `cargo clippy --all-targets --all-features -- -D warnings` は成功しています。
- 実装は大きく、`#[fixed_record]` からレコード本体、フィールド enum、メタ情報、builder、parse、動的フィールド操作、Reader/Writer、List/Index 系まで生成されています。

### 良いところ

- workspace が `crates/fixed-record`、`crates/fixed-record-macros`、`examples/basic`、`examples/no-list` に分かれていて、proc macro クレート分離と feature 検証の形は分かりやすいです。
- 利用者向け入口は `fixed-record` に寄せてあり、`fixed-record-macros` は内部実装用 crate として扱えます。
- `Fixed<N>` は小さくまとまっており、固定長バイト列としての基本操作、UTF-8 参照、ゼロ埋め、スペース埋めが揃っています。
- `examples/basic` 内に、builder、apply、Reader/Writer、List の基本挙動を確認するテストがあります。
- `FixedRecord` trait があり、Reader/Writer 側は生成型に依存しすぎない形になっています。
- フィールドの doc comment を生成 enum や accessor に引き継ぐ作りになっていて、生成 API のドキュメント体験を意識しています。

## 問題点・注意点

### 重要度高

- `unchecked` feature を有効にした場合だけ生成される `as_bytes_unchecked` / `parse_unchecked` / `from_bytes_unchecked` / `from_str_unchecked` は、`unsafe` なメモリ変換に依存しています。
  - ここでいう `unsafe` なメモリ変換とは、「構造体をそのままバイト列として見る」「バイト列をそのまま構造体として見る」という処理のことです。
  - Rust の構造体は padding や alignment の都合で、フィールドを単純に前から詰めたメモリ配置になるとは限りません。
  - 今の `Fixed<N>` は中身が `[u8; N]` なので alignment が 1 です。そのため、現在のようにフィールドが全部 `Fixed<N>` だけなら padding が入りにくく、多くのケースでは期待通り動きます。
  - ただし、これは「今たまたま成立している前提」に近いです。将来 `Fixed<N>` の中身を変えたり、macro が `Fixed<N>` 以外のフィールドを許可したり、生成コードに別のフィールドを足したりすると、padding や alignment の問題が表面化する可能性があります。
  - 通常時の `parse` / `to_bytes` はフィールド単位コピーなので、構造体全体のメモリレイアウトに依存しません。
  - unchecked API は `unsafe fn` なので、呼び出し側が「構造体のメモリレイアウトが固定長レコードのバイト配置と完全に一致している」ことを保証する必要があります。
  - 今後さらに安全寄りにするなら、unchecked API が本当に必要か、別 trait に分けるかを検討するとよさそうです。
- `Reader::next` の I/O エラー処理は改善済みです。
  - レコード先頭でまだ 1 バイトも読んでいない EOF は、通常の終端として `None` を返します。
  - レコード途中で EOF になった場合は、`Some(Err(Error::IncompleteRecord { expected, actual }))` を返します。
  - I/O エラーは `Some(Err(Error::Io(e)))` を返します。
  - 改行読み飛ばし中の `fill_buf` エラーも、現在は `Some(Err(Error::Io(e)))` として返します。
- proc macro の入力エラーは `syn::Error::new_spanned(...).to_compile_error()` を返す形へ改善済みです。
  - named struct 以外、`Fixed<N>` 以外、`N` がリテラルでない場合などで、利用者が書いた struct や field の位置を指した compile error を返します。
  - `trybuild` を使った compile-fail test を `examples/basic/tests/ui/` に置いています。
- `Fixed<N>` の `N` は正の整数リテラルだけ許可しています。
  - `with_*_int_signed` の生成コードでは `#size - 1` を使うため、フィールドサイズ 0 を許すと underflow します。
  - `Fixed<0>` は `Fixed<N> length must be greater than 0` という compile error にしています。
  - `Fixed<-1>` のような負数は `Fixed<N> length must not be negative` という compile error にしています。
- 数値 setter の桁あふれは Result 版 API で検知できます。
  - `try_with_*_int` / `try_with_*_int_signed` は、値がフィールド幅を超える場合に `Error::FieldOverflow` を返します。
  - 明示的に切り捨てたい場合は `with_*_int_truncated` / `with_*_int_signed_truncated` を使います。
  - 既存の `with_*_int` / `with_*_int_signed` は互換性のため残し、切り捨て時は stderr に警告を出します。
- `set_field_bytes` / `set_field_str` のクリア挙動は整理済みです。
  - `set_field_*` は書き込み前に `CLEAR_BYTE` でフィールドをクリアします。
  - `CLEAR_BYTE` は `#[fixed_record(clear_byte = SPACE)]` のように指定できます。
  - 未指定時の `CLEAR_BYTE` は `0x00` です。
  - `builder()`、`Default`、`cleared()` は `CLEAR_BYTE` で初期化します。
  - `zeroed()` は常に `0x00`、`spaced()` は常に半角スペースで初期化します。
  - クリアせず先頭から上書きしたい場合は `set_field_bytes_no_clear` / `set_field_str_no_clear` を使います。
  - `with_*` はメソッドチェーン用の部分上書き API として、書き込み前のクリアを行いません。
- `List` の検索 API は `try_find_by` を追加済みです。
  - `find_by<const N: usize>` / `first_by<const N: usize>` は互換性のため残しています。
  - `try_find_by` / `try_first_by` は呼び出し側にフィールド幅 `N` を指定させず、フィールド enum から幅を判断します。
  - 検索値がフィールド幅より長い場合は `Error::FieldOverflow` を返します。
  - 検索値がフィールド幅より短い場合は、後続バイトが `0x00` または半角スペースのレコードも一致します。
  - 指定フィールドの昇順で最初の有効レコードを返す幅指定なし API は `try_first_sorted_by` です。
  - `try_find_by_prefix` / `try_first_by_prefix` は、後続バイトの内容に関係なく先頭一致で検索します。
- `List` の ID 指定 API は `get` / `update` を追加済みです。
  - `get` は論理削除済みレコードを返しません。
  - `update` は `&mut self` でのみ呼べるため、immutable な list ではコンパイル時点で禁止されます。
  - `update` は古い検索インデックスを外し、新しいレコード値で検索インデックスを登録し直します。
- `List::remove` は論理削除と同時に検索インデックスから ID を除外します。
  - `vacuum` は削除済みレコードを `records` / `order` から物理削除する用途として残しています。
- `Reader` のシーケンスチェック API は `with_sequence_check` / `with_sequence_check_options` を追加済みです。
  - 指定フィールド配列をキーにして、前回レコードより今回レコードが小さい場合は `Error::SequenceError` を返します。
  - 同一キーはデフォルト許可で、`with_sequence_check_options(fields, false)` の場合は禁止します。
  - 対象レコードと異なる field enum やフィールド数より長い配列はコンパイルエラーにします。
  - 同じフィールドの重複指定は、通常配列の値なので設定時に検出して panic します。
- `{StructName}List` / Index 系の生成は default feature の `list` で制御します。
  - デフォルトでは従来どおり生成します。
  - `fixed_record` を `default-features = false` で依存すると、レコード本体とフィールド操作だけを生成できます。

### 重要度中

### 重要度低

- `fixed-record` が `fixed-record-macros` に依存して再エクスポートしています。
  - 利用者は `fixed_record::prelude::*` だけで使えるので便利です。
  - 公開時の利用者向け依存は `fixed-record` だけに寄せ、`fixed-record-macros` は内部実装用 crate として扱います。
  - 一方で通常ライブラリと proc macro の依存関係が常にセットになるため、最小依存にしたい場合は feature 化を検討できます。
- `examples/basic` にサンプルとテストがかなり入っています。
  - workspace の検証としては便利ですが、ライブラリの保証としては `fixed-record` / `fixed-record-macros` 側にテストが少なく見えます。
  - 主要な挙動は library crate の unit/integration test へ移すと保守しやすいです。
- `Error::AlignmentError` と `Error::ParseError` の doc comment が薄く、利用者がどの API で発生するか追いづらいです。

## 次に直すなら

1. unchecked API を残すか、別 trait に分けるかを決める。
2. `as_bytes_unchecked` / `parse_unchecked` / `from_bytes_unchecked` / `from_str_unchecked` の安全条件をテストとドキュメントでさらに固める。
3. example 側のテストを library crate の integration test へ移す。
4. package metadata を埋めて `cargo package --dry-run` を通す。

## 公開前のおすすめ方針

### 1. 公開名と crate 構成を決める

利用者体験としては、公開して使わせる名前は 1 つに寄せるのがよいです。

- 公開名は `fixed-record` に寄せます。コード上は `fixed_record` として import されます。
- `fixed-record-macros` は proc macro 実装用 crate として公開が必要になる可能性がありますが、利用者には直接依存させない方針を維持します。
- 生成コード内の参照は `::fixed_record::...` に統一済みです。

### 2. package metadata を整える

crates.io 公開前に、各 `Cargo.toml` の metadata を埋めます。

- `description`
- `license` または `license-file`
- `repository`
- `readme`
- `keywords`
- `categories`
- `exclude` / `include`

特に proc macro crate は内部向けであることを description に明記しておくと、利用者がどちらへ依存すべきか迷いにくくなります。

### 3. unsafe feature の扱いを決める

`unchecked` feature は速度より安全条件の説明が重要です。公開前には次のどちらかに寄せたいです。

- 初回公開では `unchecked` を残すが、README と rustdoc に安全条件を強めに書く。
- 初回公開では `unchecked` を外す、または experimental 扱いにして、通常APIを安定面として出す。

今のおすすめは、まず通常APIを主役にして、`unchecked` は明示featureかつ上級者向けとして扱うことです。

### 4. テスト配置を library crate 寄りにする

現在は `examples/basic` が強い検証役を持っています。公開ライブラリとしては、主要な挙動を `crates/fixed-record/tests/` へ移すと保守しやすくなります。

- builder / parse / to_bytes
- Reader / Writer
- List / Index
- compile-fail
- feature combinations: default, `default-features = false`, `unchecked`

`examples/basic` はサンプル実行用に薄く残し、保証は library crate 側に寄せるのがよさそうです。

### 5. CI の合格ラインを決める

公開前の最低ラインは次のセットがおすすめです。

```bash
cargo fmt --all --check
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

現状は `-D warnings` まで通るので、CI ではこのラインを採用できます。

### 6. README を公開向けに磨く

README は、内部構成よりも利用者が最初に動かす導線を優先します。

- install
- 最小サンプル
- Reader / Writer のサンプル
- List を使うサンプル
- `default-features = false` の用途
- `unchecked` feature の注意

workspace の説明や内部設計は、README では短く触れる程度にして、詳しい設計メモは DEVELOPMENT に寄せます。
