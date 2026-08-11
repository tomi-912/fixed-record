use fixed_record_main::prelude::*;

#[fixed_record_main]
pub struct User {
    /// ユーザーの固有ID（8桁）
    pub id: Fixed<8>,

    /// 名前
    pub name: Fixed<16>,

    /// 年齢
    pub age: Fixed<3>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // テスト用の構造体を定義
    #[fixed_record_main]
    pub struct TestRecord {
        /// 名前フィールド
        name: Fixed<10>,
        /// コードフィールド
        code: Fixed<5>,
        /// 金額フィールド
        amount: Fixed<8>,
    }

    #[fixed_record_main]
    pub struct SplitUtf8Record {
        name: Fixed<8>,
        rest: Fixed<2>,
    }

    #[fixed_record_main(clear_byte = SPACE)]
    pub struct SpaceClearRecord {
        name: Fixed<6>,
    }

    // ---- apply<F> のテスト ----

    /// `apply` のクロージャで複数フィールドをまとめて更新できることを確認します。
    #[test]
    fn test_apply_closure() {
        let aa = TestRecord::builder().apply_str("test      ttte");

        let rec = TestRecord::spaced().apply(|r| {
            r.set_field_str(TestRecordField::Name, "Alice");
            r.set_field_str(TestRecordField::Code, "A001");
        });
        assert_eq!(aa.get_field_trimmed(TestRecordField::Name).unwrap(), "test");

        assert_eq!(
            rec.get_field_trimmed(TestRecordField::Name).unwrap(),
            "Alice"
        );
        assert_eq!(
            rec.get_field_trimmed(TestRecordField::Code).unwrap(),
            "A001"
        );
    }

    // ---- apply_bytes のテスト ----

    /// 全フィールド分ぴったりのバイト列を先頭から流し込めることを確認します。
    #[test]
    fn test_apply_bytes_exact() {
        // ちょうど TOTAL_LEN (23) バイト
        let data = b"HelloWorldABCDE12345678";
        let rec = TestRecord::spaced().apply_bytes(data);

        assert_eq!(rec.name(), b"HelloWorld");
        assert_eq!(rec.code(), b"ABCDE");
        assert_eq!(rec.amount(), b"12345678");
    }

    /// 入力バイト列が途中で終わる場合に、未到達部分が元のスペース埋めのまま残ることを確認します。
    #[test]
    fn test_apply_bytes_shorter_than_total() {
        // データが途中で終わる場合 → 残りのフィールドはspaced のまま
        let data = b"HelloWorldABC"; // name(10) + code の途中(3)
        let rec = TestRecord::spaced().apply_bytes(data);

        assert_eq!(rec.name(), b"HelloWorld");
        assert_eq!(&rec.code()[..3], b"ABC");
        assert_eq!(&rec.code()[3..], &[0, 0]); // 到達したフィールドの残りはCLEAR_BYTE
        assert_eq!(rec.amount(), b"        "); // 未到達はspaced のまま
    }

    // ---- apply_str のテスト ----

    /// 文字列入力をバイト列として各フィールドへ順番に流し込めることを確認します。
    #[test]
    fn test_apply_str() {
        let s = b"HelloWorldABCDE12345678";
        let rec = TestRecord::spaced().apply_bytes(s);

        assert_eq!(
            rec.get_field_trimmed(TestRecordField::Name).unwrap(),
            "HelloWorld"
        );
        assert_eq!(
            rec.get_field_trimmed(TestRecordField::Code).unwrap(),
            "ABCDE"
        );
        assert_eq!(
            rec.get_field_trimmed(TestRecordField::Amount).unwrap(),
            "12345678"
        );
    }

    // ---- apply_bytes_from のテスト ----

    /// 開始フィールドが先頭の場合に `apply_bytes` と同じ結果になることを確認します。
    #[test]
    fn test_apply_bytes_from_first_field() {
        // 先頭から開始 → apply_bytes と同じ動作
        let data = b"HelloWorldABCDE12345678";
        let rec = TestRecord::spaced().apply_bytes_from(TestRecordField::Name, data);

        assert_eq!(rec.name(), b"HelloWorld");
        assert_eq!(rec.code(), b"ABCDE");
        assert_eq!(rec.amount(), b"12345678");
    }

