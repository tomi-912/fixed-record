use crate::{Error, FixedRecord};
use std::io::{self, BufRead, ErrorKind, Write};
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

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = vec![0u8; T::TOTAL_LEN];

        if let Err(e) = self.reader.read_exact(&mut buf) {
            if e.kind() == ErrorKind::UnexpectedEof {
                return None;
            }
            return None;
        }

        let record = T::parse(&buf);

        loop {
            let available = match self.reader.fill_buf() {
                Ok(bytes) => bytes,
                Err(_) => break,
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
