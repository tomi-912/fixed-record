use fixed_record::prelude::*;

#[fixed_record]
enum NotARecord {
    Value,
}

/// Empty entry point for a compile-fail fixture.
/// compile-fail fixture の空エントリポイントです。
fn main() {}
