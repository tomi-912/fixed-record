use fixed_record_main::prelude::*;

#[fixed_record_main]
pub struct User {
    /// ユーザーの固有ID（8桁）
    pub id: Fixed<8>,

    /// 名前
    pub name: Fixed<16>,

    /// 年齢
    pub age: Fixed<3>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // テスト用の構造体を定義
    #[fixed_record_main]
    pub struct TestRecord {
        /// 名前フィールド
        name: Fixed<10>,
        /// コードフィールド
        code: Fixed<5>,
        /// 金額フィールド
        amount: Fixed<8>,
    }

    // ---- apply<F> のテスト ----

    #[test]
    fn test_apply_closure() {
        let aa = TestRecord::builder().apply_str("test      ttte");

        let rec = TestRecord::spaced().apply(|r| {
            r.set_field_str(TestRecordField::Name, "Alice");
            r.set_field_str(TestRecordField::Code, "A001");
        });
        assert_eq!(aa.get_field_trimmed(TestRecordField::Name).unwrap(), "test");

        assert_eq!(
            rec.get_field_trimmed(TestRecordField::Name).unwrap(),
            "Alice"
        );
        assert_eq!(
            rec.get_field_trimmed(TestRecordField::Code).unwrap(),
            "A001"
        );
    }

    // ---- apply_bytes のテスト ----

    #[test]
    fn test_apply_bytes_exact() {
        // ちょうど TOTAL_LEN (23) バイト
        let data = b"HelloWorldABCDE12345678";
        let rec = TestRecord::spaced().apply_bytes(data);

        assert_eq!(rec.name(), b"HelloWorld");
        assert_eq!(rec.code(), b"ABCDE");
        assert_eq!(rec.amount(), b"12345678");
    }

    #[test]
    fn test_apply_bytes_shorter_than_total() {
        // データが途中で終わる場合 → 残りのフィールドはspaced のまま
        let data = b"HelloWorldABC"; // name(10) + code の途中(3)
        let rec = TestRecord::spaced().apply_bytes(data);

        assert_eq!(rec.name(), b"HelloWorld");
        assert_eq!(&rec.code()[..3], b"ABC");
        assert_eq!(&rec.code()[3..], b"  "); // 残りはspaced
        assert_eq!(rec.amount(), b"        "); // 未到達はspaced のまま
    }

    // ---- apply_str のテスト ----

    #[test]
    fn test_apply_str() {
        let s = b"HelloWorldABCDE12345678";
        let rec = TestRecord::spaced().apply_bytes(s);

        assert_eq!(
            rec.get_field_trimmed(TestRecordField::Name).unwrap(),
            "HelloWorld"
        );
        assert_eq!(
            rec.get_field_trimmed(TestRecordField::Code).unwrap(),
            "ABCDE"
        );
        assert_eq!(
            rec.get_field_trimmed(TestRecordField::Amount).unwrap(),
            "12345678"
        );
    }

    // ---- apply_bytes_from のテスト ----

    #[test]
    fn test_apply_bytes_from_first_field() {
        // 先頭から開始 → apply_bytes と同じ動作
        let data = b"HelloWorldABCDE12345678";
        let rec = TestRecord::spaced().apply_bytes_from(TestRecordField::Name, data);

        assert_eq!(rec.name(), b"HelloWorld");
        assert_eq!(rec.code(), b"ABCDE");
        assert_eq!(rec.amount(), b"12345678");
    }

    #[test]
    fn test_apply_bytes_from_middle_field() {
        // code フィールドから開始 → name はspaced のまま
        let data = b"ABCDE12345678";
        let rec = TestRecord::spaced().apply_bytes_from(TestRecordField::Code, data);

        assert_eq!(rec.name(), b"          "); // 未変更
        assert_eq!(rec.code(), b"ABCDE");
        assert_eq!(rec.amount(), b"12345678");
    }

    #[test]
    fn test_apply_bytes_from_last_field() {
        // amount フィールドのみ書き込み
        let data = b"99999999";
        let rec = TestRecord::spaced().apply_bytes_from(TestRecordField::Amount, data);

        assert_eq!(rec.name(), b"          "); // 未変更
        assert_eq!(rec.code(), b"     "); // 未変更
        assert_eq!(rec.amount(), b"99999999");
    }

    // ---- apply_str_from のテスト ----

    #[test]
    fn test_apply_str_from_middle() {
        let s = "ABCDE12345678";
        let rec = TestRecord::spaced().apply_str_from(TestRecordField::Code, s);

        assert_eq!(rec.get_field_trimmed(TestRecordField::Name).unwrap(), "");
        assert_eq!(
            rec.get_field_trimmed(TestRecordField::Code).unwrap(),
            "ABCDE"
        );
        assert_eq!(
            rec.get_field_trimmed(TestRecordField::Amount).unwrap(),
            "12345678"
        );
    }