    /// 中間フィールドから流し込むと、それより前のフィールドが変更されないことを確認します。
    #[test]
    fn test_apply_bytes_from_middle_field() {
        // code フィールドから開始 → name はspaced のまま
        let data = b"ABCDE12345678";
        let rec = TestRecord::spaced().apply_bytes_from(TestRecordField::Code, data);

        assert_eq!(rec.name(), b"          "); // 未変更
        assert_eq!(rec.code(), b"ABCDE");
        assert_eq!(rec.amount(), b"12345678");
    }

    /// 最後のフィールドだけを指定してバイト列を流し込めることを確認します。
    #[test]
    fn test_apply_bytes_from_last_field() {
        // amount フィールドのみ書き込み
        let data = b"99999999";
        let rec = TestRecord::spaced().apply_bytes_from(TestRecordField::Amount, data);

        assert_eq!(rec.name(), b"          "); // 未変更
        assert_eq!(rec.code(), b"     "); // 未変更
        assert_eq!(rec.amount(), b"99999999");
    }

    // ---- apply_str_from のテスト ----

    /// 中間フィールドから文字列を流し込めることを確認します。
    #[test]
    fn test_apply_str_from_middle() {
        let s = "ABCDE12345678";
        let rec = TestRecord::spaced().apply_str_from(TestRecordField::Code, s);

        assert_eq!(rec.get_field_trimmed(TestRecordField::Name).unwrap(), "");
        assert_eq!(
            rec.get_field_trimmed(TestRecordField::Code).unwrap(),
            "ABCDE"
        );
        assert_eq!(
            rec.get_field_trimmed(TestRecordField::Amount).unwrap(),
            "12345678"
        );
    }

    // ---- メソッドチェーンの組み合わせテスト ----

    /// builder と `with_*` 系メソッドを連鎖してレコードを作成できることを確認します。
    #[test]
    fn test_method_chain() {
        let rec = TestRecord::builder()
            .with_name("Alice")
            .with_code("A001")
            .with_amount_int(12345)
            .build();

        assert_eq!(
            rec.get_field_trimmed(TestRecordField::Name).unwrap(),
            "Alice"
        );
        assert_eq!(
            rec.get_field_trimmed(TestRecordField::Code).unwrap(),
            "A001"
        );
        assert_eq!(
            rec.get_field_trimmed(TestRecordField::Amount).unwrap(),
            "00012345"
        );
    }

    /// `set_field_str` がデフォルトの `CLEAR_BYTE` でクリアしてから書き込むことを確認します。
    #[test]
    fn test_set_field_str_clears_with_default_zero_before_write() {
        let mut rec = TestRecord::spaced();
        rec.set_field_str(TestRecordField::Name, "Bob");

        assert_eq!(rec.name(), b"Bob\0\0\0\0\0\0\0");
        assert_eq!(TestRecord::CLEAR_BYTE, 0x00);
    }

    /// `set_field_str_no_clear` が既存フィールドの後続バイトを残すことを確認します。
    #[test]
    fn test_set_field_str_no_clear_keeps_existing_tail_bytes() {
        let mut rec = TestRecord::spaced().with_name("Alice");
        rec.set_field_str_no_clear(TestRecordField::Name, "Bo");

        assert_eq!(rec.name(), b"Boice     ");
    }

    /// `with_*` がクリアせずに先頭から上書きし、後続バイトを残すことを確認します。
    #[test]
    fn test_with_str_keeps_existing_tail_bytes() {
        let rec = TestRecord::spaced().with_name("Alice").with_name("Bo");

        assert_eq!(rec.name(), b"Boice     ");
    }

    /// `clear_byte` option で `set_field_*` のクリア値をスペースに変更できることを確認します。
    #[test]
    fn test_set_field_str_uses_configured_clear_byte() {
        let mut rec = SpaceClearRecord::zeroed();
        rec.set_field_str(SpaceClearRecordField::Name, "Bo");

        assert_eq!(SpaceClearRecord::CLEAR_BYTE, b' ');
        assert_eq!(rec.name(), b"Bo    ");
    }

