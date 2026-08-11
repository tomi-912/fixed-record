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

今後は、単体版 `fixed_record` にだけ追加されている機能が出てきたら、こちらの `fixed_record_main/` と `fixed_record_macros/` へ移植していく流れが自然です。

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
  - `trybuild` を使った compile-fail test を `app/tests/ui/` に置いています。

### 重要度中

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

### 重要度低

- `fixed_record_main` が `fixed_record_macros` に依存して再エクスポートしています。
  - 利用者は `fixed_record_main::prelude::*` だけで使えるので便利です。
  - 一方で通常ライブラリと proc macro の依存関係が常にセットになるため、最小依存にしたい場合は feature 化を検討できます。
- `app` にサンプルとテストがかなり入っています。
  - workspace の検証としては便利ですが、ライブラリの保証としては `fixed_record_main` / `fixed_record_macros` 側にテストが少なく見えます。
  - 主要な挙動は library crate の unit/integration test へ移すと保守しやすいです。
- `Error::AlignmentError` と `Error::ParseError` の doc comment が薄く、利用者がどの API で発生するか追いづらいです。

## 次に直すなら

1. unchecked API を残すか、別 trait に分けるかを決める。
2. `as_bytes_unchecked` / `parse_unchecked` / `from_bytes_unchecked` / `from_str_unchecked` の安全条件をテストとドキュメントでさらに固める。
3. 桁あふれや `Fixed<0>` の扱いを決め、Result 版 setter または入力検証を追加する。
4. app 側のテストを library crate の integration test へ移す。
5. Clippy 警告を潰して、`cargo clippy -- -D warnings` でも通る状態にする。
