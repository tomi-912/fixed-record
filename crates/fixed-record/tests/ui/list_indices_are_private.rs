use fixed_record::prelude::*;

mod records {
    use super::*;

    #[fixed_record]
    pub struct User {
        id: Fixed<4>,
    }
}

fn main() {
    let _list = records::UserList::new();
    let _indices = records::UserListIndices::default();
}
