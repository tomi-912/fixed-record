# app examples

この `app` は、公開時に利用者が依存する `fixed_record_main` の使い方を実際に動かして確認するための場所です。

## ざっくりした違い

- `fixed_record_main`: 利用者が普通に依存する本体クレートです。`Fixed<N>`、`Error`、`Reader`、`Writer`、`FixedRecord`、`prelude`、`#[fixed_record_main]` の再エクスポートを提供します。
- `fixed_record_macros`: `#[fixed_record_main]` を実装する内部向け proc macro クレートです。構造体を読んで、パース、ビルダー、フィールド enum、リスト管理などのコードを生成します。

## どちらを使うべきか

普段のアプリケーションコードでは、基本的に `fixed_record_main` だけを使います。

```rust
use fixed_record_main::prelude::*;
```

この `prelude` の中に `Fixed` や `Reader` / `Writer`、さらに `#[fixed_record_main]` macro の再エクスポートも入っています。

公開パッケージとして使う側は、基本的に `fixed_record_macros` へ直接依存しません。

## 実行例

利用者目線の通常利用:

```bash
cargo run -p app --bin fixed_record_main_usage
```

再エクスポートされた proc macro を明示 import する例:

```bash
cargo run -p app --bin fixed_record_macros_role
```

## 関係性

`fixed_record_macros` は、生成するコードの中で `::fixed_record_main::Fixed` や `::fixed_record_main::Error` などを参照します。

つまり、`fixed_record_macros` は「コードを生成する係」で、`fixed_record_main` は「生成されたコードが使う実体と利用者向け入口を提供する係」です。
