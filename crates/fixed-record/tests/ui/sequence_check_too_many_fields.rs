use fixed_record::prelude::*;
use std::io::{BufReader, Cursor};

#[fixed_record]
pub struct Payment {
    pub code: Fixed<4>,
}

/// レコードのフィールド数より長い配列はシーケンスチェックに指定できないことを確認します。
fn main() {
    let bytes = Vec::new();
    let _reader = Reader::<_, Payment>::new(BufReader::new(Cursor::new(bytes)))
        .with_sequence_check([PaymentField::Code, PaymentField::Code]);
}
