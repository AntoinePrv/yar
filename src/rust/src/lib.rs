pub mod array;
pub mod doc;
pub mod event;
pub mod map;
pub mod message;
pub mod state;
pub mod text;
pub mod transaction;
pub mod type_conversion;
pub mod update;
pub(crate) mod utils;

pub use crate::type_conversion::{FromExtendr, IntoExtendr};
pub use crate::{
    array::ArrayRef, doc::Doc, map::MapRef, state::DeleteSet, state::Snapshot, state::StateVector,
    text::TextRef, transaction::Origin, transaction::Transaction, update::Update,
};
pub(crate) use transaction::try_read;
pub(crate) use transaction::ExtendrTransaction;

// Register function with R.
// See corresponding C code in `entrypoint.c`.
extendr_api::extendr_module! {
    mod yr;
    use transaction;
    use array;
    use map;
    use message;
    use state;
    use text;
    use update;
    use doc;
}
