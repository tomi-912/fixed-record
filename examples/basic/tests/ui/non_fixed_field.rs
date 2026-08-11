use fixed_record::prelude::*;

#[fixed_record]
pub struct User {
    pub id: String,
}

/// compile-fail fixture の空エントリポイントです。
fn main() {}
