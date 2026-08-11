use fixed_record::prelude::*;

#[fixed_record]
pub struct Header {
    pub code: Fixed<4>,
    pub name: Fixed<8>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let header = Header::builder()
        .with_code("A001")
        .with_name("Tanaka")
        .build();

    assert_eq!(Header::TOTAL_LEN, 12);
    assert_eq!(header.get_field_trimmed(HeaderField::Name)?, "Tanaka");
    assert_eq!(header.to_bytes(), *b"A001Tanaka\0\0");

    Ok(())
}
