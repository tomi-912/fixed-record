# app examples

この `app` は、`fixed_record_main` と `fixed_record_macros` の違いを実際に動かして確認するための場所です。

## ざっくりした違い

- `fixed_record_main`: 利用者が普通に依存する本体クレートです。`Fixed<N>`、`Error`、`Reader`、`Writer`、`FixedRecord`、`prelude` を提供します。
- `fixed_record_macros`: `#[fixed_record_main]` を実装する proc macro クレートです。構造体を読んで、パース、ビルダー、フィールド enum、リスト管理などのコードを生成します。

## どちらを使うべきか

普段のアプリケーションコードでは、基本的に `fixed_record_main` だけを使います。

```rust
use fixed_record_main::prelude::*;
```

この `prelude` の中に `Fixed` や `Reader` / `Writer`、さらに `#[fixed_record_main]` macro の再エクスポートも入っています。

`fixed_record_macros` を直接使うのは、macro クレート自体の挙動を確認したいとき、または「これは本体ではなくコード生成担当なんだ」と明示したい実験コードのときです。

## 実行例

利用者目線の通常利用:

```bash
cargo run -p app --bin fixed_record_main_usage
```

proc macro クレートを直接 import する例:

```bash
cargo run -p app --bin fixed_record_macros_role
```

## 関係性

`fixed_record_macros` は、生成するコードの中で `::fixed_record_main::Fixed` や `::fixed_record_main::Error` などを参照します。

つまり、`fixed_record_macros` は「コードを生成する係」で、`fixed_record_main` は「生成されたコードが使う実体を提供する係」です。
