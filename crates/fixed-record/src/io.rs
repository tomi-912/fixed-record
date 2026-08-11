use crate::{Error, FixedRecord, traits::SequenceFields};
use std::cmp::Ordering;
use std::io::{self, BufRead, Write};
use std::marker::PhantomData;

/// Separator written after each record by [`Writer`].
/// [`Writer`] が各レコードの後ろに書き出す区切りです。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordSeparator {
    /// Line feed (`\n`).
    /// LF (`\n`) です。
    Lf,
    /// Carriage return (`\r`).
    /// CR (`\r`) です。
    Cr,
    /// Carriage return and line feed (`\r\n`).
    /// CRLF (`\r\n`) です。
    Crlf,
    /// Comma (`,`).
    /// カンマ (`,`) です。
    Comma,
}

impl RecordSeparator {
    /// Returns the byte sequence for this separator.
    /// この区切りを表すバイト列を返します。
    pub const fn as_bytes(self) -> &'static [u8] {
        match self {
            Self::Lf => b"\n",
            Self::Cr => b"\r",
            Self::Crlf => b"\r\n",
            Self::Comma => b",",
        }
    }
}

/// Iterator that reads fixed-width records from a stream.
/// 固定長レコードをストリームから順に読み込むイテレータです。
///
/// A trailing `\n`, `\r`, `\r\n`, or `,` immediately after each record is skipped automatically.
/// 各レコードの直後にある `\n`、`\r`、`\r\n`、`,` は自動的に読み飛ばします。
///
/// # Examples
///
/// ```
/// use fixed_record::prelude::*;
/// use std::io::{BufReader, Cursor};
///
/// #[fixed_record]
/// pub struct Payment {
///     id: Fixed<4>,
///     amount: Fixed<6>,
/// }
///
/// let first = Payment::builder()
///     .with_id("A001")
///     .with_amount_int(1200)
///     .build();
/// let second = Payment::builder()
///     .with_id("A002")
///     .with_amount_int(3400)
///     .build();
///
/// let mut input = Vec::new();
/// input.extend_from_slice(&first.to_bytes());
/// input.extend_from_slice(b"\r\n");
/// input.extend_from_slice(&second.to_bytes());
/// input.push(b',');
///
/// let mut reader = Reader::<_, Payment>::new(BufReader::new(Cursor::new(input)));
///
/// assert_eq!(
///     reader.next().unwrap().unwrap().get_field_trimmed(PaymentField::Id).unwrap(),
///     "A001"
/// );
/// assert_eq!(
///     reader.next().unwrap().unwrap().get_field_trimmed(PaymentField::Amount).unwrap(),
///     "003400"
/// );
/// assert!(reader.next().is_none());
/// ```
pub struct Reader<R, T: FixedRecord> {
    reader: R,
    sequence_fields: Vec<T::Field>,
    allow_equal_sequence: bool,
    previous_sequence_key: Option<Vec<Vec<u8>>>,
    _marker: PhantomData<T>,
}

impl<R: BufRead, T: FixedRecord> Reader<R, T> {
    /// Creates a new reader.
    /// 新しいリーダーを作成します。
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            sequence_fields: Vec::new(),
            allow_equal_sequence: true,
            previous_sequence_key: None,
            _marker: PhantomData,
        }
    }

    /// Enables ascending sequence checks using the specified fields as the key.
    /// 指定フィールドをキーにした昇順シーケンスチェックを有効にします。
    pub fn with_sequence_check<F>(mut self, fields: F) -> Self
    where
        F: SequenceFields<T>,
    {
        self.set_sequence_check_fields(fields.to_sequence_fields());
        self
    }

    /// Configures ascending sequence checks using the specified fields as the key.
    /// 指定フィールドをキーにした昇順シーケンスチェックを設定します。
    pub fn with_sequence_check_options<F>(mut self, fields: F, allow_equal: bool) -> Self
    where
        F: SequenceFields<T>,
    {
        self.set_sequence_check_fields(fields.to_sequence_fields());
        self.allow_equal_sequence = allow_equal;
        self
    }

    /// Stores the fields used for sequence checks.
    /// シーケンスチェック対象フィールドを設定します。
    fn set_sequence_check_fields(&mut self, fields: Vec<T::Field>) {
        for (index, field) in fields.iter().enumerate() {
            if fields[index + 1..].contains(field) {
                panic!("duplicate sequence check field `{}`", T::field_name(*field));
            }
        }

        self.sequence_fields = fields;
    }

    /// Verifies that the loaded record is ordered after the previous record.
    /// 読み込んだレコードが前回レコード以降の順序になっているか確認します。
    fn check_sequence(&mut self, record: &T) -> Result<(), Error> {
        if self.sequence_fields.is_empty() {
            return Ok(());
        }

        let current_key: Vec<Vec<u8>> = self
            .sequence_fields
            .iter()
            .map(|field| record.field_bytes(*field).to_vec())
            .collect();

        if let Some(previous_key) = &self.previous_sequence_key {
            let ordering = current_key.cmp(previous_key);
            let is_error = ordering == Ordering::Less
                || (ordering == Ordering::Equal && !self.allow_equal_sequence);
            if is_error {
                return Err(Error::SequenceError {
                    fields: self
                        .sequence_fields
                        .iter()
                        .map(|field| T::field_name(*field))
                        .collect(),
                    previous: previous_key.clone(),
                    current: current_key,
                });
            }
        }

        self.previous_sequence_key = Some(current_key);
        Ok(())
    }
}

