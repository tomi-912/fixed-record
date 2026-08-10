use fixed_record_main::prelude::*;

#[fixed_record_main]
enum NotARecord {
    Value,
}

/// compile-fail fixture の空エントリポイントです。
fn main() {}
