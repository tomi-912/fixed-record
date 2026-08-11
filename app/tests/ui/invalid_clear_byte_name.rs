use fixed_record_main::prelude::*;

#[fixed_record_main(clear_byte = TAB)]
pub struct User {
    pub id: Fixed<8>,
}

/// compile-fail fixture の空エントリポイントです。
fn main() {}
