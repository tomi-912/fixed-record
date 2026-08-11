use fixed_record_main::prelude::*;

#[fixed_record_main]
pub struct User {
    pub id: Fixed<4>,
}

/// immutable な list ではレコード更新できないことを確認します。
fn main() {
    let list = UserList::new();
    let user = User::builder().with_id("0001").build();

    list.update(0, user);
}
