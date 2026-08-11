use fixed_record::prelude::*;

#[fixed_record]
pub struct User {
    /// ユーザーの固有ID（8桁）
    pub id: Fixed<8>,

    /// 名前
    pub name: Fixed<16>,

    /// 年齢
    pub age: Fixed<3>,
}

/// サンプルアプリとして固定長レコードの主要 API を順番に実行します。
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

    // builder() (内部で cleared() を使用) と with_... メソッドのテスト
    let new_user = User::builder()
        .with_id("999") // 文字列を先頭から上書き
        .with_name("Alice") // 文字列を先頭から上書き
        .with_age_int(20) // 数値をセット（左ゼロ埋め: "020"）
        .build();

    println!("Created by Builder:");
    let new_user_bytes = new_user.to_bytes();
    println!("  Raw bytes: \"{}\"", std::str::from_utf8(&new_user_bytes)?);
    println!("  ID field   : \"{}\"", new_user.id_str()?);
    println!("  Age field  : \"{}\"\n", new_user.age_str()?);

    // zeroed() / spaced() / cleared() の直接比較
    let z = User::zeroed();
    let s = User::spaced();
    let c = User::cleared();
    println!("Initialization Comparison:");
    println!("  zeroed bytes: {:?}", &z.to_bytes()[..5]); // [0, 0, 0, 0, 0]
    println!("  spaced bytes: {:?}", &s.to_bytes()[..5]); // [32, 32, 32, 32, 32] (0x20)
    println!("  cleared bytes: {:?}", &c.to_bytes()[..5]); // default CLEAR_BYTE is 0x00

    // --- 4. シリアライズ (to_bytes) ---
    println!("\n--- 4. Serialization Test ---");
    let byte_array: [u8; 27] = new_user.to_bytes();
    println!("Copied byte array length: {}", byte_array.len());
    assert_eq!(byte_array.len(), User::TOTAL_LEN);

    // --- 5. デフォルト値のテスト ---
    let default_user = User::default();
    println!("\n--- 5. Default Trait Test ---");
    println!(
        "Default user (cleared) valid: {}",
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