    /// `builder` と `Default` がデフォルトの `CLEAR_BYTE` で初期化することを確認します。
    #[test]
    fn test_builder_and_default_use_default_clear_byte() {
        assert_eq!(
            TestRecord::builder().name(),
            &[0; TestRecord::FIELD_SIZE_NAME]
        );
        assert_eq!(
            TestRecord::default().name(),
            &[0; TestRecord::FIELD_SIZE_NAME]
        );
        assert_eq!(
            TestRecord::cleared().name(),
            &[0; TestRecord::FIELD_SIZE_NAME]
        );
    }

    /// `builder` と `Default` が attribute で指定した `CLEAR_BYTE` で初期化することを確認します。
    #[test]
    fn test_builder_and_default_use_configured_clear_byte() {
        assert_eq!(SpaceClearRecord::builder().name(), b"      ");
        assert_eq!(SpaceClearRecord::default().name(), b"      ");
        assert_eq!(SpaceClearRecord::cleared().name(), b"      ");
    }

    /// 数値 setter の Result 版がフィールド幅を超える値をエラーにすることを確認します。
    #[test]
    fn test_try_with_int_reports_field_overflow() {
        let err = TestRecord::builder()
            .try_with_amount_int(123456789)
            .unwrap_err();

        assert_eq!(
            err,
            Error::FieldOverflow {
                field: "amount",
                size: TestRecord::FIELD_SIZE_AMOUNT,
                actual: 9,
            }
        );
    }

    /// 符号付き数値 setter の Result 版が符号込みの桁あふれをエラーにすることを確認します。
    #[test]
    fn test_try_with_signed_int_reports_field_overflow() {
        let err = TestRecord::builder()
            .try_with_amount_int_signed(12345678)
            .unwrap_err();

        assert_eq!(
            err,
            Error::FieldOverflow {
                field: "amount",
                size: TestRecord::FIELD_SIZE_AMOUNT,
                actual: 9,
            }
        );
    }

    /// 明示的な切り捨て版の数値 setter が幅を超えた値を先頭側だけ残すことを確認します。
    #[test]
    fn test_truncated_int_setter_keeps_existing_truncation_behavior() {
        let rec = TestRecord::builder()
            .with_amount_int_truncated(123456789)
            .build();

        assert_eq!(rec.amount(), b"12345678");
    }

    /// 明示的な切り捨て版の符号付き数値 setter が符号込みで先頭側だけ残すことを確認します。
    #[test]
    fn test_truncated_signed_int_setter_keeps_existing_truncation_behavior() {
        let rec = TestRecord::builder()
            .with_amount_int_signed_truncated(-12345678)
            .build();

        assert_eq!(rec.amount(), b"-1234567");
    }

    /// `FixedRecord` trait 経由のバイト化と `Reader` / `Writer` の往復を確認します。
    #[test]
    fn test_reader_writer_and_fixed_record_trait() {
        use fixed_record_main::{FixedRecord, Reader, Writer};
        use std::io::{BufReader, Cursor};

        /// `FixedRecord` trait だけに依存してレコードをバイト列へ変換します。
        fn bytes_via_trait<T: FixedRecord>(record: &T) -> Vec<u8> {
            record.to_bytes()
        }

        let first = TestRecord::builder()
            .with_name("Alice")
            .with_code("A0001")
            .with_amount_int(10)
            .build();
        let second = TestRecord::builder()
            .with_name("Bob")
            .with_code("B0001")
            .with_amount_int(20)
            .build();

        assert_eq!(bytes_via_trait(&first).len(), TestRecord::TOTAL_LEN);

        let mut buf = Vec::new();
        let mut writer = Writer::new(&mut buf).with_newline(b"\r\n");
        writer.write_record(&first).unwrap();
        writer.write_record(&second).unwrap();
        writer.flush().unwrap();

        let mut reader = Reader::<_, TestRecord>::new(BufReader::new(Cursor::new(buf)));
        assert_eq!(
            reader
                .next()
                .unwrap()
                .unwrap()
                .get_field_trimmed(TestRecordField::Name)
                .unwrap(),
            "Alice"
        );
        assert_eq!(
            reader
                .next()
                .unwrap()
                .unwrap()
                .get_field_trimmed(TestRecordField::Name)
                .unwrap(),
            "Bob"
        );
        assert!(reader.next().is_none());
    }

