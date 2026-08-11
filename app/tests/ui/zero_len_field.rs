use fixed_record_main::prelude::*;

#[fixed_record_main]
pub struct User {
    pub id: Fixed<0>,
}

/// compile-fail fixture の空エントリポイントです。
fn main() {}
