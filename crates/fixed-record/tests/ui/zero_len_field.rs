use fixed_record::prelude::*;

#[fixed_record]
pub struct User {
    pub id: Fixed<0>,
}

/// Empty entry point for a compile-fail fixture.
/// compile-fail fixture の空エントリポイントです。
fn main() {}
