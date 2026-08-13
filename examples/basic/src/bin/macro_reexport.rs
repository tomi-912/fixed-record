use fixed_record::{Fixed, FixedRecord, fixed_record};

#[fixed_record]
struct Customer {
    id: Fixed<6>,
    name: Fixed<12>,
}

/// Calls the generated `FixedRecord` implementation through a trait bound.
/// 生成された `FixedRecord` 実装を trait 境界越しに呼び出します。
fn bytes_from_generated_trait<T: FixedRecord>(record: &T) -> Vec<u8> {
    record.to_bytes()
}

/// Runs an example that explicitly imports the proc macro re-exported by the main crate.
/// 本体クレートから再エクスポートされた proc macro を明示 import するサンプルを実行します。
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let customer = Customer::builder()
        .with_id("C00001")
        .with_name("Tanaka")
        .build();

    println!("fixed_record macro re-export example");
    println!("crate role: users only depend on fixed_record");
    println!("generated enum name: CustomerField");
    println!(
        "name field offset: {}",
        Customer::offset_of(CustomerField::Name)
    );
    println!("name field size: {}", CustomerField::Name.size());
    println!(
        "trimmed name: {}",
        customer.get_field_trimmed(CustomerField::Name)?
    );
    println!(
        "bytes from generated FixedRecord impl: {:?}",
        bytes_from_generated_trait(&customer)
    );

    Ok(())
}
