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

- `unchecked` feature を有効にした場合だけ生成される `as_bytes_unchecked` / `parse_unchecked` / `from_bytes_unchecked` / `from_str_unchecked` は、`unsafe` なメモリ変換に依存しています。
  - ここでいう `unsafe` なメモリ変換とは、「構造体をそのままバイト列として見る」「バイト列をそのまま構造体として見る」という処理のことです。
  - 例として、次のようなレコードがあるとします。

    ```rust
    #[fixed_record_main]
    pub struct User {
        pub id: Fixed<8>,
        pub name: Fixed<16>,
        pub age: Fixed<3>,
    }
    ```

    人間の感覚では、この構造体は `8 + 16 + 3 = 27` バイトぴったりに見えます。固定長レコードとしても、そう扱いたいです。
  - しかし Rust の構造体は、内部メモリ上で必ずしも「フィールドを単純に前から詰めた形」になるとは限りません。CPU が読みやすい位置にフィールドを置くため、フィールドとフィールドの間や末尾に、見えない余白バイトが入ることがあります。この余白を padding と呼びます。
  - padding が入った構造体をそのままバイト列として扱うと、固定長レコードとして期待するバイト列と、実際のメモリ上のバイト列がずれる可能性があります。
  - `#[repr(C)]` は、Rust に「この構造体のフィールド順や配置ルールを C 言語互換寄りにしてほしい」と伝える属性です。外部の C 言語コードと構造体をやり取りしたい時などに使います。
  - ただし `#[repr(C)]` は「padding が絶対に入らない」ことを保証するものではありません。フィールド順は安定しやすくなりますが、C のルールでも alignment の都合で padding は入り得ます。
  - alignment は「この型の値はメモリ上の何バイト境界に置かれる必要があるか」という制約です。例えば alignment が 4 の型は、4 の倍数のアドレスに置かれる必要があります。
  - 今の `Fixed<N>` は中身が `[u8; N]` なので alignment が 1 です。そのため、現在のようにフィールドが全部 `Fixed<N>` だけなら padding が入りにくく、多くのケースでは期待通り動きます。
  - ただし、これは「今たまたま成立している前提」に近いです。将来 `Fixed<N>` の中身を変えたり、macro が `Fixed<N>` 以外のフィールドを許可したり、生成コードに別のフィールドを足したりすると、padding や alignment の問題が表面化する可能性があります。
  - 特に危ないのは、公開 API の見た目が安全そうなことです。以前の `User::from_str("...")` は普通のパース処理に見えましたが、内部では文字列のバイト列を `&User` として直接読み替えていました。
  - 現在の `parse` は、入力バイト列を各フィールドの長さごとに切り出して `Fixed<N>` へコピーする実装になっています。
  - 現在の `to_bytes` も、各フィールドの `as_bytes()` を順番に出力配列へコピーする実装になっています。
  - そのため、通常利用では構造体全体のメモリレイアウトに依存しません。
  - 元のレイアウト依存版は、`unchecked` feature 有効時のみ `as_bytes_unchecked` / `parse_unchecked` / `from_bytes_unchecked` / `from_str_unchecked` として生成されます。
  - これらは `unsafe fn` なので、呼び出し側が「構造体のメモリレイアウトが固定長レコードのバイト配置と完全に一致している」ことを保証する必要があります。
  - `#[repr(C)]` も `unchecked` feature 有効時だけ付与されます。通常時の `parse` / `to_bytes` はフィールド単位コピーなので、`#[repr(C)]` に依存しません。
  - 今後さらに安全寄りにするなら、unchecked API が本当に必要か、別 trait に分けるかを検討するとよさそうです。
- `Reader::next` の I/O エラー処理は改善済みです。
  - 改善前は、`read_exact` が `UnexpectedEof` 以外のエラーを返しても `None` になっていました。
  - 改善前は、`fill_buf` のエラーも無視されていました。
  - `Iterator<Item = Result<T, Error>>` なのに I/O エラーを呼び出し側へ返せないため、途中の読み取り失敗が正常終了に見える状態でした。
  - 改善前の挙動は、おおよそ次の形でした。

    ```rust
    if let Err(e) = self.reader.read_exact(&mut buf) {
        if e.kind() == ErrorKind::UnexpectedEof {
            return None;
        }
        return None;
    }

    loop {
        let available = match self.reader.fill_buf() {
            Ok(bytes) => bytes,
            Err(_) => break,
        };
        // 改行読み飛ばし
    }
    ```

  - `Iterator::next` の `None` は、本来「これ以上レコードがない」という正常終了を表します。
  - そのため、ディスク、ネットワーク、権限、途中で壊れたストリームなどの I/O エラーを `None` にしてしまうと、呼び出し側が `for rec in reader { ... }` のように処理している場合、途中で読み取りに失敗しても単に最後まで読み終わったように見えます。
  - 現在は `fixed_record_main::error::Error` に I/O エラー用の variant を追加しています。

    ```rust
    pub enum Error {
        TooShort,
        IncompleteRecord { expected: usize, actual: usize },
        Io(std::io::Error),
        AlignmentError,
        ParseError,
    }
    ```

  - 現在の `Reader::next` は `read` を使って `T::TOTAL_LEN` まで自前で読み進め、読み取ったバイト数を見て判定します。

    ```rust
    let mut read_len = 0;

    while read_len < T::TOTAL_LEN {
        match self.reader.read(&mut buf[read_len..]) {
            Ok(0) if read_len == 0 => return None,
            Ok(0) => {
                return Some(Err(Error::IncompleteRecord {
                    expected: T::TOTAL_LEN,
                    actual: read_len,
                }));
            }
            Ok(n) => read_len += n,
            Err(e) => return Some(Err(Error::Io(e))),
        }
    }
    ```

  - レコード先頭でまだ 1 バイトも読んでいない EOF は、通常の終端として `None` を返します。
  - レコード途中で EOF になった場合は、`Some(Err(Error::IncompleteRecord { expected, actual }))` を返します。
  - I/O エラーは `Some(Err(Error::Io(e)))` を返します。
  - 改行読み飛ばし中の `fill_buf` エラーも、現在は `Some(Err(Error::Io(e)))` として返します。
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
- README の機能一覧は現在の生成内容とおおむね合っています。

### 次に直すなら

1. unchecked API を残すか、別 trait に分けるかを決める。
2. `as_bytes_unchecked` / `parse_unchecked` / `from_bytes_unchecked` / `from_str_unchecked` の安全条件をテストとドキュメントでさらに固める。
3. proc macro の `panic!` を `syn::Error` に置き換えて compile error を改善する。
4. 桁あふれや `Fixed<0>` の扱いを決め、Result 版 setter または入力検証を追加する。
5. app 側のテストを library crate の integration test へ移す。
6. Clippy 警告を潰して、`cargo clippy -- -D warnings` でも通る状態にする。