    // ---- メソッドチェーンの組み合わせテスト ----

    #[test]
    fn test_method_chain() {
        let rec = TestRecord::builder()
            .with_name("Alice")
            .with_code("A001")
            .with_amount_int(12345)
            .build();

        assert_eq!(
            rec.get_field_trimmed(TestRecordField::Name).unwrap(),
            "Alice"
        );
        assert_eq!(
            rec.get_field_trimmed(TestRecordField::Code).unwrap(),
            "A001"
        );
        assert_eq!(
            rec.get_field_trimmed(TestRecordField::Amount).unwrap(),
            "00012345"
        );
    }

    #[test]
    fn test_reader_writer_and_fixed_record_trait() {
        use fixed_record_main::{FixedRecord, Reader, Writer};
        use std::io::{BufReader, Cursor};

        fn bytes_via_trait<T: FixedRecord>(record: &T) -> Vec<u8> {
            record.to_bytes()
        }

        let first = TestRecord::builder()
            .with_name("Alice")
            .with_code("A0001")
            .with_amount_int(10)
            .build();
        let second = TestRecord::builder()
            .with_name("Bob")
            .with_code("B0001")
            .with_amount_int(20)
            .build();

        assert_eq!(bytes_via_trait(&first).len(), TestRecord::TOTAL_LEN);

        let mut buf = Vec::new();
        let mut writer = Writer::new(&mut buf).with_newline(b"\r\n");
        writer.write_record(&first).unwrap();
        writer.write_record(&second).unwrap();
        writer.flush().unwrap();

        let mut reader = Reader::<_, TestRecord>::new(BufReader::new(Cursor::new(buf)));
        assert_eq!(
            reader
                .next()
                .unwrap()
                .unwrap()
                .get_field_trimmed(TestRecordField::Name)
                .unwrap(),
            "Alice"
        );
        assert_eq!(
            reader
                .next()
                .unwrap()
                .unwrap()
                .get_field_trimmed(TestRecordField::Name)
                .unwrap(),
            "Bob"
        );
        assert!(reader.next().is_none());
    }