    /// 生成された List 型の追加、検索、ソート、論理削除、vacuum を確認します。
    #[test]
    fn test_generated_list_management() {
        let mut list = TestRecordList::new();

        let id_b = list.insert(
            TestRecord::builder()
                .with_name("Bob")
                .with_code("B0001")
                .with_amount_int(20)
                .build(),
        );
        let id_a = list.insert(
            TestRecord::builder()
                .with_name("Alice")
                .with_code("A0001")
                .with_amount_int(10)
                .build(),
        );

        assert_eq!(list.len(), 2);

        let found = list.find_by(TestRecordField::Code, *b"A0001");
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].get_field_trimmed(TestRecordField::Name).unwrap(),
            "Alice"
        );

        list.sort_by(&[TestRecordField::Name]);
        let names: Vec<_> = list
            .iter()
            .map(|record| {
                record
                    .get_field_string_trimmed(TestRecordField::Name)
                    .unwrap()
            })
            .collect();
        assert_eq!(names, vec!["Alice".to_string(), "Bob".to_string()]);

        let first_by_code = list.try_first_sorted_by(TestRecordField::Code).unwrap();
        assert_eq!(
            first_by_code
                .get_field_trimmed(TestRecordField::Code)
                .unwrap(),
            "A0001"
        );

        assert!(list.remove(id_b));
        assert_eq!(list.len(), 1);
        assert_eq!(list.all_ids().len(), 2);
        list.vacuum();
        assert_eq!(list.all_ids(), vec![id_a]);
    }

    /// `try_find_by` が短い検索値で 0x00 / スペース埋めの固定長フィールドを取得できることを確認します。
    #[test]
    fn test_try_find_by_matches_short_value_with_zero_or_space_padding() {
        let mut list = TestRecordList::new();

        let space_padded = TestRecord::spaced()
            .with_name("Space")
            .with_code("A00")
            .with_amount_int(1)
            .build();

        let zero_padded = TestRecord::builder()
            .with_name("Zero")
            .with_code("A00")
            .with_amount_int(2)
            .build();

        let mut mixed_padded = TestRecord::spaced()
            .with_name("Mixed")
            .with_amount_int(3)
            .build();
        mixed_padded.set_field_bytes(TestRecordField::Code, b"A00 ");

        let other = TestRecord::spaced()
            .with_name("Other")
            .with_code("A00XX")
            .with_amount_int(4)
            .build();

        list.insert(space_padded);
        list.insert(zero_padded);
        list.insert(mixed_padded);
        list.insert(other);

        let found = list.try_find_by(TestRecordField::Code, b"A00").unwrap();
        let mut names: Vec<_> = found
            .iter()
            .map(|record| {
                record
                    .get_field_string_trimmed(TestRecordField::Name)
                    .unwrap()
            })
            .collect();
        names.sort();

        assert_eq!(
            names,
            vec!["Mixed".to_string(), "Space".to_string(), "Zero".to_string()]
        );
    }

    /// `try_find_by` がフィールド幅を超える検索値をエラーにすることを確認します。
    #[test]
    fn test_try_find_by_reports_overflow_for_too_long_value() {
        let list = TestRecordList::new();
        let err = list
            .try_find_by(TestRecordField::Code, b"A00001")
            .unwrap_err();

        assert_eq!(
            err,
            Error::FieldOverflow {
                field: "code",
                size: TestRecord::FIELD_SIZE_CODE,
                actual: 6,
            }
        );
    }

    /// `try_first_by` が短い検索値で 0x00 / スペース埋めの固定長フィールドを昇順の先頭から取得できることを確認します。
    #[test]
    fn test_try_first_by_matches_short_value_with_zero_or_space_padding() {
        let mut list = TestRecordList::new();

        let space_padded = TestRecord::spaced()
            .with_name("Space")
            .with_code("A00")
            .with_amount_int(1)
            .build();

        let zero_padded = TestRecord::builder()
            .with_name("Zero")
            .with_code("A00")
            .with_amount_int(2)
            .build();

        let other = TestRecord::spaced()
            .with_name("Other")
            .with_code("A00XX")
            .with_amount_int(3)
            .build();

        list.insert(space_padded);
        list.insert(zero_padded);
        list.insert(other);

        let found = list
            .try_first_by(TestRecordField::Code, b"A00")
            .unwrap()
            .unwrap();

        assert_eq!(
            found
                .get_field_string_trimmed(TestRecordField::Name)
                .unwrap(),
            "Zero"
        );
        assert!(
            list.try_first_by(TestRecordField::Code, b"A00X")
                .unwrap()
                .is_none()
        );
    }

    /// `try_first_by` がフィールド幅を超える検索値をエラーにすることを確認します。
    #[test]
    fn test_try_first_by_reports_overflow_for_too_long_value() {
        let list = TestRecordList::new();
        let err = list
            .try_first_by(TestRecordField::Code, b"A00001")
            .unwrap_err();

        assert_eq!(
            err,
            Error::FieldOverflow {
                field: "code",
                size: TestRecord::FIELD_SIZE_CODE,
                actual: 6,
            }
        );
    }

    /// `try_find_by_prefix` が後続バイトに関係なく先頭一致で固定長フィールドを取得できることを確認します。
    #[test]
    fn test_try_find_by_prefix_matches_any_trailing_bytes() {
        let mut list = TestRecordList::new();

        let space_padded = TestRecord::spaced()
            .with_name("Space")
            .with_code("A00")
            .with_amount_int(1)
            .build();

        let zero_padded = TestRecord::builder()
            .with_name("Zero")
            .with_code("A00")
            .with_amount_int(2)
            .build();

        let other = TestRecord::spaced()
            .with_name("Other")
            .with_code("A00XX")
            .with_amount_int(3)
            .build();

        let different_prefix = TestRecord::spaced()
            .with_name("Different")
            .with_code("B00XX")
            .with_amount_int(4)
            .build();

        list.insert(space_padded);
        list.insert(zero_padded);
        list.insert(other);
        list.insert(different_prefix);

        let found = list
            .try_find_by_prefix(TestRecordField::Code, b"A00")
            .unwrap();
        let mut names: Vec<_> = found
            .iter()
            .map(|record| {
                record
                    .get_field_string_trimmed(TestRecordField::Name)
                    .unwrap()
            })
            .collect();
        names.sort();

        assert_eq!(
            names,
            vec!["Other".to_string(), "Space".to_string(), "Zero".to_string()]
        );
    }

    /// `try_find_by_prefix` がフィールド幅を超える検索値をエラーにすることを確認します。
    #[test]
    fn test_try_find_by_prefix_reports_overflow_for_too_long_value() {
        let list = TestRecordList::new();
        let err = list
            .try_find_by_prefix(TestRecordField::Code, b"A00001")
            .unwrap_err();

        assert_eq!(
            err,
            Error::FieldOverflow {
                field: "code",
                size: TestRecord::FIELD_SIZE_CODE,
                actual: 6,
            }
        );
    }

    /// `try_first_by_prefix` が先頭一致した固定長フィールドのうち昇順で最初のレコードを返すことを確認します。
    #[test]
    fn test_try_first_by_prefix_returns_first_matching_record() {
        let mut list = TestRecordList::new();

        let later = TestRecord::spaced()
            .with_name("Later")
            .with_code("A01AA")
            .with_amount_int(1)
            .build();

        let first = TestRecord::spaced()
            .with_name("First")
            .with_code("A00")
            .with_amount_int(2)
            .build();

        let with_suffix = TestRecord::spaced()
            .with_name("Suffix")
            .with_code("A00XX")
            .with_amount_int(3)
            .build();

        let different_prefix = TestRecord::spaced()
            .with_name("Different")
            .with_code("B00XX")
            .with_amount_int(4)
            .build();

        list.insert(later);
        list.insert(first);
        list.insert(with_suffix);
        list.insert(different_prefix);

        let found = list
            .try_first_by_prefix(TestRecordField::Code, b"A0")
            .unwrap()
            .unwrap();

        assert_eq!(
            found
                .get_field_string_trimmed(TestRecordField::Name)
                .unwrap(),
            "First"
        );
        assert_eq!(
            found
                .get_field_string_trimmed(TestRecordField::Code)
                .unwrap(),
            "A00"
        );
    }

    /// `try_first_by_prefix` がフィールド幅を超える検索値をエラーにすることを確認します。
    #[test]
    fn test_try_first_by_prefix_reports_overflow_for_too_long_value() {
        let list = TestRecordList::new();
        let err = list
            .try_first_by_prefix(TestRecordField::Code, b"A00001")
            .unwrap_err();

        assert_eq!(
            err,
            Error::FieldOverflow {
                field: "code",
                size: TestRecord::FIELD_SIZE_CODE,
                actual: 6,
            }
        );
    }

    // ---- apply の冪等性テスト ----

    /// 同じデータを繰り返し適用しても結果が変わらないことを確認します。
    #[test]
    fn test_apply_idempotent() {
        // 同じデータを2回 apply しても結果は同じ
        let data = b"HelloWorldABCDE12345678";
        let rec1 = TestRecord::spaced().apply_bytes(data);
        let rec2 = TestRecord::spaced().apply_bytes(data).apply_bytes(data);

        assert_eq!(rec1.to_bytes(), rec2.to_bytes());
    }

    /// UTF-8 文字列が固定バイト幅どおり保持され、余白だけ trim されることを確認します。
    #[test]
    fn test_utf8_field_keeps_exact_bytes_and_trims_padding_spaces() {
        let mut record = TestRecord::builder().build();
        record.fill_space();
        record.set_field_str(TestRecordField::Name, "あいう ");

        let expected = "あいう ".as_bytes();

        assert_eq!(expected.len(), TestRecord::FIELD_SIZE_NAME);
        assert_eq!(record.name().len(), TestRecord::FIELD_SIZE_NAME);
        assert_eq!(record.name(), expected);
        assert_eq!(
            record.name(),
            &[0xe3, 0x81, 0x82, 0xe3, 0x81, 0x84, 0xe3, 0x81, 0x86, b' ']
        );
        let name_text = record.name_str().unwrap();
        assert_eq!(name_text, "あいう ");
        assert_eq!(name_text.len(), TestRecord::FIELD_SIZE_NAME);
        assert_eq!(name_text.chars().count(), 4);
        assert_eq!(
            record.get_field_trimmed(TestRecordField::Name).unwrap(),
            "あいう"
        );
        assert_eq!(
            record
                .get_field_string_trimmed(TestRecordField::Name)
                .unwrap(),
            "あいう".to_string()
        );
    }

    /// UTF-8 を含む固定長レコードを `Reader` がバイト境界で正確に読み取ることを確認します。
    #[test]
    fn test_utf8_reader_reads_fixed_byte_records_exactly() {
        use fixed_record_main::Reader;
        use std::io::{BufReader, Cursor};

        let first = TestRecord::builder()
            .with_name("あいう ")
            .with_code("JP001")
            .with_amount_int(1)
            .build();
        let second = TestRecord::builder()
            .with_name("Rust")
            .with_code("EN001")
            .with_amount_int(2)
            .build();

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&first.to_bytes());
        bytes.extend_from_slice(b"\n");
        bytes.extend_from_slice(&second.to_bytes());

        let mut reader = Reader::<_, TestRecord>::new(BufReader::new(Cursor::new(bytes)));

        let read_first = reader.next().unwrap().unwrap();
        assert_eq!(read_first.name(), "あいう ".as_bytes());
        assert_eq!(read_first.name().len(), TestRecord::FIELD_SIZE_NAME);
        assert_eq!(
            read_first.get_field_trimmed(TestRecordField::Name).unwrap(),
            "あいう"
        );
        assert_eq!(read_first.code(), b"JP001");
        assert_eq!(read_first.amount(), b"00000001");

        let read_second = reader.next().unwrap().unwrap();
        assert_eq!(
            read_second
                .get_field_trimmed(TestRecordField::Name)
                .unwrap(),
            "Rust"
        );
        assert_eq!(read_second.code(), b"EN001");
        assert_eq!(read_second.amount(), b"00000002");

        assert!(reader.next().is_none());
    }

    /// フィールド境界が UTF-8 文字の途中に来た場合に文字列変換が失敗することを確認します。
    #[test]
    fn test_utf8_field_split_at_byte_boundary_reports_utf8_error() {
        let record = SplitUtf8Record::parse("あいう ".as_bytes()).unwrap();

        assert_eq!(SplitUtf8Record::TOTAL_LEN, 10);
        assert_eq!(record.name(), &"あいう ".as_bytes()[..8]);
        assert_eq!(record.rest(), &"あいう ".as_bytes()[8..10]);
        assert!(matches!(record.name_str(), Err(Error::Utf8Error)));
        assert!(matches!(
            record.get_field_str(SplitUtf8RecordField::Name),
            Err(Error::Utf8Error)
        ));
    }

    /// UTF-8 がフィールド幅に収まる場合に `Writer` と `Reader` で往復できることを確認します。
    #[test]
    fn test_utf8_writer_reader_round_trip_when_field_width_is_large_enough() {
        use fixed_record_main::{Reader, Writer};
        use std::io::{BufReader, Cursor};

        let record = TestRecord::builder()
            .with_name("あいう ")
            .with_code("JP002")
            .with_amount_int(300)
            .build();

        let mut bytes = Vec::new();
        let mut writer = Writer::new(&mut bytes);
        writer.write_record(&record).unwrap();
        writer.flush().unwrap();

        let mut reader = Reader::<_, TestRecord>::new(BufReader::new(Cursor::new(bytes)));
        let read_back = reader.next().unwrap().unwrap();

        assert_eq!(read_back.name(), "あいう ".as_bytes());
        assert_eq!(read_back.name_str().unwrap(), "あいう ");
        assert_eq!(
            read_back.get_field_trimmed(TestRecordField::Name).unwrap(),
            "あいう"
        );
        assert_eq!(read_back.code(), b"JP002");
        assert_eq!(read_back.amount(), b"00000300");
    }

    /// `unchecked` feature 有効時に unsafe なゼロコピー系 API が利用できることを確認します。
    #[cfg(feature = "unchecked")]
    #[test]
    fn test_unchecked_feature_methods_are_available() {
        let data = b"HelloWorldABCDE12345678";

        let rec = unsafe { TestRecord::parse_unchecked(data).unwrap() };
        assert_eq!(rec.to_bytes(), *data);

        let rec_ref = unsafe { TestRecord::from_bytes_unchecked(data).unwrap() };
        assert_eq!(rec_ref.name(), b"HelloWorld");

        let raw = unsafe { rec.as_bytes_unchecked() };
        assert_eq!(raw, data);
    }
}

