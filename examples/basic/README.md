# fixed-record basic example

この example は、公開時に利用者が依存する `fixed-record` の使い方を実際に動かして確認するための場所です。

主要APIの保証は `crates/fixed-record/tests/` に置き、この example は読みやすい実行サンプルとして薄く保ちます。

## ざっくりした違い

- `fixed-record`: 利用者が普通に依存する本体クレートです。`Fixed<N>`、`Error`、`Reader`、`Writer`、`FixedRecord`、`prelude`、`#[fixed_record]` の再エクスポートを提供します。
- `fixed-record-macros`: `#[fixed_record]` を実装する内部向け proc macro クレートです。構造体を読んで、パース、ビルダー、フィールド enum、リスト管理などのコードを生成します。

## どちらを使うべきか

普段のアプリケーションコードでは、基本的に `fixed-record` だけに依存します。

```rust
use fixed_record::prelude::*;
```

この `prelude` の中に `Fixed` や `Reader` / `Writer`、さらに `#[fixed_record]` macro の再エクスポートも入っています。

公開パッケージとして使う側は、基本的に `fixed-record-macros` へ直接依存しません。

## 実行例

利用者目線の通常利用:

```bash
cargo run -p fixed-record-basic-example --bin fixed_record_usage
```

再エクスポートされた proc macro を明示 import する例:

```bash
cargo run -p fixed-record-basic-example --bin macro_reexport
```

## 関係性

`fixed-record-macros` は、生成するコードの中で `::fixed_record::Fixed` や `::fixed_record::Error` などを参照します。

つまり、`fixed-record-macros` は「コードを生成する係」で、`fixed-record` は「生成されたコードが使う実体と利用者向け入口を提供する係」です。
