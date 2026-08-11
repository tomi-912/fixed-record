use fixed_record::prelude::*;

#[fixed_record]
enum NotARecord {
    Value,
}

/// compile-fail fixture の空エントリポイントです。
fn main() {}
