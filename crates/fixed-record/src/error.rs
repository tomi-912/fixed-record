use std::{fmt, io};

/// 固定長データのパース処理や操作中に発生するエラーを表します。
#[derive(Debug)]
pub enum Error {
    /// 入力データの長さが、期待される固定長サイズよりも短い場合。
    ///
    /// 例えば、10バイト必要なフィールドに対して、5バイトのデータしか与えられなかった場合などに発生します。
    TooShort,

    /// バイト列から文字列（UTF-8）への変換に失敗した場合。
    ///
    /// 固定長フィールド内に、UTF-8として不正なバイトシーケンスが含まれている場合に発生します。
    /// (Shift_JISなど他のエンコーディングを扱う場合は、バイト列として取得し別途デコードしてください)
    Utf8Error,
    /// 固定長レコードの途中で入力が終わった場合。
    IncompleteRecord {
        expected: usize,
        actual: usize,
    },
    /// Reader / Writer などの I/O 処理中に発生したエラー。
    Io(io::Error),
    /// フィールドに入れる値が固定長幅を超えた場合。
    FieldOverflow {
        field: &'static str,
        size: usize,
        actual: usize,
    },
    /// Reader のシーケンスチェックで、現在レコードが前回レコードより小さい、または同一禁止設定で同一だった場合。
    SequenceError {
        fields: Vec<&'static str>,
        previous: Vec<Vec<u8>>,
        current: Vec<Vec<u8>>,
    },
    AlignmentError,
    ParseError,
}

impl fmt::Display for Error {
    /// エラー内容を利用者向けの短いメッセージとして整形します。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::TooShort => write!(f, "input data is too short for the fixed length"),
            Error::Utf8Error => write!(f, "input data contains invalid UTF-8 sequence"),
            Error::IncompleteRecord { expected, actual } => write!(
                f,
                "incomplete fixed record: expected {expected} bytes, got {actual} bytes"
            ),
            Error::Io(err) => write!(f, "I/O error while processing fixed record: {err}"),
            Error::FieldOverflow {
                field,
                size,
                actual,
            } => write!(
                f,
                "field `{field}` is too wide for the fixed length: expected at most {size} bytes, got {actual} bytes"
            ),
            Error::SequenceError {
                fields,
                previous,
                current,
            } => write!(
                f,
                "record sequence is not ascending for fields {fields:?}: previous={previous:?}, current={current:?}"
            ),
            Error::AlignmentError => write!(f, "Alignment error"),
            Error::ParseError => write!(f, "failed to parse field value"),
        }
    }
}

impl PartialEq for Error {
    /// I/O エラーは `ErrorKind` で比較し、それ以外は variant と値で比較します。
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Error::TooShort, Error::TooShort) => true,
            (Error::Utf8Error, Error::Utf8Error) => true,
            (
                Error::IncompleteRecord {
                    expected: left_expected,
                    actual: left_actual,
                },
                Error::IncompleteRecord {
                    expected: right_expected,
                    actual: right_actual,
                },
            ) => left_expected == right_expected && left_actual == right_actual,
            (Error::Io(left), Error::Io(right)) => left.kind() == right.kind(),
            (
                Error::FieldOverflow {
                    field: left_field,
                    size: left_size,
                    actual: left_actual,
                },
                Error::FieldOverflow {
                    field: right_field,
                    size: right_size,
                    actual: right_actual,
                },
            ) => {
                left_field == right_field && left_size == right_size && left_actual == right_actual
            }
            (
                Error::SequenceError {
                    fields: left_fields,
                    previous: left_previous,
                    current: left_current,
                },
                Error::SequenceError {
                    fields: right_fields,
                    previous: right_previous,
                    current: right_current,
                },
            ) => {
                left_fields == right_fields
                    && left_previous == right_previous
                    && left_current == right_current
            }
            (Error::AlignmentError, Error::AlignmentError) => true,
            (Error::ParseError, Error::ParseError) => true,
            _ => false,
        }
    }
}

impl Eq for Error {}

impl std::error::Error for Error {
    /// ラップしている I/O エラーがあれば原因として返します。
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    /// 標準 I/O エラーを固定長レコード用エラーへ変換します。
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}
