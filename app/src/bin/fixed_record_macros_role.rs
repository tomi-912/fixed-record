use fixed_record_macros::fixed_record_main;
use fixed_record_main::{Fixed, FixedRecord};

#[fixed_record_main]
pub struct Customer {
    pub id: Fixed<6>,
    pub name: Fixed<12>,
}

/// 生成された `FixedRecord` 実装を trait 境界越しに呼び出します。
fn bytes_from_generated_trait<T: FixedRecord>(record: &T) -> Vec<u8> {
    record.to_bytes()
}

/// proc macro クレートを直接 import するサンプルを実行します。
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let customer = Customer::builder()
        .with_id("C00001")
        .with_name("Tanaka")
        .build();

    println!("fixed_record_macros example");
    println!("crate role: this crate only provides the attribute macro");
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
