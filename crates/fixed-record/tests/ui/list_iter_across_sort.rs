use fixed_record::prelude::*;

#[fixed_record]
pub struct User {
    pub id: Fixed<4>,
}

/// Verifies that an iterator cannot remain in use across a mutable sort.
/// iterator を保持したまま mutable な sort を実行できないことを確認します。
fn main() {
    let mut list = UserList::new();
    list.push(User::builder().with_id("0001").build());

    let mut records = list.iter();
    list.sort();
    let _ = records.next();
}
