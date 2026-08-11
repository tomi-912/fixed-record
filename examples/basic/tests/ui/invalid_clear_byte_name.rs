use fixed_record::prelude::*;

#[fixed_record(clear_byte = TAB)]
pub struct User {
    pub id: Fixed<8>,
}

/// compile-fail fixture の空エントリポイントです。
fn main() {}
