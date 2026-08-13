//! Integration tests for generated fixed-record APIs.
//! 生成される fixed-record API の integration test です。

mod tests {
    use fixed_record::prelude::*;
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

    #[fixed_record]
    pub struct SplitUtf8Record {
        name: Fixed<8>,
        rest: Fixed<2>,
    }

    #[fixed_record(clear_byte = SPACE)]
    pub struct SpaceClearRecord {
        name: Fixed<6>,
    }

    #[fixed_record(clear_byte = ZERO)]
    pub struct ZeroClearRecord {
        name: Fixed<6>,
    }

    #[fixed_record(clear_byte = 0)]
    pub struct NumericZeroClearRecord {
        name: Fixed<6>,
    }
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
    #[test]
    fn test_apply_bytes_exact() {
        let data = b"HelloWorldABCDE12345678";
        let rec = TestRecord::spaced().apply_bytes(data);

        assert_eq!(rec.name(), b"HelloWorld");
        assert_eq!(rec.code(), b"ABCDE");
        assert_eq!(rec.amount(), b"12345678");
    }
    #[test]
    fn test_apply_bytes_shorter_than_total() {
        let data = b"HelloWorldABC";
        let rec = TestRecord::spaced().apply_bytes(data);

        assert_eq!(rec.name(), b"HelloWorld");
        assert_eq!(&rec.code()[..3], b"ABC");
        assert_eq!(&rec.code()[3..], b"  ");
        assert_eq!(rec.amount(), b"        ");
    }
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
    #[test]
    fn test_apply_bytes_from_first_field() {
        let data = b"HelloWorldABCDE12345678";
        let rec = TestRecord::spaced().apply_bytes_from(TestRecordField::Name, data);

        assert_eq!(rec.name(), b"HelloWorld");
        assert_eq!(rec.code(), b"ABCDE");
        assert_eq!(rec.amount(), b"12345678");
    }
    #[test]
    fn test_apply_bytes_from_middle_field() {
        let data = b"ABCDE12345678";
        let rec = TestRecord::spaced().apply_bytes_from(TestRecordField::Code, data);

        assert_eq!(rec.name(), b"          ");
        assert_eq!(rec.code(), b"ABCDE");
        assert_eq!(rec.amount(), b"12345678");
    }
    #[test]
    fn test_apply_bytes_from_last_field() {
        let data = b"99999999";
        let rec = TestRecord::spaced().apply_bytes_from(TestRecordField::Amount, data);

        assert_eq!(rec.name(), b"          ");
        assert_eq!(rec.code(), b"     ");
        assert_eq!(rec.amount(), b"99999999");
    }
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
    #[test]
    fn test_set_field_str_clears_with_default_space_before_write() {
        let mut rec = TestRecord::spaced();
        rec.set_field_str(TestRecordField::Name, "Bob");

        assert_eq!(rec.name(), b"Bob       ");
        assert_eq!(TestRecord::CLEAR_BYTE, b' ');
    }
    #[test]
    fn test_set_field_str_no_clear_keeps_existing_tail_bytes() {
        let mut rec = TestRecord::spaced().with_name("Alice");
        rec.set_field_str_no_clear(TestRecordField::Name, "Bo");

        assert_eq!(rec.name(), b"Boice     ");
    }
    #[test]
    fn test_with_str_keeps_existing_tail_bytes() {
        let rec = TestRecord::spaced().with_name("Alice").with_name("Bo");

        assert_eq!(rec.name(), b"Boice     ");
    }
    #[test]
    fn test_set_field_str_uses_configured_clear_byte() {
        let mut rec = SpaceClearRecord::zeroed();
        rec.set_field_str(SpaceClearRecordField::Name, "Bo");

        assert_eq!(SpaceClearRecord::CLEAR_BYTE, b' ');
        assert_eq!(rec.name(), b"Bo    ");
    }
    #[test]
    fn test_set_field_str_uses_zero_clear_byte() {
        let mut rec = ZeroClearRecord::spaced();
        rec.set_field_str(ZeroClearRecordField::Name, "Bo");

        assert_eq!(ZeroClearRecord::CLEAR_BYTE, 0x00);
        assert_eq!(rec.name(), b"Bo\0\0\0\0");
    }
    #[test]
    fn test_set_field_str_uses_numeric_zero_clear_byte() {
        let mut rec = NumericZeroClearRecord::spaced();
        rec.set_field_str(NumericZeroClearRecordField::Name, "Bo");

        assert_eq!(NumericZeroClearRecord::CLEAR_BYTE, 0x00);
        assert_eq!(rec.name(), b"Bo\0\0\0\0");
    }
    #[test]
    fn test_builder_and_default_use_default_clear_byte() {
        assert_eq!(TestRecord::builder().name(), b"          ");
        assert_eq!(TestRecord::default().name(), b"          ");
        assert_eq!(TestRecord::cleared().name(), b"          ");
    }
    #[test]
    fn test_builder_and_default_use_configured_clear_byte() {
        assert_eq!(SpaceClearRecord::builder().name(), b"      ");
        assert_eq!(SpaceClearRecord::default().name(), b"      ");
        assert_eq!(SpaceClearRecord::cleared().name(), b"      ");
    }
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
    #[test]
    fn test_truncated_int_setter_keeps_existing_truncation_behavior() {
        let rec = TestRecord::builder()
            .with_amount_int_truncated(123456789)
            .build();

        assert_eq!(rec.amount(), b"12345678");
    }
    #[test]
    fn test_truncated_signed_int_setter_keeps_existing_truncation_behavior() {
        let rec = TestRecord::builder()
            .with_amount_int_signed_truncated(-12345678)
            .build();

        assert_eq!(rec.amount(), b"-1234567");
    }
    #[test]
    fn test_reader_writer_and_fixed_record_trait() {
        use fixed_record::{FixedRecord, Reader, RecordSeparator, Writer};
        use std::io::{BufReader, Cursor};
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
        let mut writer = Writer::new(&mut buf).with_separator(RecordSeparator::Crlf);
        writer.write_record(&first).unwrap();
        writer.write_record(&second).unwrap();
        writer.flush().unwrap();
        drop(writer);

        let mut reader = Reader::<_, TestRecord>::new(BufReader::new(Cursor::new(buf)))
            .with_separator(RecordSeparator::Crlf);
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
    #[test]
    fn test_reader_writer_round_trip_file_with_lf_separator() {
        use fixed_record::{Reader, RecordSeparator, Writer};
        use std::fs::File;
        use std::io::BufReader;

        let path = std::env::temp_dir().join(format!(
            "fixed-record-lf-{}-{}.dat",
            std::process::id(),
            "generated-api"
        ));

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

        {
            let file = File::create(&path).unwrap();
            let mut writer = Writer::new(file).with_separator(RecordSeparator::Lf);
            writer.write_record(&first).unwrap();
            writer.write_record(&second).unwrap();
            writer.flush().unwrap();
        }

        let raw = std::fs::read(&path).unwrap();
        let mut expected = Vec::new();
        expected.extend_from_slice(&first.to_bytes());
        expected.push(b'\n');
        expected.extend_from_slice(&second.to_bytes());
        expected.push(b'\n');
        assert_eq!(raw, expected);

        let file = File::open(&path).unwrap();
        let mut reader = Reader::<_, TestRecord>::new(BufReader::new(file));
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

        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn test_reader_writer_round_trip_file_with_crlf_separator() {
        use fixed_record::{Reader, RecordSeparator, Writer};
        use std::fs::File;
        use std::io::BufReader;

        let path = std::env::temp_dir().join(format!(
            "fixed-record-crlf-{}-{}.dat",
            std::process::id(),
            "generated-api"
        ));

        let first = TestRecord::builder()
            .with_name("Carol")
            .with_code("C0001")
            .with_amount_int(30)
            .build();
        let second = TestRecord::builder()
            .with_name("Dave")
            .with_code("D0001")
            .with_amount_int(40)
            .build();

        {
            let file = File::create(&path).unwrap();
            let mut writer = Writer::new(file).with_separator(RecordSeparator::Crlf);
            writer.write_record(&first).unwrap();
            writer.write_record(&second).unwrap();
            writer.flush().unwrap();
        }

        let raw = std::fs::read(&path).unwrap();
        assert!(raw.windows(2).any(|bytes| bytes == b"\r\n"));

        let file = File::open(&path).unwrap();
        let mut reader = Reader::<_, TestRecord>::new(BufReader::new(file))
            .with_separator(RecordSeparator::Crlf);
        assert_eq!(
            reader
                .next()
                .unwrap()
                .unwrap()
                .get_field_trimmed(TestRecordField::Name)
                .unwrap(),
            "Carol"
        );
        assert_eq!(
            reader
                .next()
                .unwrap()
                .unwrap()
                .get_field_trimmed(TestRecordField::Name)
                .unwrap(),
            "Dave"
        );
        assert!(reader.next().is_none());

        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn test_reader_writer_round_trip_file_with_cr_separator() {
        use fixed_record::{Reader, RecordSeparator, Writer};
        use std::fs::File;
        use std::io::BufReader;

        let path = std::env::temp_dir().join(format!(
            "fixed-record-cr-{}-{}.dat",
            std::process::id(),
            "generated-api"
        ));

        let first = TestRecord::builder()
            .with_name("Ivy")
            .with_code("I0001")
            .with_amount_int(90)
            .build();
        let second = TestRecord::builder()
            .with_name("Judy")
            .with_code("J0001")
            .with_amount_int(100)
            .build();

        {
            let file = File::create(&path).unwrap();
            let mut writer = Writer::new(file).with_separator(RecordSeparator::Cr);
            writer.write_record(&first).unwrap();
            writer.write_record(&second).unwrap();
            writer.flush().unwrap();
        }

        let raw = std::fs::read(&path).unwrap();
        let mut expected = Vec::new();
        expected.extend_from_slice(&first.to_bytes());
        expected.push(b'\r');
        expected.extend_from_slice(&second.to_bytes());
        expected.push(b'\r');
        assert_eq!(raw, expected);

        let file = File::open(&path).unwrap();
        let mut reader =
            Reader::<_, TestRecord>::new(BufReader::new(file)).with_separator(RecordSeparator::Cr);
        assert_eq!(
            reader
                .next()
                .unwrap()
                .unwrap()
                .get_field_trimmed(TestRecordField::Name)
                .unwrap(),
            "Ivy"
        );
        assert_eq!(
            reader
                .next()
                .unwrap()
                .unwrap()
                .get_field_trimmed(TestRecordField::Name)
                .unwrap(),
            "Judy"
        );
        assert!(reader.next().is_none());

        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn test_reader_writer_round_trip_file_with_comma_separator() {
        use fixed_record::{Reader, RecordSeparator, Writer};
        use std::fs::File;
        use std::io::BufReader;

        let path = std::env::temp_dir().join(format!(
            "fixed-record-comma-{}-{}.dat",
            std::process::id(),
            "generated-api"
        ));

        let first = TestRecord::builder()
            .with_name("Eve")
            .with_code("E0001")
            .with_amount_int(50)
            .build();
        let second = TestRecord::builder()
            .with_name("Frank")
            .with_code("F0001")
            .with_amount_int(60)
            .build();

        {
            let file = File::create(&path).unwrap();
            let mut writer = Writer::new(file).with_separator(RecordSeparator::Comma);
            writer.write_record(&first).unwrap();
            writer.write_record(&second).unwrap();
            writer.flush().unwrap();
        }

        let raw = std::fs::read(&path).unwrap();
        let mut expected = Vec::new();
        expected.extend_from_slice(&first.to_bytes());
        expected.push(b',');
        expected.extend_from_slice(&second.to_bytes());
        expected.push(b',');
        assert_eq!(raw, expected);

        let file = File::open(&path).unwrap();
        let mut reader = Reader::<_, TestRecord>::new(BufReader::new(file))
            .with_separator(RecordSeparator::Comma);
        assert_eq!(
            reader
                .next()
                .unwrap()
                .unwrap()
                .get_field_trimmed(TestRecordField::Name)
                .unwrap(),
            "Eve"
        );
        assert_eq!(
            reader
                .next()
                .unwrap()
                .unwrap()
                .get_field_trimmed(TestRecordField::Name)
                .unwrap(),
            "Frank"
        );
        assert!(reader.next().is_none());

        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn test_reader_writer_round_trip_without_separator() {
        use fixed_record::{Reader, RecordSeparator, Writer};
        use std::io::{BufReader, Cursor};

        let first = TestRecord::builder()
            .with_name("Nina")
            .with_code("N0001")
            .with_amount_int(110)
            .build();
        let second = TestRecord::builder()
            .with_name("Owen")
            .with_code("O0001")
            .with_amount_int(120)
            .build();

        let mut bytes = Vec::new();
        let mut writer = Writer::new(&mut bytes).with_separator(RecordSeparator::None);
        writer.write_record(&first).unwrap();
        writer.write_record(&second).unwrap();
        writer.flush().unwrap();
        drop(writer);

        let mut expected = Vec::new();
        expected.extend_from_slice(&first.to_bytes());
        expected.extend_from_slice(&second.to_bytes());
        assert_eq!(bytes, expected);

        let mut reader = Reader::<_, TestRecord>::new(BufReader::new(Cursor::new(bytes)))
            .with_separator(RecordSeparator::None);
        assert_eq!(
            reader
                .next()
                .unwrap()
                .unwrap()
                .get_field_trimmed(TestRecordField::Name)
                .unwrap(),
            "Nina"
        );
        assert_eq!(
            reader
                .next()
                .unwrap()
                .unwrap()
                .get_field_trimmed(TestRecordField::Name)
                .unwrap(),
            "Owen"
        );
        assert!(reader.next().is_none());
    }
    #[test]
    fn test_reader_uses_configured_comma_separator() {
        use fixed_record::{Reader, RecordSeparator};
        use std::io::{BufReader, Cursor};

        let first = TestRecord::builder()
            .with_name("Grace")
            .with_code("G0001")
            .with_amount_int(70)
            .build();
        let second = TestRecord::builder()
            .with_name("Heidi")
            .with_code("H0001")
            .with_amount_int(80)
            .build();

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&first.to_bytes());
        bytes.push(b',');
        bytes.extend_from_slice(&second.to_bytes());
        bytes.push(b',');

        let mut reader = Reader::<_, TestRecord>::new(BufReader::new(Cursor::new(bytes)))
            .with_separator(RecordSeparator::Comma);
        assert_eq!(
            reader
                .next()
                .unwrap()
                .unwrap()
                .get_field_trimmed(TestRecordField::Name)
                .unwrap(),
            "Grace"
        );
        assert_eq!(
            reader
                .next()
                .unwrap()
                .unwrap()
                .get_field_trimmed(TestRecordField::Name)
                .unwrap(),
            "Heidi"
        );
        assert!(reader.next().is_none());
    }
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

        let code_index = list
            .indices
            .get(&TestRecordField::Code)
            .unwrap()
            .downcast_ref::<
                std::collections::BTreeMap<Fixed<5>, std::collections::BTreeSet<usize>>,
            >()
            .unwrap();
        assert!(code_index.contains_key(&Fixed::<5>::from_slice(b"B0001").unwrap()));

        assert!(list.remove(id_b));
        assert_eq!(list.len(), 1);
        let code_index = list
            .indices
            .get(&TestRecordField::Code)
            .unwrap()
            .downcast_ref::<
                std::collections::BTreeMap<Fixed<5>, std::collections::BTreeSet<usize>>,
            >()
            .unwrap();
        assert!(!code_index.contains_key(&Fixed::<5>::from_slice(b"B0001").unwrap()));
        assert_eq!(list.all_ids().len(), 2);
        list.vacuum();
        assert_eq!(list.all_ids(), vec![id_a]);
    }
    #[test]
    fn test_list_get_returns_only_active_record_by_id() {
        let mut list = TestRecordList::new();

        let id = list.insert(
            TestRecord::builder()
                .with_name("Alice")
                .with_code("A0001")
                .with_amount_int(10)
                .build(),
        );

        assert_eq!(
            list.get(id)
                .unwrap()
                .get_field_string_trimmed(TestRecordField::Name)
                .unwrap(),
            "Alice"
        );
        assert!(list.get(id + 1).is_none());

        assert!(list.remove(id));
        assert!(list.get(id).is_none());
        assert_eq!(list.all_ids(), vec![id]);
    }
    #[test]
    fn test_list_update_replaces_record_and_rebuilds_indices_for_id() {
        let mut list = TestRecordList::new();

        let id = list.insert(
            TestRecord::builder()
                .with_name("Alice")
                .with_code("A0001")
                .with_amount_int(10)
                .build(),
        );

        assert!(
            list.update(
                id,
                TestRecord::builder()
                    .with_name("Carol")
                    .with_code("C0001")
                    .with_amount_int(30)
                    .build(),
            )
        );

        assert!(list.find_by(TestRecordField::Code, *b"A0001").is_empty());

        let found = list.find_by(TestRecordField::Code, *b"C0001");
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0]
                .get_field_string_trimmed(TestRecordField::Name)
                .unwrap(),
            "Carol"
        );
        assert_eq!(
            list.get(id)
                .unwrap()
                .get_field_string_trimmed(TestRecordField::Amount)
                .unwrap(),
            "00000030"
        );
    }
    #[test]
    fn test_list_update_rejects_missing_or_deleted_id() {
        let mut list = TestRecordList::new();

        let id = list.insert(
            TestRecord::builder()
                .with_name("Alice")
                .with_code("A0001")
                .with_amount_int(10)
                .build(),
        );
        let replacement = TestRecord::builder()
            .with_name("Carol")
            .with_code("C0001")
            .with_amount_int(30)
            .build();

        assert!(!list.update(id + 1, replacement));

        assert!(list.remove(id));
        assert!(
            !list.update(
                id,
                TestRecord::builder()
                    .with_name("Dave")
                    .with_code("D0001")
                    .with_amount_int(40)
                    .build(),
            )
        );
        assert!(list.get(id).is_none());
        assert!(list.find_by(TestRecordField::Code, *b"D0001").is_empty());
    }
    #[test]
    fn test_try_find_by_matches_short_value_with_zero_or_space_padding() {
        let mut list = TestRecordList::new();

        let space_padded = TestRecord::spaced()
            .with_name("Space")
            .with_code("A00")
            .with_amount_int(1)
            .build();

        let zero_padded = TestRecord::zeroed()
            .with_name("Zero")
            .with_code("A00")
            .with_amount_int(2);

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
            "Space"
        );
        assert!(
            list.try_first_by(TestRecordField::Code, b"A00X")
                .unwrap()
                .is_none()
        );
    }
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
    #[test]
    fn test_apply_idempotent() {
        let data = b"HelloWorldABCDE12345678";
        let rec1 = TestRecord::spaced().apply_bytes(data);
        let rec2 = TestRecord::spaced().apply_bytes(data).apply_bytes(data);

        assert_eq!(rec1.to_bytes(), rec2.to_bytes());
    }
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
    #[test]
    fn test_utf8_reader_reads_fixed_byte_records_exactly() {
        use fixed_record::Reader;
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
        bytes.extend_from_slice(b"\n");

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
    #[test]
    fn test_reader_sequence_check_accepts_ascending_fields() {
        use fixed_record::Reader;
        use std::io::{BufReader, Cursor};

        let first = TestRecord::builder()
            .with_name("Alice")
            .with_code("A0001")
            .with_amount_int(10)
            .build();
        let second = TestRecord::builder()
            .with_name("Bob")
            .with_code("A0002")
            .with_amount_int(20)
            .build();

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&first.to_bytes());
        bytes.push(b'\n');
        bytes.extend_from_slice(&second.to_bytes());
        bytes.push(b'\n');

        let mut reader = Reader::<_, TestRecord>::new(BufReader::new(Cursor::new(bytes)))
            .with_sequence_check([TestRecordField::Code, TestRecordField::Amount]);

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
    #[test]
    fn test_reader_sequence_check_reports_descending_fields() {
        use fixed_record::Reader;
        use std::io::{BufReader, Cursor};

        let first = TestRecord::builder()
            .with_name("Bob")
            .with_code("A0002")
            .with_amount_int(20)
            .build();
        let second = TestRecord::builder()
            .with_name("Alice")
            .with_code("A0001")
            .with_amount_int(10)
            .build();

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&first.to_bytes());
        bytes.push(b'\n');
        bytes.extend_from_slice(&second.to_bytes());
        bytes.push(b'\n');

        let mut reader = Reader::<_, TestRecord>::new(BufReader::new(Cursor::new(bytes)))
            .with_sequence_check([TestRecordField::Code, TestRecordField::Amount]);

        assert!(reader.next().unwrap().is_ok());
        assert_eq!(
            reader.next().unwrap().unwrap_err(),
            Error::SequenceError {
                fields: vec!["code", "amount"],
                previous: vec![b"A0002".to_vec(), b"00000020".to_vec()],
                current: vec![b"A0001".to_vec(), b"00000010".to_vec()],
            }
        );
    }
    #[test]
    fn test_reader_sequence_check_can_reject_equal_fields() {
        use fixed_record::Reader;
        use std::io::{BufReader, Cursor};

        let first = TestRecord::builder()
            .with_name("Alice")
            .with_code("A0001")
            .with_amount_int(10)
            .build();
        let second = TestRecord::builder()
            .with_name("Bob")
            .with_code("A0001")
            .with_amount_int(10)
            .build();

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&first.to_bytes());
        bytes.push(b'\n');
        bytes.extend_from_slice(&second.to_bytes());
        bytes.push(b'\n');

