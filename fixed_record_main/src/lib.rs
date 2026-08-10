pub mod error;
pub mod io;
pub mod traits;
pub mod types;

pub use fixed_record_macros::fixed_record_main;

pub use error::Error;
pub use io::{Reader, Writer};
pub use traits::FixedRecord;
pub use types::Fixed;

pub mod prelude {
    pub use crate::fixed_record_main;
    pub use crate::{Error, Fixed, FixedRecord, Reader, Writer};
}