impl<R: BufRead, T: FixedRecord> Iterator for Reader<R, T> {
    type Item = Result<T, Error>;

    /// Reads the next fixed-width record, returning end-of-stream or an error when appropriate.
    /// 次の固定長レコードを読み込み、終端またはエラーを返します。
    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = vec![0u8; T::TOTAL_LEN];
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
                Err(err) => return Some(Err(Error::Io(err))),
            }
        }

        let record = match T::parse(&buf) {
            Ok(record) => record,
            Err(err) => return Some(Err(err)),
        };

        loop {
            let available = match self.reader.fill_buf() {
                Ok(bytes) => bytes,
                Err(err) => return Some(Err(Error::Io(err))),
            };
            if available.is_empty() {
                break;
            }

            if available.starts_with(b"\r\n") {
                self.reader.consume(2);
                continue;
            }

            match available[0] {
                b'\n' | b'\r' | b',' => self.reader.consume(1),
                _ => break,
            }
        }

        if let Err(err) = self.check_sequence(&record) {
            return Some(Err(err));
        }

        Some(Ok(record))
    }
}

/// Writer for fixed-width records.
/// 固定長レコードをストリームへ書き出すライターです。
///
/// During writing, NUL bytes (`0x00`) are replaced with spaces (`0x20`) and a separator is appended.
/// 書き込み時は NUL (`0x00`) をスペース (`0x20`) に置換し、レコード末尾に区切りを付けます。
///
/// # Examples
///
/// ```
/// use fixed_record::prelude::*;
///
/// #[fixed_record]
/// pub struct Payment {
///     id: Fixed<4>,
///     amount: Fixed<6>,
/// }
///
/// let first = Payment::builder()
///     .with_id("A001")
///     .with_amount_int(1200)
///     .build();
/// let second = Payment::builder()
///     .with_id("A002")
///     .with_amount_int(3400)
///     .build();
///
/// let mut output = Vec::new();
/// let mut writer = Writer::new(&mut output)
///     .with_separator(RecordSeparator::Comma);
///
/// writer.write_record(&first).unwrap();
/// writer.write_record(&second).unwrap();
/// writer.flush().unwrap();
///
/// let mut expected = Vec::new();
/// expected.extend_from_slice(&first.to_bytes());
/// expected.push(b',');
/// expected.extend_from_slice(&second.to_bytes());
/// expected.push(b',');
///
/// assert_eq!(output, expected);
/// ```
pub struct Writer<W> {
    writer: W,
    separator: &'static [u8],
}

