use crate::Error;

/// 固定長レコードとして扱える型の共通インターフェースです。
///
/// 通常は `#[fixed_record]` attribute macro によって自動実装されます。
pub trait FixedRecord {
    /// シーケンスチェックで指定できるフィールド enum の型です。
    type Field: Copy + Eq;

    /// レコード全体のバイト長です。
    const TOTAL_LEN: usize;

    /// バイト列からレコードを作成します。
    fn parse(src: &[u8]) -> Result<Self, Error>
    where
        Self: Sized;

    /// レコードを固定長バイト列へコピーして返します。
    fn to_bytes(&self) -> Vec<u8>;

    /// 指定フィールドの定義名を返します。
    fn field_name(field: Self::Field) -> &'static str;

    /// 指定フィールドのバイト列を返します。
    fn field_bytes(&self, field: Self::Field) -> &[u8];
}

/// Reader のシーケンスチェックに指定できるフィールド配列です。
pub trait SequenceFields<T: FixedRecord> {
    /// シーケンスチェック対象フィールドを Vec にして返します。
    fn to_sequence_fields(self) -> Vec<T::Field>;
}
