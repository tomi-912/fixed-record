use fixed_record::prelude::*;
use std::io::{BufReader, Cursor};

#[fixed_record]
pub struct User {
    pub id: Fixed<4>,
}

#[fixed_record]
pub struct Payment {
    pub code: Fixed<4>,
}

/// 対象レコードと異なる field enum はシーケンスチェックに指定できないことを確認します。
fn main() {
    let bytes = Vec::new();
    let _reader = Reader::<_, Payment>::new(BufReader::new(Cursor::new(bytes)))
        .with_sequence_check([UserField::Id]);
}
