mod model {
    use fixed_record::prelude::*;

    #[fixed_record]
    pub struct User {
        pub id: Fixed<4>,
    }
}

/// Verifies that search result indexes remain private implementation details.
/// 検索結果の index が非公開の実装詳細であることを確認します。
fn main() {
    let list = model::UserList::new();

    let _ = list.try_find_ids_by(model::UserField::Id, b"0001");
}
