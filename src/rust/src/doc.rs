use extendr_api::prelude::*;
use yrs::TransactionCleanupEvent as YTransactionCleanupEvent;

use crate::utils::{self, lifetime, ExtendrRef};
use crate::{event, ArrayRef, DeleteSet, IntoExtendr, MapRef, StateVector, TextRef};

utils::extendr_struct!(#[extendr] pub Doc(yrs::Doc));

#[extendr]
impl Doc {
    fn new() -> Self {
        Self(yrs::Doc::new())
    }

    fn client_id(&self) -> u64 {
        self.0.client_id()
    }

    fn guid(&self) -> Strings {
        (*self.0.guid()).into()
    }

    fn get_or_insert_text(&self, name: &str) -> TextRef {
        self.0.get_or_insert_text(name).into()
    }

    fn get_or_insert_map(&self, name: &str) -> MapRef {
        self.0.get_or_insert_map(name).into()
    }

    fn get_or_insert_array(&self, name: &str) -> ArrayRef {
        self.0.get_or_insert_array(name).into()
    }

    // Method must be explicitly visible by extendr to be bound to R.
    #[allow(clippy::inherent_to_string)]
    fn to_string(&self) -> String {
        self.0.to_string()
    }

    fn __yr_print__(&self) -> String {
        self.0.to_string()
    }

    pub fn observe_transaction_cleanup_ec(&self, f: Function, key: &Robj) -> Result<(), Error> {
        let result = event::observe_with!(
            self.as_ref(),
            observe_transaction_cleanup_with,
            TransactionCleanupEvent,
            f,
            key
        );
        result.extendr()
    }

    pub fn unobserve_transaction_cleanup_ec(&self, key: &Robj) -> Result<bool, Error> {
        let result = event::unobserve_with!(self.as_ref(), unobserve_transaction_cleanup, key);
        result.extendr()
    }
}

utils::extendr_struct!(#[extendr] pub TransactionCleanupEvent(lifetime::CheckedRef<YTransactionCleanupEvent>));

#[extendr]
impl TransactionCleanupEvent {
    pub fn before_state_ec(&self) -> Result<StateVector, Error> {
        self.try_map(|event| event.before_state.clone().into())
    }

    pub fn after_state_ec(&self) -> Result<StateVector, Error> {
        self.try_map(|event| event.after_state.clone().into())
    }

    pub fn delete_set_ec(&self) -> Result<DeleteSet, Error> {
        self.try_map(|event| event.delete_set.clone().into())
    }
}

extendr_module! {
    mod doc;
    impl Doc;
    impl TransactionCleanupEvent;
}
