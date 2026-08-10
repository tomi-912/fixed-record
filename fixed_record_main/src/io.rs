use crate::{Error, FixedRecord};
use std::io::{self, BufRead, Write};
use std::marker::PhantomData;

/// 固定長レコードをストリームから順に読み込むイテレータです。
///
/// 各レコードの直後にある `\n` または `\r\n` は自動的に読み飛ばします。
pub struct Reader<R, T> {
    reader: R,
    _marker: PhantomData<T>,
}

impl<R: BufRead, T: FixedRecord> Reader<R, T> {
    /// 新しいリーダーを作成します。
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            _marker: PhantomData,
        }
    }
}

impl<R: BufRead, T: FixedRecord> Iterator for Reader<R, T> {
    type Item = Result<T, Error>;

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

        let record = T::parse(&buf);

        loop {
            let available = match self.reader.fill_buf() {
                Ok(bytes) => bytes,
                Err(err) => return Some(Err(Error::Io(err))),
            };
            if available.is_empty() {
                break;
            }

            match available[0] {
                b'\n' | b'\r' => self.reader.consume(1),
                _ => break,
            }
        }

        Some(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, Cursor, ErrorKind, Read};

    #[derive(Debug, PartialEq, Eq)]
    struct TestRecord([u8; 4]);

    impl FixedRecord for TestRecord {
        const TOTAL_LEN: usize = 4;

        /// 4バイトの入力からテスト用レコードを作成します。
        fn parse(src: &[u8]) -> Result<Self, Error> {
            if src.len() < Self::TOTAL_LEN {
                return Err(Error::TooShort);
            }
            let mut bytes = [0; 4];
            bytes.copy_from_slice(&src[..4]);
            Ok(Self(bytes))
        }

        /// テスト用レコードの内部バイト列を返します。
        fn to_bytes(&self) -> Vec<u8> {
            self.0.to_vec()
        }
    }

    struct FillBufErrorAfterRead {
        cursor: Cursor<Vec<u8>>,
    }

    impl Read for FillBufErrorAfterRead {
        /// 内部カーソルから通常どおりバイト列を読み込みます。
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.cursor.read(buf)
        }
    }

    impl BufRead for FillBufErrorAfterRead {
        /// レコード読込後の改行読み飛ばしで I/O エラーを発生させます。
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            Err(io::Error::new(ErrorKind::Other, "fill_buf failed"))
        }

        /// テスト用なのでバッファ消費は何もしません。
        fn consume(&mut self, _amt: usize) {}
    }

    /// 入力が空のときに正常な終端として `None` を返すことを確認します。
    #[test]
    fn reader_returns_none_on_clean_eof() {
        let mut reader = Reader::<_, TestRecord>::new(BufReader::new(Cursor::new(Vec::new())));

        assert!(reader.next().is_none());
    }

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

    /// 改行読み飛ばし中の I/O エラーが `Error::Io` として返ることを確認します。
    #[test]
    fn reader_returns_io_error_from_fill_buf() {
        let mut reader = Reader::<_, TestRecord>::new(FillBufErrorAfterRead {
            cursor: Cursor::new(b"abcd".to_vec()),
        });

        let err = reader.next().unwrap().unwrap_err();
        assert!(matches!(err, Error::Io(_)));
    }
}

/// 固定長レコードをストリームへ書き出すライターです。
///
/// 書き込み時は NUL (`0x00`) をスペース (`0x20`) に置換し、レコード末尾に改行を付けます。
pub struct Writer<W> {
    writer: W,
    newline: &'static [u8],
}

impl<W: Write> Writer<W> {
    /// デフォルト改行コード `\n` でライターを作成します。
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            newline: b"\n",
        }
    }

    /// 改行コードを変更します。
    pub fn with_newline(mut self, newline: &'static [u8]) -> Self {
        self.newline = newline;
        self
    }

    /// レコードを1件書き込みます。
    pub fn write_record<T: FixedRecord>(&mut self, record: &T) -> io::Result<()> {
        let mut bytes = record.to_bytes();
        for byte in &mut bytes {
            if *byte == 0x00 {
                *byte = b' ';
            }
        }

        self.writer.write_all(&bytes)?;
        self.writer.write_all(self.newline)?;
        Ok(())
    }

    /// 内部ライターを flush します。
    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
