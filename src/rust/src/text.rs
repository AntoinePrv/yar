use extendr_api::prelude::*;
use yrs::types::{text::TextEvent as YTextEvent, PathSegment as YPathSegment};
use yrs::{GetString as YGetString, Observable as YObservable, Text as YText};

use crate::type_conversion::IntoExtendr;
use crate::utils::{self, lifetime, ExtendrRef};
use crate::{try_read, Origin, Transaction};

utils::extendr_struct!(#[extendr] pub TextRef(yrs::TextRef));

#[extendr]
impl TextRef {
    pub fn len(&self, transaction: &Transaction) -> Result<u32, Error> {
        try_read!(transaction, t => self.0.len(t))
    }

    pub fn insert(
        &self,
        transaction: &mut Transaction,
        index: u32,
        chunk: &str,
    ) -> Result<(), Error> {
        transaction
            .try_write_mut()
            .map(|trans| self.0.insert(trans, index, chunk))
    }

    pub fn push(&self, transaction: &mut Transaction, chunk: &str) -> Result<(), Error> {
        transaction
            .try_write_mut()
            .map(|trans| self.0.push(trans, chunk))
    }

    pub fn remove_range(
        &self,
        transaction: &mut Transaction,
        index: u32,
        len: u32,
    ) -> Result<(), Error> {
        transaction
            .try_write_mut()
            .map(|trans| self.0.remove_range(trans, index, len))
    }

    pub fn get_string(&self, transaction: &Transaction) -> Result<String, Error> {
        try_read!(transaction, t => self.0.get_string(t))
    }

    pub fn observe(&self, f: Function, key: &Robj) -> Result<(), Error> {
        if f.formals().map(|g| g.len()).unwrap_or(0) != 2 {
            return Err(Error::Other(
                "Callback expect exactly two parameters: transaction and event".into(),
            ));
        }
        self.0.observe_with(
            Origin::new(key)?,
            move |trans: &yrs::TransactionMut<'_>, event: &YTextEvent| {
                // Converting to Robj first as the converter will set the class symbol attribute,
                // otherwise it will only be seen as an `externalptr` from R.
                let event = TextEvent::guard(event);
                let mut trans: Robj = Transaction::from_ref(trans).into();
                let result = f.call(pairlist!(trans.clone(), event.get().clone()));
                TryInto::<&mut Transaction>::try_into(&mut trans)
                    .unwrap()
                    .unlock();
                // TODO Either take an on_error, or store it somewhere
                result.unwrap();
            },
        );
        Ok(())
    }

    pub fn unobserve(&self, key: &Robj) -> Result<(), Error> {
        self.0.unobserve(Origin::new(key)?);
        Ok(())
    }
}

utils::extendr_struct!(#[extendr] pub TextEvent(lifetime::CheckedRef<YTextEvent>));

#[extendr]
impl TextEvent {
    fn target(&self) -> Result<TextRef, Error> {
        // Cloning is shallow BranchPtr copy pinting to same data.
        self.try_map(|event| event.target().clone().into())
    }

    fn delta(&self, transaction: &Transaction) -> Result<Robj, Error> {
        self.try_map(|event| {
            transaction
                .try_write()
                .map(|trans| event.delta(trans).extendr())
        })
        .and_then(|r| r) // TODO(MSRV 1.89) .flatten()
        .and_then(|r| r) // TODO(MSRV 1.89) .flatten()
    }

    fn path(&self) -> Result<List, Error> {
        self.try_map(|event| {
            event
                .path()
                .into_iter()
                .map(|seg| match seg {
                    YPathSegment::Key(k) => Strings::from_values([k]).into_robj(),
                    YPathSegment::Index(i) => IntoRobj::into_robj(i),
                })
                .collect()
        })
    }
}

extendr_module! {
    mod text;
    impl TextRef;
    impl TextEvent;
}
