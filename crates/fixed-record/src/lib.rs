//! Fixed-width record helpers generated from plain Rust structs.
//!
//! `fixed-record` lets application code describe a fixed-width record once as a Rust struct,
//! then use generated builders, parsers, field accessors, stream readers/writers, and searchable
//! list APIs.
//!
//! `fixed-record` は、固定長レコードのレイアウトを Rust の struct として一度だけ定義し、
//! builder、parser、フィールド accessor、stream 用 Reader/Writer、検索可能な List API を
//! 生成して使うための crate です。
//!
//! Most users should depend only on this crate and import [`prelude`].
//! The `fixed-record-macros` crate is the proc-macro implementation detail re-exported here.
//!
//! 通常の利用者はこの crate だけに依存し、[`prelude`] を import すれば十分です。
//! `fixed-record-macros` はここから再エクスポートされる proc macro 実装用 crate です。
//!
//! # Installation / インストール
//!
//! Before the crate is published to crates.io, depend on the Git repository.
//!
//! crates.io に公開する前は、Git repository 依存として追加します。
//!
//! ```toml
//! [dependencies]
//! fixed-record = { git = "https://github.com/tomi-912/fixed-record.git" }
//! ```
//!
//! After publishing to crates.io, use the versioned dependency.
//!
//! crates.io 公開後は、version 指定の依存として追加できます。
//!
//! ```toml
//! [dependencies]
//! fixed-record = "0.1"
//! ```
//!
//! # Define a Record / レコードを定義する
//!
//! Add [`macro@fixed_record`] to a struct whose fields are [`Fixed<N>`](Fixed). Each field width is
//! a byte width, not a character count.
//!
//! [`macro@fixed_record`] を [`Fixed<N>`](Fixed) フィールドだけを持つ struct に付けます。
//! 各フィールド幅は文字数ではなくバイト数です。
//!
//! ```
//! use fixed_record::prelude::*;
//!
//! #[fixed_record]
//! struct Order {
//!     customer_id: Fixed<4>,
//!     order_no: Fixed<6>,
//!     amount: Fixed<8>,
//! }
//!
//! assert_eq!(Order::TOTAL_LEN, 18);
//! assert_eq!(Order::FIELD_SIZE_CUSTOMER_ID, 4);
//! assert_eq!(Order::offset_of(OrderField::Amount), 10);
//! assert_eq!(Order::name_of(OrderField::OrderNo), "order_no");
//! assert_eq!(OrderField::Amount.as_str(), "amount");
//! assert_eq!(OrderField::Amount.size(), 8);
//! ```
//!
//! Generated names follow the input struct:
//!
//! 生成名は入力 struct から決まります。
//! `OrderField` and `OrderList` use the same visibility as `Order`.
//! `OrderField` と `OrderList` は `Order` と同じ可視性で生成されます。
//!
//! - `Order`: the record type / レコード型
//! - `OrderField`: the field enum / フィールド enum
//! - `OrderList`: the searchable list type when the default `list` feature is enabled /
//!   default feature の `list` が有効な場合の検索用 list 型
//!
//! # Build Records / レコードを作成する
//!
//! `builder()` starts with `CLEAR_BYTE` padding. Plain `with_*` methods write from the beginning of
//! the field, while numeric helpers write zero-padded strings.
//!
//! `builder()` は `CLEAR_BYTE` で埋めた値から開始します。通常の `with_*` はフィールド先頭から
//! 上書きし、数値用 helper はゼロ埋め文字列を書き込みます。
//!
//! ```
//! use fixed_record::prelude::*;
//!
//! #[fixed_record]
//! struct Order {
//!     customer_id: Fixed<4>,
//!     order_no: Fixed<6>,
//!     amount: Fixed<8>,
//! }
//!
//! let order = Order::builder()
//!     .with_customer_id("C001")
//!     .with_order_no("A00042")
//!     .with_amount_int(1250)
//!     .build();
//!
//! assert_eq!(order.customer_id(), b"C001");
//! assert_eq!(order.order_no_str().unwrap(), "A00042");
//! assert_eq!(order.amount(), b"00001250");
//! assert_eq!(order.byte_len(), Order::TOTAL_LEN);
//! ```
//!
//! Use `try_with_*_int` or `try_with_*_int_signed` when overflow must be an error. The non-`try`
//! variants keep the leading bytes when a formatted number is too wide.
//!
//! 幅超過をエラーとして扱いたい場合は `try_with_*_int` / `try_with_*_int_signed` を使います。
//! `try` なしの variant は、幅を超えた数値の先頭側を残します。
//!
//! ```
//! use fixed_record::prelude::*;
//!
//! #[fixed_record]
//! struct Balance {
//!     account: Fixed<4>,
//!     signed_amount: Fixed<6>,
//! }
//!
//! let balance = Balance::builder()
//!     .with_account("A001")
//!     .try_with_signed_amount_int_signed(-42)
//!     .unwrap()
//!     .build();
//!
//! assert_eq!(balance.signed_amount(), b"-00042");
//!
//! let err = Balance::builder()
//!     .try_with_signed_amount_int_signed(-123456)
//!     .unwrap_err();
//! assert!(matches!(err, fixed_record::Error::FieldOverflow { field: "signed_amount", .. }));
//! ```
//!
//! # Parse and Serialize / パースとシリアライズ
//!
//! `parse` / `parse_str` read fixed-width bytes into an owned record. `to_bytes` copies the record
//! back to its fixed-width byte array.
//!
//! `parse` / `parse_str` は固定長バイト列を所有値のレコードへ変換します。`to_bytes` はレコードを
//! 固定長バイト配列へコピーして返します。
//!
//! ```
//! use fixed_record::prelude::*;
//!
//! #[fixed_record]
//! struct Order {
//!     customer_id: Fixed<4>,
//!     order_no: Fixed<6>,
//!     amount: Fixed<8>,
//! }
//!
//! let raw = b"C001A0004200001250";
//! let order = Order::parse(raw).unwrap();
//!
//! assert_eq!(order.get_field_trimmed(OrderField::CustomerId).unwrap(), "C001");
//! assert_eq!(order.get_field_as::<u32>(OrderField::Amount).unwrap(), 1250);
//! assert_eq!(order.to_bytes(), *raw);
//!
//! let same = Order::parse_str("C001A0004200001250").unwrap();
//! assert_eq!(same.to_bytes(), *raw);
//! ```
//!
//! # Dynamic Field Operations / 動的フィールド操作
//!
//! Field enum values let you write generic code that does not hard-code accessor method names.
//! `set_field_*` clears the target field first; `set_field_*_no_clear` preserves trailing bytes.
//!
//! フィールド enum を使うと、accessor 名を固定せずに汎用的な処理を書けます。
//! `set_field_*` は対象フィールドを先にクリアし、`set_field_*_no_clear` は後続バイトを残します。
//!
//! ```
//! use fixed_record::prelude::*;
//!
//! #[fixed_record]
//! struct Order {
//!     customer_id: Fixed<4>,
//!     order_no: Fixed<6>,
//!     amount: Fixed<8>,
//! }
//!
//! let mut order = Order::spaced()
//!     .with_customer_id("C001")
//!     .with_order_no("A00042")
//!     .with_amount_int(1250)
//!     .build();
//!
//! order.set_field_str(OrderField::OrderNo, "B7");
//! assert_eq!(order.order_no(), b"B7    ");
//!
//! order.set_field_str_no_clear(OrderField::OrderNo, "C8");
//! assert_eq!(order.order_no(), b"C8    ");
//!
//! order.fill_field_zero(OrderField::Amount);
//! assert_eq!(order.amount(), &[0, 0, 0, 0, 0, 0, 0, 0]);
//! ```
//!
//! # Bulk Application Helpers / 一括流し込み helper
//!
//! `apply_bytes` and `apply_str` split input across fields in declaration order. The `_from`
//! variants begin at a selected field.
//!
//! `apply_bytes` / `apply_str` は定義順にフィールド幅ごとに入力を流し込みます。`_from` variant は
//! 指定フィールドから開始します。
//!
//! ```
//! use fixed_record::prelude::*;
//!
//! #[fixed_record]
//! struct Order {
//!     customer_id: Fixed<4>,
//!     order_no: Fixed<6>,
//!     amount: Fixed<8>,
//! }
//!
//! let order = Order::spaced().apply_bytes(b"C001A0004200001250");
//! assert_eq!(order.customer_id(), b"C001");
//! assert_eq!(order.order_no(), b"A00042");
//! assert_eq!(order.amount(), b"00001250");
//!
//! let changed_tail = Order::spaced()
//!     .with_customer_id("C999")
//!     .apply_str_from(OrderField::OrderNo, "B0007700000900");
//!
//! assert_eq!(changed_tail.customer_id(), b"C999");
//! assert_eq!(changed_tail.order_no(), b"B00077");
//! assert_eq!(changed_tail.amount(), b"00000900");
//! ```
//!
//! # Reader and Writer / Reader / Writer
//!
//! [`Reader`] reads records sequentially from any `BufRead`. `Reader::new` expects LF (`\n`) after
//! records. Use [`Reader::with_separator`] when the input uses a different separator or no
//! separator. When configured, the separator must also appear after the final record. [`Writer`]
//! writes `to_bytes()` output and appends LF by default.
//!
//! [`Reader`] は任意の `BufRead` から固定長レコードを順に読みます。`Reader::new` はレコード後ろに
//! LF (`\n`) がある入力を想定します。別の区切り、または区切りなしの入力を読む場合は
//! [`Reader::with_separator`] で指定します。区切りを設定した場合は、最終レコードの後ろにもその区切りが必要です。
//! [`Writer`] は `to_bytes()` の結果を書き出し、デフォルトでは LF を付けます。
//!
//! ```
//! use fixed_record::prelude::*;
//! use std::io::{BufReader, Cursor};
//!
//! #[fixed_record]
//! struct Order {
//!     customer_id: Fixed<4>,
//!     order_no: Fixed<6>,
//!     amount: Fixed<8>,
//! }
//!
//! let first = Order::builder()
//!     .with_customer_id("C001")
//!     .with_order_no("A00001")
//!     .with_amount_int(100)
//!     .build();
//! let second = Order::builder()
//!     .with_customer_id("C002")
//!     .with_order_no("A00002")
//!     .with_amount_int(200)
//!     .build();
//!
//! let mut output = Vec::new();
//! let mut writer = Writer::new(&mut output)
//!     .with_separator(RecordSeparator::Crlf);
//! writer.write_record(&first).unwrap();
//! writer.write_record(&second).unwrap();
//! writer.flush().unwrap();
//! drop(writer);
//!
//! let mut reader = Reader::<_, Order>::new(BufReader::new(Cursor::new(output)))
//!     .with_separator(RecordSeparator::Crlf);
//! assert_eq!(reader.next().unwrap().unwrap().order_no(), b"A00001");
//! assert_eq!(reader.next().unwrap().unwrap().order_no(), b"A00002");
//! assert!(reader.next().is_none());
//! ```
//!
//! Sequence checks can validate that input is sorted by selected fields.
//!
//! シーケンスチェックを使うと、入力が指定フィールド順に昇順であることを検証できます。
//!
//! ```
//! use fixed_record::prelude::*;
//! use std::io::{BufReader, Cursor};
//!
//! #[fixed_record]
//! struct Order {
//!     customer_id: Fixed<4>,
//!     order_no: Fixed<6>,
//!     amount: Fixed<8>,
//! }
//!
//! let first = Order::builder().with_customer_id("C001").with_order_no("A00001").with_amount_int(100).build();
//! let second = Order::builder().with_customer_id("C001").with_order_no("A00002").with_amount_int(200).build();
//!
//! let mut input = Vec::new();
//! input.extend_from_slice(&first.to_bytes());
//! input.push(b'\n');
//! input.extend_from_slice(&second.to_bytes());
//! input.push(b'\n');
//!
//! let mut reader = Reader::<_, Order>::new(BufReader::new(Cursor::new(input)))
//!     .with_sequence_check([OrderField::CustomerId, OrderField::OrderNo]);
//!
//! assert!(reader.next().unwrap().is_ok());
//! assert!(reader.next().unwrap().is_ok());
//! assert!(reader.next().is_none());
//! ```
//!
//! # Searchable Lists / 検索可能な List
//!
//! With the default `list` feature, the macro generates the optional helper `{StructName}List`. It
//! stores boxed records in a vector and maintains ordered field-byte indexes for appending,
//! position-based insertion, lookup, update, physical removal, popping the last record, sorting,
//! exact searches, padded searches, prefix searches, range searches, and index-aware callback edits.
//! Exact searches use the index directly instead of scanning every record.
//! Search-based edits select private current indexes internally and repair affected index entries
//! after the callback, without returning indexes or mutable references to the caller.
//! Sorting moves boxes in the vector, not the record values allocated behind them.
//! List IDs are the current vector indexes, so IDs can change after removal or sorting.
//!
//! default feature の `list` が有効な場合、macro は補助機能として `{StructName}List` を生成します。
//! これは Box 化したレコードを vector に保持し、フィールドの実バイト列による順序付き索引を管理して、
//! 末尾追加、位置指定挿入、lookup、update、物理削除、末尾レコードの pop、sort、完全一致検索、
//! padding を考慮した検索、prefix 検索、range 検索、索引対応の callback 変更を提供します。
//! 完全一致検索は全レコードを走査せず、索引を直接参照します。
//! 検索条件付き変更は非公開の現在 index を内部で選択し、index や mutable 参照を呼び出し側へ返さず、
//! callback 後に影響する索引項目を修復します。
//! ソート時は vector 内の Box が移動し、Box の先にあるレコード本体は移動しません。
//! List ID は現在の vector index なので、削除やソート後に変わる可能性があります。
//!
//! ```
//! use fixed_record::prelude::*;
//!
//! #[fixed_record]
//! struct Order {
//!     customer_id: Fixed<4>,
//!     order_no: Fixed<6>,
//!     amount: Fixed<8>,
//! }
//!
//! let mut list = OrderList::new();
//! let id_first = list.push(Order::builder()
//!     .with_customer_id("C001")
//!     .with_order_no("A00002")
//!     .with_amount_int(200)
//!     .build());
//! let id_second = list.push(Order::builder()
//!     .with_customer_id("C001")
//!     .with_order_no("A00001")
//!     .with_amount_int(100)
//!     .build());
//!
//! assert_eq!(list.len(), 2);
//! assert_eq!(list.get(id_first).unwrap().amount(), b"00000200");
//!
//! let exact = list.try_find_by(OrderField::CustomerId, b"C001").unwrap();
//! assert_eq!(exact.len(), 2);
//!
//! let first_exact = list.try_first_by(OrderField::OrderNo, b"A00001").unwrap().unwrap();
//! assert_eq!(first_exact.amount(), b"00000100");
//!
//! let padded = list.try_find_by_padded(OrderField::OrderNo, b"A00001").unwrap();
//! assert_eq!(padded[0].amount(), b"00000100");
//!
//! let prefix = list.try_find_by_prefix(OrderField::OrderNo, b"A000").unwrap();
//! assert_eq!(prefix.len(), 2);
//!
//! let first_by_order_no = list.first_sorted_by(OrderField::OrderNo).unwrap();
//! assert_eq!(first_by_order_no.order_no(), b"A00001");
//!
//! assert!(list.try_edit_first_by(OrderField::OrderNo, b"A00001", |order| {
//!     order.set_field_str(OrderField::Amount, "00000150");
//! }).unwrap());
//! assert_eq!(list.try_find_by(OrderField::Amount, b"00000150").unwrap().len(), 1);
//!
//! list.update(id_first, Order::builder()
//!     .with_customer_id("C002")
//!     .with_order_no("B00001")
//!     .with_amount_int(300)
//!     .build());
//! assert!(list.try_find_by_padded(OrderField::CustomerId, b"C001").unwrap().len() == 1);
//!
//! list.for_each_mut(|order| order.set_field_str(OrderField::CustomerId, "C003"));
//! assert_eq!(list.try_find_by(OrderField::CustomerId, b"C003").unwrap().len(), 2);
//!
//! assert!(list.remove(id_second));
//! assert_eq!(list.len(), 1);
//! assert_eq!(list.all_ids(), vec![id_first]);
//! ```
//!
//! Fallible search methods use the `try_` prefix. Range searches accept byte-like bounds through
//! [`ByteRangeBounds`](traits::ByteRangeBounds).
//!
//! 失敗する可能性がある検索メソッドは `try_` prefix を使います。range 検索は
//! [`ByteRangeBounds`](traits::ByteRangeBounds) を通して byte 列として扱える境界値を受け取ります。
//!
//! ```
//! use fixed_record::prelude::*;
//!
//! #[fixed_record]
//! struct Order {
//!     customer_id: Fixed<4>,
//!     order_no: Fixed<6>,
//!     amount: Fixed<8>,
//! }
//!
//! let mut list = OrderList::new();
//! list.push(Order::builder().with_customer_id("C001").with_order_no("A00001").with_amount_int(100).build());
//! list.push(Order::builder().with_customer_id("C001").with_order_no("A00002").with_amount_int(200).build());
//! list.push(Order::builder().with_customer_id("C001").with_order_no("A00003").with_amount_int(300).build());
//!
//! let found = list
//!     .try_find_range_by(OrderField::Amount, b"00000150"..=b"00000300")
//!     .unwrap();
//!
//! assert_eq!(found.len(), 2);
//! assert_eq!(found[0].amount(), b"00000200");
//! assert_eq!(found[1].amount(), b"00000300");
//! ```
//!
//! # Clear Byte and Initialization / clear_byte と初期化
//!
//! By default, [`macro@fixed_record`] uses spaces (`0x20`) as `CLEAR_BYTE`. You can change this with
//! `clear_byte = ZERO`, `clear_byte = SPACE`, a byte literal, or an integer from `0` to `255`.
//!
//! 標準では [`macro@fixed_record`] は半角スペース (`0x20`) を `CLEAR_BYTE` として使います。
//! `clear_byte = ZERO`、`clear_byte = SPACE`、byte literal、`0` から `255` の整数で変更できます。
//!
//! ```
//! use fixed_record::prelude::*;
//!
//! #[fixed_record(clear_byte = ZERO)]
//! struct ZeroPaddedName {
//!     name: Fixed<6>,
//! }
//!
//! let mut record = ZeroPaddedName::builder().with_name("Alice").build();
//! record.set_field_str(ZeroPaddedNameField::Name, "Bo");
//!
//! assert_eq!(ZeroPaddedName::CLEAR_BYTE, 0x00);
//! assert_eq!(record.name(), b"Bo\0\0\0\0");
//! assert_eq!(ZeroPaddedName::zeroed().name(), b"\0\0\0\0\0\0");
//! assert_eq!(ZeroPaddedName::spaced().name(), b"      ");
//! assert_eq!(ZeroPaddedName::cleared().name(), b"\0\0\0\0\0\0");
//! ```
//!
//! # FixedRecord Trait / FixedRecord trait
//!
//! Generated records implement [`FixedRecord`], so generic code can parse or serialize any generated
//! record type accepted by [`Reader`] and [`Writer`].
//!
//! 生成されたレコードは [`FixedRecord`] を実装します。そのため [`Reader`] / [`Writer`] が扱える
//! 任意の生成レコードに対して generic な parse / serialize 処理を書けます。
//!
//! ```
//! use fixed_record::prelude::*;
//!
//! #[fixed_record]
//! struct Order {
//!     customer_id: Fixed<4>,
//!     order_no: Fixed<6>,
//!     amount: Fixed<8>,
//! }
//!
//! fn serialize<T: FixedRecord>(record: &T) -> Vec<u8> {
//!     record.to_bytes()
//! }
//!
//! let order = Order::builder()
//!     .with_customer_id("C001")
//!     .with_order_no("A00042")
//!     .with_amount_int(1250)
//!     .build();
//!
//! assert_eq!(serialize(&order), b"C001A0004200001250");
//! ```
//!
//! # Feature Flags / feature flag
//!
//! - `list`: enabled by default. Generates the optional `{StructName}List` helper.
//! - `list`: default で有効です。補助機能の `{StructName}List` を生成します。
//!
//! Disable default features when you only need records, field operations, parsing, `Reader`, and
//! `Writer`:
//!
//! レコード本体、フィールド操作、parse、`Reader`、`Writer` だけを使う場合:
//!
//! ```toml
//! [dependencies]
//! fixed-record = { version = "0.1", default-features = false }
//! ```
//!
//! # Zerocopy Support / zerocopy 対応
//!
//! Generated records always use `#[repr(C)]` and derive `zerocopy::FromBytes`,
//! `zerocopy::IntoBytes`, `zerocopy::Immutable`, and `zerocopy::KnownLayout`.
//! The zerocopy traits are re-exported from [`prelude`].
//!
//! 生成レコードには常に `#[repr(C)]` が付き、`zerocopy::FromBytes`、
//! `zerocopy::IntoBytes`、`zerocopy::Immutable`、`zerocopy::KnownLayout` を derive します。
//! zerocopy traits は [`prelude`] から再エクスポートされます。
//!
//! ```
//! use fixed_record::prelude::*;
//!
//! #[fixed_record]
//! struct Order {
//!     customer_id: Fixed<4>,
//!     order_no: Fixed<6>,
//!     amount: Fixed<8>,
//! }
//!
//! let raw = b"C001A0004200001250";
//!
//! let order = Order::ref_from_bytes(raw).unwrap();
//! assert_eq!(order.customer_id(), b"C001");
//! assert_eq!(order.as_bytes(), raw);
//! assert_eq!(order.as_str().unwrap(), "C001A0004200001250");
//! ```
//!
//! Use `ref_from_bytes_prefix` when the input may contain trailing bytes after the first record.
//!
//! 先頭1レコードの後ろに余りバイトが続く可能性がある場合は、`ref_from_bytes_prefix` を使います。
//!
//! ```
//! use fixed_record::prelude::*;
//!
//! #[fixed_record]
//! struct Order {
//!     customer_id: Fixed<4>,
//!     order_no: Fixed<6>,
//!     amount: Fixed<8>,
//! }
//!
//! let raw = b"C001A0004200001250rest";
//! let order = Order::ref_from_bytes_prefix(raw).unwrap();
//! assert_eq!(order.customer_id(), b"C001");
//! ```
//!
//! String inputs can use `ref_from_str` or `ref_from_str_prefix`.
//!
//! 文字列入力では `ref_from_str` または `ref_from_str_prefix` を使えます。
//!
//! ```
//! use fixed_record::prelude::*;
//!
//! #[fixed_record]
//! struct Order {
//!     customer_id: Fixed<4>,
//!     order_no: Fixed<6>,
//!     amount: Fixed<8>,
//! }
//!
//! let order = Order::ref_from_str_prefix("C001A0004200001250rest").unwrap();
//! assert_eq!(order.order_no_str().unwrap(), "A00042");
//! ```
//!
extern crate self as fixed_record;

