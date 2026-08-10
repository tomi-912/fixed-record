use std::fmt;

/// 固定長データのパース処理や操作中に発生するエラーを表します。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    AlignmentError,
    ParseError,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::TooShort => write!(f, "input data is too short for the fixed length"),
            Error::Utf8Error => write!(f, "input data contains invalid UTF-8 sequence"),
            Error::AlignmentError => write!(f, "Alignment error"),
            Error::ParseError => write!(f, "failed to parse field value"),
        }
    }
}

impl std::error::Error for Error {}
