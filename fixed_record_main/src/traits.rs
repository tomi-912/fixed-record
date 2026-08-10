use crate::Error;

/// 固定長レコードとして扱える型の共通インターフェースです。
///
/// 通常は `#[fixed_record_main]` attribute macro によって自動実装されます。
pub trait FixedRecord {
    /// レコード全体のバイト長です。
    const TOTAL_LEN: usize;

    /// バイト列からレコードを作成します。
    fn parse(src: &[u8]) -> Result<Self, Error>
    where
        Self: Sized;

    /// レコードを固定長バイト列へコピーして返します。
    fn to_bytes(&self) -> Vec<u8>;
}
