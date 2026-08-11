use fixed_record_main::prelude::*;

#[fixed_record_main]
pub struct User {
    pub id: Fixed<4>,
}

/// immutable な list では論理削除できないことを確認します。
fn main() {
    let list = UserList::new();

    list.remove(0);
}
