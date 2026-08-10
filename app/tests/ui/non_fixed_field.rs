use fixed_record_main::prelude::*;

#[fixed_record_main]
pub struct User {
    pub id: String,
}

/// compile-fail fixture の空エントリポイントです。
fn main() {}