impl<W: Write> Writer<W> {
    /// Creates a writer that uses `\n` as the default separator.
    /// デフォルト区切り `\n` でライターを作成します。
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            separator: RecordSeparator::Lf.as_bytes(),
        }
    }

    /// Changes the separator used after each record.
    /// レコードごとの区切りを変更します。
    pub fn with_separator(mut self, separator: RecordSeparator) -> Self {
        self.separator = separator.as_bytes();
        self
    }

    /// Writes one record.
    /// レコードを1件書き込みます。
    pub fn write_record<T: FixedRecord>(&mut self, record: &T) -> io::Result<()> {
        let mut bytes = record.to_bytes();
        for byte in &mut bytes {
            if *byte == 0x00 {
                *byte = b' ';
            }
        }

        self.writer.write_all(&bytes)?;
        self.writer.write_all(self.separator)?;
        Ok(())
    }

    /// Flushes the inner writer.
    /// 内部ライターを flush します。
    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::io::{BufReader, Cursor, Read};
    use std::rc::Rc;

    #[derive(Debug, PartialEq, Eq)]
    struct TestRecord([u8; 4]);

    impl FixedRecord for TestRecord {
        type Field = ();

        const TOTAL_LEN: usize = 4;

        /// Creates the test record from a 4-byte input.
        /// 4バイトの入力からテスト用レコードを作成します。
        fn parse(src: &[u8]) -> Result<Self, Error> {
            if src.len() < Self::TOTAL_LEN {
                return Err(Error::TooShort);
            }
            let mut bytes = [0; 4];
            bytes.copy_from_slice(&src[..4]);
            Ok(Self(bytes))
        }

        /// Returns the internal bytes of the test record.
        /// テスト用レコードの内部バイト列を返します。
        fn to_bytes(&self) -> Vec<u8> {
            self.0.to_vec()
        }

        /// Returns a fixed name because this test record has no named fields.
        /// テスト用レコードには名前付きフィールドがないため固定名を返します。
        fn field_name(_field: Self::Field) -> &'static str {
            "record"
        }

        /// Returns the entire test record as the sequence-check byte key.
        /// テスト用レコード全体をシーケンスチェック用バイト列として返します。
        fn field_bytes(&self, _field: Self::Field) -> &[u8] {
            &self.0
        }
    }

    struct FillBufErrorAfterRead {
        cursor: Cursor<Vec<u8>>,
    }

    impl Read for FillBufErrorAfterRead {
        /// Reads bytes normally from the inner cursor.
        /// 内部カーソルから通常どおりバイト列を読み込みます。
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.cursor.read(buf)
        }
    }

    impl BufRead for FillBufErrorAfterRead {
        /// Produces an I/O error while the reader skips trailing separators after a record.
        /// レコード読込後の区切り読み飛ばしで I/O エラーを発生させます。
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            Err(io::Error::other("fill_buf failed"))
        }

        /// Does nothing because this helper is only used by tests.
        /// テスト用なのでバッファ消費は何もしません。
        fn consume(&mut self, _amt: usize) {}
    }

    struct TrackingBufRead {
        cursor: Cursor<Vec<u8>>,
        consume_amounts: Rc<RefCell<Vec<usize>>>,
    }

    impl Read for TrackingBufRead {
        /// Reads bytes normally from the inner cursor.
        /// 内部カーソルから通常どおりバイト列を読み込みます。
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.cursor.read(buf)
        }
    }

    impl BufRead for TrackingBufRead {
        /// Returns the remaining bytes from the current cursor position.
        /// 現在のカーソル位置から残りのバイト列を返します。
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            let position = self.cursor.position() as usize;
            Ok(&self.cursor.get_ref()[position..])
        }

        /// Records the consumed byte count and advances the cursor.
        /// 消費したバイト数を記録してカーソルを進めます。
        fn consume(&mut self, amt: usize) {
            self.consume_amounts.borrow_mut().push(amt);
            let position = self.cursor.position() + amt as u64;
            self.cursor.set_position(position);
        }
    }

    /// Verifies that empty input is treated as a clean end-of-stream.
    /// 入力が空のときに正常な終端として `None` を返すことを確認します。
    #[test]
    fn reader_returns_none_on_clean_eof() {
        let mut reader = Reader::<_, TestRecord>::new(BufReader::new(Cursor::new(Vec::new())));

        assert!(reader.next().is_none());
    }

    /// Verifies that EOF in the middle of a record returns `IncompleteRecord`.
    /// レコード途中で EOF になったときに `IncompleteRecord` を返すことを確認します。
    #[test]
    fn reader_returns_incomplete_record_on_short_tail() {
        let mut reader = Reader::<_, TestRecord>::new(BufReader::new(Cursor::new(b"abc".to_vec())));

        let err = reader.next().unwrap().unwrap_err();
        assert!(matches!(
            err,
            Error::IncompleteRecord {
                expected: 4,
                actual: 3
            }
        ));
    }

    /// Verifies that an I/O error while skipping separators is returned as `Error::Io`.
    /// 区切り読み飛ばし中の I/O エラーが `Error::Io` として返ることを確認します。
    #[test]
    fn reader_returns_io_error_from_fill_buf() {
        let mut reader = Reader::<_, TestRecord>::new(FillBufErrorAfterRead {
            cursor: Cursor::new(b"abcd".to_vec()),
        });

        let err = reader.next().unwrap().unwrap_err();
        assert!(matches!(err, Error::Io(_)));
    }

    /// Verifies that CRLF separators are consumed as a single two-byte separator.
    /// CRLF 区切りが 2 バイトの区切りとしてまとめて消費されることを確認します。
    #[test]
    fn reader_consumes_crlf_separator_together() {
        let consume_amounts = Rc::new(RefCell::new(Vec::new()));
        let input = b"abcd\r\nefgh".to_vec();
        let mut reader = Reader::<_, TestRecord>::new(TrackingBufRead {
            cursor: Cursor::new(input),
            consume_amounts: Rc::clone(&consume_amounts),
        });

        assert_eq!(reader.next().unwrap().unwrap(), TestRecord(*b"abcd"));
        assert_eq!(reader.next().unwrap().unwrap(), TestRecord(*b"efgh"));
        assert!(reader.next().is_none());
        assert_eq!(*consume_amounts.borrow(), vec![2]);
    }
}