pub mod error;
pub mod io;
pub mod traits;
pub mod types;

pub use fixed_record_macros::fixed_record;
pub use zerocopy;

pub use error::Error;
pub use io::{Reader, RecordSeparator, Writer};
pub use traits::{ByteRangeBounds, FixedRecord};
pub use types::Fixed;
pub use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

pub mod prelude {
    pub use crate::fixed_record;
    pub use crate::{
        ByteRangeBounds, Error, Fixed, FixedRecord, FromBytes, Immutable, IntoBytes, KnownLayout,
        Reader, RecordSeparator, Writer,
    };
}

/// Documentation examples for APIs generated by [`macro@fixed_record`].
/// [`macro@fixed_record`] が生成する API を rustdoc で確認するためのサンプルです。
#[cfg(doc)]
pub mod doc_examples {
    use crate::prelude::*;

    /// Test record used to document the generated fixed-record API.
    /// 生成される fixed-record API のドキュメント用テストレコードです。
    #[fixed_record]
    pub struct TestRecord {
        /// Name field used by generated API tests.
        /// 生成 API テストで使う名前フィールドです。
        name: Fixed<10>,
        /// Code field used by generated API tests.
        /// 生成 API テストで使うコードフィールドです。
        code: Fixed<5>,
        /// Amount field used by generated API tests.
        /// 生成 API テストで使う金額フィールドです。
        amount: Fixed<8>,
    }
}
