use crate::Error;

/// Common interface for types that can be handled as fixed-width records.
/// 固定長レコードとして扱える型の共通インターフェースです。
///
/// This is usually implemented by the `#[fixed_record]` attribute macro.
/// 通常は `#[fixed_record]` attribute macro によって自動実装されます。
pub trait FixedRecord {
    /// The field enum type that can be used for sequence checks.
    /// シーケンスチェックで指定できるフィールド enum の型です。
    type Field: Copy + Eq;

    /// Total byte length of the record.
    /// レコード全体のバイト長です。
    const TOTAL_LEN: usize;

    /// Creates a record from a byte slice.
    /// バイト列からレコードを作成します。
    fn parse(src: &[u8]) -> Result<Self, Error>
    where
        Self: Sized;

    /// Copies the record into its fixed-width byte representation.
    /// レコードを固定長バイト列へコピーして返します。
    fn to_bytes(&self) -> Vec<u8>;

    /// Returns the declaration name of the specified field.
    /// 指定フィールドの定義名を返します。
    fn field_name(field: Self::Field) -> &'static str;

    /// Returns the bytes stored in the specified field.
    /// 指定フィールドのバイト列を返します。
    fn field_bytes(&self, field: Self::Field) -> &[u8];
}

/// Field arrays that can be passed to `Reader` sequence checks.
/// `Reader` のシーケンスチェックに指定できるフィールド配列です。
pub trait SequenceFields<T: FixedRecord> {
    /// Converts the sequence-check field list into a `Vec`.
    /// シーケンスチェック対象フィールドを `Vec` にして返します。
    fn to_sequence_fields(self) -> Vec<T::Field>;
}
