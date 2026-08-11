use fixed_record::prelude::*;

#[fixed_record]
pub struct User {
    pub id: Fixed<4>,
}

/// Verifies that an immutable list cannot update records.
/// immutable な list ではレコード更新できないことを確認します。
fn main() {
    let list = UserList::new();
    let user = User::builder().with_id("0001").build();

    list.update(0, user);
}
