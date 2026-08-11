use fixed_record::prelude::*;

#[fixed_record]
pub struct User {
    pub id: Fixed<4>,
}

/// Verifies that an immutable list cannot logically remove records.
/// immutable な list では論理削除できないことを確認します。
fn main() {
    let list = UserList::new();

    list.remove(0);
}
