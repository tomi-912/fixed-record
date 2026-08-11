use fixed_record::prelude::*;

#[fixed_record]
pub struct Header {
    pub code: Fixed<4>,
}

fn main() {
    let _ = HeaderList::new();
}