/// サンプルアプリとして固定長レコードの主要 API を順番に実行します。
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- 1. 既存のパース機能 ---
    let raw_data = "00000001Tanaka Tarou    025";
    println!("--- 1. Basic Access ---");
    let user_ref = User::parse_str(raw_data)?;
    println!(
        "ID: {}, Name: {}, Age: {}\n",
        user_ref.id_str()?,
        user_ref.name_str()?,
        user_ref.age_str()?
    );

    // --- 2. メタ情報のテスト (前回分) ---
    println!("--- 2. Meta Information Test ---");
    println!("Total Length: {}", User::TOTAL_LEN);
    println!("Field 'Name' size: {}\n", User::size_of(UserField::Name));

    // --- 3. 新機能: ビルダーパターンと初期化 ---
    println!("--- 3. Builder & Initialization Test ---");

    // builder() (内部で cleared() を使用) と with_... メソッドのテスト
    let new_user = User::builder()
        .with_id("999") // 文字列を先頭から上書き
        .with_name("Alice") // 文字列を先頭から上書き
        .with_age_int(20) // 数値をセット（左ゼロ埋め: "020"）
        .build();

    println!("Created by Builder:");
    let new_user_bytes = new_user.to_bytes();
    println!("  Raw bytes: \"{}\"", std::str::from_utf8(&new_user_bytes)?);
    println!("  ID field   : \"{}\"", new_user.id_str()?);
    println!("  Age field  : \"{}\"\n", new_user.age_str()?);

    // zeroed() / spaced() / cleared() の直接比較
    let z = User::zeroed();
    let s = User::spaced();
    let c = User::cleared();
    println!("Initialization Comparison:");
    println!("  zeroed bytes: {:?}", &z.to_bytes()[..5]); // [0, 0, 0, 0, 0]
    println!("  spaced bytes: {:?}", &s.to_bytes()[..5]); // [32, 32, 32, 32, 32] (0x20)
    println!("  cleared bytes: {:?}", &c.to_bytes()[..5]); // default CLEAR_BYTE is 0x00

    // --- 4. シリアライズ (to_bytes) ---
    println!("\n--- 4. Serialization Test ---");
    let byte_array: [u8; 27] = new_user.to_bytes();
    println!("Copied byte array length: {}", byte_array.len());
    assert_eq!(byte_array.len(), User::TOTAL_LEN);

    // --- 5. デフォルト値のテスト ---
    let default_user = User::default();
    println!("\n--- 5. Default Trait Test ---");
    println!(
        "Default user (cleared) valid: {}",
        default_user.id().iter().all(|&b| b == 0)
    );

    // --- 6. フィールド操作（動的アクセス）のテスト ---
    println!("\n--- 6. Dynamic Field Access Test ---");
    let mut user = User::parse_str("00000001Tanaka Tarou    025")?;

    // get_field_as<T> で数値として取得
    let age_num: u32 = user.get_field_as(UserField::Age)?;
    println!("Parsed Age as u32: {}", age_num);

    // get_field_trimmed で余計なスペースを排除
    let name_trimmed = user.get_field_trimmed(UserField::Name)?;
    println!("Trimmed Name: \"{}\"", name_trimmed);

    // set_field_str による動的更新
    user.set_field_str(UserField::Name, "Sato Jiro");
    println!("After set_field_str: \"{}\"", user.name_str()?);

    // fill_space のテスト
    user.fill_space();
    println!(
        "After fill_space, Name is empty: {}",
        user.name().iter().all(|&b| b == b' ')
    );

    // --- 7. 一括適用・流し込み (Apply系) のテスト ---
    println!("\n--- 7. Apply & Bulk Injection Test ---");

    // apply_str で先頭から一気に流し込む
    // ID(8) + Name(16) + Age(3) = 27bytes
    let bulk_data = "99999999Yamada Hanako   080";
    let bulk_user = User::spaced().apply_str(bulk_data);

    println!("Bulk Applied Result:");
    println!(
        "  ID: {}, Name: {}, Age: {}",
        bulk_user.id_str()?,
        bulk_user.name_str()?,
        bulk_user.age_str()?
    );

    // apply_bytes_from で特定のフィールド以降を上書き
    // Nameフィールド(offset 8) から 19バイト分(16+3)を流し込む
    let partial_data = "Suzuki Ichiro   051";
    let final_user = bulk_user.apply_bytes_from(UserField::Name, partial_data.as_bytes());

    println!("\nPartial Applied Result (From Name):");
    println!("  ID: {} (Unchanged)", final_user.id_str()?);
    println!("  Name: {}", final_user.name_str()?);
    println!("  Age:  {}", final_user.age_str()?);

    // apply クロージャによるカスタム加工
    let custom_user = final_user.apply(|u| {
        u.set_field_str(UserField::Id, "NEW-ID");
        // 条件に応じた複雑な処理も可能
        if let Ok(age) = u.get_field_as::<i32>(UserField::Age) {
            let next_age = age + 1;
            // 数値をフォーマットして直接 set する
            let s = format!("{:0>3}", next_age);
            u.set_field_str(UserField::Age, &s);
        }
    });

    println!("\nCustom Apply (ID update & Age +1):");
    println!(
        "  ID: {}, Age: {}",
        custom_user.id_str()?,
        custom_user.age_str()?
    );

    println!("\nAll advanced systems green! 🚀");
    Ok(())
}
