use fixed_record_main::prelude::*;

#[fixed_record_main]
pub struct Header {
    pub code: Fixed<4>,
}

fn main() {
    let _ = HeaderList::new();
}