    #[test]
    fn test_generated_list_management() {
        let mut list = TestRecordList::new();

        let id_b = list.insert(
            TestRecord::builder()
                .with_name("Bob")
                .with_code("B0001")
                .with_amount_int(20)
                .build(),
        );
        let id_a = list.insert(
            TestRecord::builder()
                .with_name("Alice")
                .with_code("A0001")
                .with_amount_int(10)
                .build(),
        );

        assert_eq!(list.len(), 2);

        let found = list.find_by(TestRecordField::Code, *b"A0001");
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].get_field_trimmed(TestRecordField::Name).unwrap(),
            "Alice"
        );

        list.sort_by(&[TestRecordField::Name]);
        let names: Vec<_> = list
            .iter()
            .map(|record| {
                record
                    .get_field_string_trimmed(TestRecordField::Name)
                    .unwrap()
            })
            .collect();
        assert_eq!(names, vec!["Alice".to_string(), "Bob".to_string()]);

        let first_by_code = list.first_by::<5>(TestRecordField::Code).unwrap();
        assert_eq!(
            first_by_code
                .get_field_trimmed(TestRecordField::Code)
                .unwrap(),
            "A0001"
        );

        assert!(list.remove(id_b));
        assert_eq!(list.len(), 1);
        assert_eq!(list.all_ids().len(), 2);
        list.vacuum();
        assert_eq!(list.all_ids(), vec![id_a]);
    }

    // ---- apply の冪等性テスト ----

    #[test]
    fn test_apply_idempotent() {
        // 同じデータを2回 apply しても結果は同じ
        let data = b"HelloWorldABCDE12345678";
        let rec1 = TestRecord::spaced().apply_bytes(data);
        let rec2 = TestRecord::spaced().apply_bytes(data).apply_bytes(data);

        assert_eq!(rec1.to_bytes(), rec2.to_bytes());
    }

    #[cfg(feature = "unchecked")]
    #[test]
    fn test_unchecked_feature_methods_are_available() {
        let data = b"HelloWorldABCDE12345678";

        let rec = unsafe { TestRecord::parse_unchecked(data).unwrap() };
        assert_eq!(rec.to_bytes(), *data);

        let rec_ref = unsafe { TestRecord::from_bytes_unchecked(data).unwrap() };
        assert_eq!(rec_ref.name(), b"HelloWorld");

        let raw = unsafe { rec.as_bytes_unchecked() };
        assert_eq!(raw, data);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- 1. 既存のパース機能 ---
    let raw_data = "00000001Tanaka Tarou    025";
    println!("--- 1. Basic Access ---");
    let user_ref = User::parse_str(raw_data)?;
    println!(
        "ID: {}, Name: {}, Age: {}\n",
        user_ref.id_str()?,
        user_ref.name_str()?,
        user_ref.age_str()?
    );

    // --- 2. メタ情報のテスト (前回分) ---
    println!("--- 2. Meta Information Test ---");
    println!("Total Length: {}", User::TOTAL_LEN);
    println!("Field 'Name' size: {}\n", User::size_of(UserField::Name));

    // --- 3. 新機能: ビルダーパターンと初期化 ---
    println!("--- 3. Builder & Initialization Test ---");

    // builder() (内部で spaced() を使用) と with_... メソッドのテスト
    let new_user = User::builder()
        .with_id("999") // 文字列をセット（右スペース埋め）
        .with_name("Alice") // 文字列をセット（右スペース埋め）
        .with_age_int(20) // 数値をセット（左ゼロ埋め: "020"）
        .build();

    println!("Created by Builder:");
    let new_user_bytes = new_user.to_bytes();
    println!("  Raw bytes: \"{}\"", std::str::from_utf8(&new_user_bytes)?);
    println!("  ID field   : \"{}\"", new_user.id_str()?);
    println!("  Age field  : \"{}\"\n", new_user.age_str()?);

    // zeroed() と spaced() の直接比較
    let z = User::zeroed();
    let s = User::spaced();
    println!("Initialization Comparison:");
    println!("  zeroed bytes: {:?}", &z.to_bytes()[..5]); // [0, 0, 0, 0, 0]
    println!("  spaced bytes: {:?}", &s.to_bytes()[..5]); // [32, 32, 32, 32, 32] (0x20)

    // --- 4. シリアライズ (to_bytes) ---
    println!("\n--- 4. Serialization Test ---");
    let byte_array: [u8; 27] = new_user.to_bytes();
    println!("Copied byte array length: {}", byte_array.len());
    assert_eq!(byte_array.len(), User::TOTAL_LEN);

    // --- 5. デフォルト値のテスト ---
    let default_user = User::default();
    println!("\n--- 5. Default Trait Test ---");
    println!(
        "Default user (zeroed) valid: {}",
        default_user.id().iter().all(|&b| b == 0)
    );

    // --- 6. フィールド操作（動的アクセス）のテスト ---
    println!("\n--- 6. Dynamic Field Access Test ---");
    let mut user = User::parse_str("00000001Tanaka Tarou    025")?;

    // get_field_as<T> で数値として取得
    let age_num: u32 = user.get_field_as(UserField::Age)?;
    println!("Parsed Age as u32: {}", age_num);

    // get_field_trimmed で余計なスペースを排除
    let name_trimmed = user.get_field_trimmed(UserField::Name)?;
    println!("Trimmed Name: \"{}\"", name_trimmed);

    // set_field_str による動的更新
    user.set_field_str(UserField::Name, "Sato Jiro");
    println!("After set_field_str: \"{}\"", user.name_str()?);

    // fill_space のテスト
    user.fill_space();
    println!(
        "After fill_space, Name is empty: {}",
        user.name().iter().all(|&b| b == b' ')
    );

    // --- 7. 一括適用・流し込み (Apply系) のテスト ---
    println!("\n--- 7. Apply & Bulk Injection Test ---");

    // apply_str で先頭から一気に流し込む
    // ID(8) + Name(16) + Age(3) = 27bytes
    let bulk_data = "99999999Yamada Hanako   080";
    let bulk_user = User::spaced().apply_str(bulk_data);

    println!("Bulk Applied Result:");
    println!(
        "  ID: {}, Name: {}, Age: {}",
        bulk_user.id_str()?,
        bulk_user.name_str()?,
        bulk_user.age_str()?
    );

    // apply_bytes_from で特定のフィールド以降を上書き
    // Nameフィールド(offset 8) から 19バイト分(16+3)を流し込む
    let partial_data = "Suzuki Ichiro   051";
    let final_user = bulk_user.apply_bytes_from(UserField::Name, partial_data.as_bytes());

    println!("\nPartial Applied Result (From Name):");
    println!("  ID: {} (Unchanged)", final_user.id_str()?);
    println!("  Name: {}", final_user.name_str()?);
    println!("  Age:  {}", final_user.age_str()?);

    // apply クロージャによるカスタム加工
    let custom_user = final_user.apply(|u| {
        u.set_field_str(UserField::Id, "NEW-ID");
        // 条件に応じた複雑な処理も可能
        if let Ok(age) = u.get_field_as::<i32>(UserField::Age) {
            let next_age = age + 1;
            // 数値をフォーマットして直接 set する
            let s = format!("{:0>3}", next_age);
            u.set_field_str(UserField::Age, &s);
        }
    });

    println!("\nCustom Apply (ID update & Age +1):");
    println!(
        "  ID: {}, Age: {}",
        custom_user.id_str()?,
        custom_user.age_str()?
    );

    println!("\nAll advanced systems green! 🚀");
    Ok(())
}
