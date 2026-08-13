use fixed_record::prelude::*;

#[fixed_record]
struct User {
    /// Unique user ID, stored as 8 bytes.
    /// ユーザーの固有 ID を8バイトで保持します。
    id: Fixed<8>,

    /// User name, stored as 16 bytes.
    /// ユーザー名を16バイトで保持します。
    name: Fixed<16>,

    /// User age, stored as 3 bytes.
    /// 年齢を3バイトで保持します。
    age: Fixed<3>,
}

/// Runs the main fixed-record APIs as a compact example application.
/// サンプルアプリとして固定長レコードの主要 API を順番に実行します。
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Basic parsing.
    // 基本的なパース機能です。
    let raw_data = "00000001Tanaka Tarou    025";
    println!("--- 1. Basic Access ---");
    let user_ref = User::parse_str(raw_data)?;
    println!(
        "ID: {}, Name: {}, Age: {}\n",
        user_ref.id_str()?,
        user_ref.name_str()?,
        user_ref.age_str()?
    );

    // Metadata access.
    // メタ情報へのアクセスです。
    println!("--- 2. Meta Information Test ---");
    println!("Total Length: {}", User::TOTAL_LEN);
    println!("Field 'Name' size: {}\n", User::size_of(UserField::Name));

    // Builder and initialization APIs.
    // ビルダーパターンと初期化 API です。
    println!("--- 3. Builder & Initialization Test ---");

    // `builder()` uses `cleared()` internally, then `with_*` methods write fields.
    // `builder()` は内部で `cleared()` を使い、`with_*` メソッドでフィールドを書き込みます。
    let new_user = User::builder()
        .with_id("999")
        .with_name("Alice")
        .with_age_int(20)
        .build();

    println!("Created by Builder:");
    let new_user_bytes = new_user.to_bytes();
    println!("  Raw bytes: \"{}\"", std::str::from_utf8(&new_user_bytes)?);
    println!("  ID field   : \"{}\"", new_user.id_str()?);
    println!("  Age field  : \"{}\"\n", new_user.age_str()?);

    // Direct comparison of `zeroed()`, `spaced()`, and `cleared()`.
    // `zeroed()` / `spaced()` / `cleared()` の直接比較です。
    let z = User::zeroed();
    let s = User::spaced();
    let c = User::cleared();
    println!("Initialization Comparison:");
    println!("  zeroed bytes: {:?}", &z.to_bytes()[..5]); // [0, 0, 0, 0, 0]
    println!("  spaced bytes: {:?}", &s.to_bytes()[..5]); // [32, 32, 32, 32, 32] (0x20)
    println!("  cleared bytes: {:?}", &c.to_bytes()[..5]); // default CLEAR_BYTE is space

    // Serialization with `to_bytes`.
    // `to_bytes` によるシリアライズです。
    println!("\n--- 4. Serialization Test ---");
    let byte_array: [u8; 27] = new_user.to_bytes();
    println!("Copied byte array length: {}", byte_array.len());
    assert_eq!(byte_array.len(), User::TOTAL_LEN);

    // Default value behavior.
    // デフォルト値の挙動です。
    let default_user = User::default();
    println!("\n--- 5. Default Trait Test ---");
    println!(
        "Default user (cleared) valid: {}",
        default_user.id().iter().all(|&b| b == b' ')
    );

    // Dynamic field access.
    // フィールド操作（動的アクセス）です。
    println!("\n--- 6. Dynamic Field Access Test ---");
    let mut user = User::parse_str("00000001Tanaka Tarou    025")?;

    // Parse a field as a number through `get_field_as<T>`.
    // `get_field_as<T>` でフィールドを数値として取得します。
    let age_num: u32 = user.get_field_as(UserField::Age)?;
    println!("Parsed Age as u32: {}", age_num);

    // Trim padding spaces through `get_field_trimmed`.
    // `get_field_trimmed` で余計なスペースを排除します。
    let name_trimmed = user.get_field_trimmed(UserField::Name)?;
    println!("Trimmed Name: \"{}\"", name_trimmed);

    // Update a field dynamically through `set_field_str`.
    // `set_field_str` による動的更新です。
    user.set_field_str(UserField::Name, "Sato Jiro");
    println!("After set_field_str: \"{}\"", user.name_str()?);

    // Fill all fields with spaces.
    // 全フィールドをスペースで埋めます。
    user.fill_space();
    println!(
        "After fill_space, Name is empty: {}",
        user.name().iter().all(|&b| b == b' ')
    );

    // Bulk apply helpers.
    // 一括適用・流し込み系の API です。
    println!("\n--- 7. Apply & Bulk Injection Test ---");

    // Apply string data from the first field.
    // 先頭フィールドから文字列データを一気に流し込みます。
    // ID(8) + Name(16) + Age(3) = 27 bytes.
    // ID(8) + Name(16) + Age(3) = 27 バイトです。
    let bulk_data = "99999999Yamada Hanako   080";
    let bulk_user = User::spaced().apply_str(bulk_data);

    println!("Bulk Applied Result:");
    println!(
        "  ID: {}, Name: {}, Age: {}",
        bulk_user.id_str()?,
        bulk_user.name_str()?,
        bulk_user.age_str()?
    );

    // Overwrite from a specific field with `apply_bytes_from`.
    // `apply_bytes_from` で特定のフィールド以降を上書きします。
    // Apply 19 bytes from the Name field at offset 8.
    // offset 8 の Name フィールドから 19 バイト分を流し込みます。
    let partial_data = "Suzuki Ichiro   051";
    let final_user = bulk_user.apply_bytes_from(UserField::Name, partial_data.as_bytes());

    println!("\nPartial Applied Result (From Name):");
    println!("  ID: {} (Unchanged)", final_user.id_str()?);
    println!("  Name: {}", final_user.name_str()?);
    println!("  Age:  {}", final_user.age_str()?);

    // Customize the record through an `apply` closure.
    // `apply` クロージャによるカスタム加工です。
    let custom_user = final_user.apply(|u| {
        u.set_field_str(UserField::Id, "NEW-ID");
        // More complex conditional logic can be written here.
        // 条件に応じた複雑な処理もここに書けます。
        if let Ok(age) = u.get_field_as::<i32>(UserField::Age) {
            let next_age = age + 1;
            // Format the number and set it directly.
            // 数値をフォーマットして直接 set します。
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
