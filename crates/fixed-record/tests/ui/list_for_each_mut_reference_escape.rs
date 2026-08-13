use fixed_record::prelude::*;

#[fixed_record]
pub struct User {
    pub id: Fixed<4>,
}

/// Verifies that `for_each_mut` cannot leak a mutable record reference.
/// `for_each_mut` からレコードの mutable 参照を外へ保持できないことを確認します。
fn main() {
    let mut list = UserList::new();
    list.push(User::builder().with_id("0001").build());

    let mut retained = None;
    list.for_each_mut(|record| retained = Some(record));
    let _ = retained;
}
