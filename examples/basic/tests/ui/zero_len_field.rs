use fixed_record::prelude::*;

#[fixed_record]
pub struct User {
    pub id: Fixed<0>,
}

/// compile-fail fixture の空エントリポイントです。
fn main() {}
