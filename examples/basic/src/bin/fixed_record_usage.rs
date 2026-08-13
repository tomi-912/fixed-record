use fixed_record::prelude::*;
use std::io::{BufReader, Cursor};

#[fixed_record]
struct Payment {
    bank_code: Fixed<4>,
    account_no: Fixed<7>,
    amount: Fixed<8>,
}

/// Runs a user-facing example that imports `fixed_record::prelude`.
/// `fixed_record::prelude` を使った利用者目線のサンプルを実行します。
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let payment = Payment::builder()
        .with_bank_code("0001")
        .with_account_no("1234567")
        .with_amount_int(2500)
        .build();

    println!("fixed_record example");
    println!("crate role: users import only this crate");
    println!("total bytes: {}", Payment::TOTAL_LEN);
    println!("amount text: {}", payment.amount_str()?);
    println!(
        "amount number: {}",
        payment.get_field_as::<u32>(PaymentField::Amount)?
    );

    let mut bytes = Vec::new();
    let mut writer = Writer::new(&mut bytes);
    writer.write_record(&payment)?;
    drop(writer);

    let mut reader = Reader::<_, Payment>::new(BufReader::new(Cursor::new(bytes)));
    let read_back = reader.next().unwrap()?;
    println!("read back account: {}", read_back.account_no_str()?);

    let mut list = PaymentList::new();
    list.push(payment);
    list.push(
        Payment::builder()
            .with_bank_code("0002")
            .with_account_no("7654321")
            .with_amount_int(900)
            .build(),
    );

    let found = list.find_by(PaymentField::BankCode, b"0001");
    println!("found by bank_code=0001: {}", found.len());

    Ok(())
}