        let mut allow_equal_reader =
            Reader::<_, TestRecord>::new(BufReader::new(Cursor::new(bytes.clone())))
                .with_sequence_check([TestRecordField::Code, TestRecordField::Amount]);

        assert!(allow_equal_reader.next().unwrap().is_ok());
        assert!(allow_equal_reader.next().unwrap().is_ok());

        let mut reject_equal_reader =
            Reader::<_, TestRecord>::new(BufReader::new(Cursor::new(bytes)))
                .with_sequence_check_options(
                    [TestRecordField::Code, TestRecordField::Amount],
                    false,
                );

        assert!(reject_equal_reader.next().unwrap().is_ok());
        assert_eq!(
            reject_equal_reader.next().unwrap().unwrap_err(),
            Error::SequenceError {
                fields: vec!["code", "amount"],
                previous: vec![b"A0001".to_vec(), b"00000010".to_vec()],
                current: vec![b"A0001".to_vec(), b"00000010".to_vec()],
            }
        );
    }
    #[test]
    #[should_panic(expected = "duplicate sequence check field `code`")]
    fn test_reader_sequence_check_rejects_duplicate_fields() {
        use fixed_record::Reader;
        use std::io::{BufReader, Cursor};

        let bytes = Vec::new();
        let _reader = Reader::<_, TestRecord>::new(BufReader::new(Cursor::new(bytes)))
            .with_sequence_check([TestRecordField::Code, TestRecordField::Code]);
    }
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
    #[test]
    fn test_utf8_writer_reader_round_trip_when_field_width_is_large_enough() {
        use fixed_record::{Reader, Writer};
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
        drop(writer);

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
    #[test]
    fn test_zerocopy_traits_are_available() {
        let data = b"HelloWorldABCDE12345678";

        let rec = TestRecord::ref_from_bytes(data).unwrap();
        assert_eq!(rec.name(), b"HelloWorld");
        assert_eq!(rec.as_bytes(), data);

        let owned = TestRecord::read_from_bytes(data).unwrap();
        assert_eq!(owned.to_bytes(), *data);

        let mut writable = *data;
        let rec_mut = TestRecord::mut_from_bytes(&mut writable).unwrap();
        rec_mut.set_field_str(TestRecordField::Code, "ZZ999");
        assert_eq!(&writable[10..15], b"ZZ999");
    }

    #[test]
    fn test_ref_from_bytes_prefix_accepts_trailing_bytes() {
        let data = b"HelloWorldABCDE12345678tail";

        let rec = TestRecord::ref_from_bytes_prefix(data).unwrap();

        assert_eq!(rec.name(), b"HelloWorld");
        assert_eq!(rec.as_bytes(), b"HelloWorldABCDE12345678");
    }

    #[test]
    fn test_ref_from_bytes_prefix_reports_too_short() {
        let err = TestRecord::ref_from_bytes_prefix(b"short").unwrap_err();

        assert_eq!(err, fixed_record::Error::TooShort);
    }

    #[test]
    fn test_ref_from_str_reads_exact_record_width() {
        let rec = TestRecord::ref_from_str("HelloWorldABCDE12345678").unwrap();

        assert_eq!(rec.name(), b"HelloWorld");
        assert_eq!(rec.as_str().unwrap(), "HelloWorldABCDE12345678");
    }

    #[test]
    fn test_ref_from_str_reports_size_errors() {
        let too_short = TestRecord::ref_from_str("short").unwrap_err();
        let too_long = TestRecord::ref_from_str("HelloWorldABCDE12345678tail").unwrap_err();

        assert_eq!(too_short, fixed_record::Error::TooShort);
        assert_eq!(too_long, fixed_record::Error::ParseError);
    }

    #[test]
    fn test_ref_from_str_prefix_accepts_trailing_text() {
        let rec = TestRecord::ref_from_str_prefix("HelloWorldABCDE12345678tail").unwrap();

        assert_eq!(rec.name_str().unwrap(), "HelloWorld");
        assert_eq!(rec.as_str().unwrap(), "HelloWorldABCDE12345678");
    }

    #[test]
    fn test_ref_from_str_handles_utf8_by_byte_width() {
        let rec = SplitUtf8Record::ref_from_str("あいX OK").unwrap();

        assert_eq!(rec.name(), "あいX ".as_bytes());
        assert_eq!(rec.name_str().unwrap(), "あいX ");
        assert_eq!(rec.rest(), b"OK");
        assert_eq!(rec.as_str().unwrap(), "あいX OK");
    }

    #[test]
    fn test_ref_from_str_prefix_can_split_trailing_utf8_text() {
        let rec = SplitUtf8Record::ref_from_str_prefix("あいX OK続き").unwrap();

        assert_eq!(rec.name_str().unwrap(), "あいX ");
        assert_eq!(rec.rest(), b"OK");
        assert_eq!(rec.as_str().unwrap(), "あいX OK");
    }

    #[test]
    fn test_record_as_str_reports_invalid_utf8() {
        let mut rec = TestRecord::builder()
            .with_name("HelloWorld")
            .with_code("ABCDE")
            .with_amount_int(12345678)
            .build();

        rec.as_mut_bytes()[0] = 0xFF;

        assert_eq!(rec.as_str().unwrap_err(), fixed_record::Error::Utf8Error);
    }

    #[test]
    fn test_zerocopy_as_bytes_views_record_memory() {
        let rec = TestRecord::builder()
            .with_name("HelloWorld")
            .with_code("ABCDE")
            .with_amount_int(12345678)
            .build();

        let bytes = rec.as_bytes();

        assert_eq!(bytes, b"HelloWorldABCDE12345678");
    }

    #[test]
    fn test_zerocopy_as_mut_bytes_updates_record_memory() {
        let mut rec = TestRecord::builder()
            .with_name("HelloWorld")
            .with_code("ABCDE")
            .with_amount_int(12345678)
            .build();

        let bytes = rec.as_mut_bytes();
        bytes[10..15].copy_from_slice(b"ZZ999");

        assert_eq!(rec.code(), b"ZZ999");
        assert_eq!(rec.to_bytes(), *b"HelloWorldZZ99912345678");
    }
}
