use fixed_record::prelude::*;

mod records {
    use super::*;

    #[fixed_record]
    struct User {
        id: Fixed<4>,
    }
}

fn main() {
    let _field = records::UserField::Id;
    let _list = records::UserList::new();
}
