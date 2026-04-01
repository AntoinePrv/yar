use extendr_api::prelude::*;
use yrs::types::array::ArrayEvent as YArrayEvent;
use yrs::{Array as YArray, ArrayPrelim, MapPrelim as YMapPrelim, TextPrelim as YTextPrelim};

use crate::event::ExtendrObservable;
use crate::type_conversion::{FromExtendr, IntoExtendr};
use crate::utils::{self, lifetime, ExtendrRef};
use crate::{try_read, MapRef, TextRef, Transaction};

utils::extendr_struct!(#[extendr] pub ArrayRef(yrs::ArrayRef));

#[extendr]
impl ArrayRef {
    pub fn len(&self, transaction: &Transaction) -> Result<u32, Error> {
        try_read!(transaction, t => self.0.len(t))
    }

    pub fn insert_any(
        &self,
        transaction: &mut Transaction,
        index: u32,
        obj: Robj,
    ) -> Result<(), Error> {
        let trans = transaction.try_write_mut()?;
        let any = yrs::Any::from_extendr(obj)?;
        self.0.insert(trans, index, any);
        Ok(())
    }

    pub fn insert_text(&self, transaction: &mut Transaction, index: u32) -> Result<TextRef, Error> {
        transaction
            .try_write_mut()
            .map(|trans| TextRef::from(self.0.insert(trans, index, YTextPrelim::default())))
    }

    pub fn insert_array(
        &self,
        transaction: &mut Transaction,
        index: u32,
    ) -> Result<ArrayRef, Error> {
        transaction
            .try_write_mut()
            .map(|trans| ArrayRef::from(self.0.insert(trans, index, ArrayPrelim::default())))
    }

    pub fn insert_map(&self, transaction: &mut Transaction, index: u32) -> Result<MapRef, Error> {
        transaction
            .try_write_mut()
            .map(|trans| MapRef::from(self.0.insert(trans, index, YMapPrelim::default())))
    }

    pub fn get(&self, transaction: &mut Transaction, index: u32) -> Result<Robj, Error> {
        try_read!(transaction, t => self.0.get(t, index).as_ref().extendr()).and_then(|r| r)
    }

    pub fn remove(&self, transaction: &mut Transaction, index: u32) -> Result<(), Error> {
        transaction.try_write_mut().map(|trans| {
            self.0.remove(trans, index);
        })
    }

    pub fn observe(&self, f: Function, key: &Robj) -> Result<(), Error> {
        ExtendrObservable::<ArrayEvent>::observe(self, f, key)
    }

    pub fn unobserve(&self, key: &Robj) -> Result<(), Error> {
        ExtendrObservable::<ArrayEvent>::unobserve(self, key)
    }
}

utils::extendr_struct!(#[extendr] pub ArrayEvent(lifetime::CheckedRef<YArrayEvent>));

#[extendr]
impl ArrayEvent {
    pub fn target(&self) -> Result<ArrayRef, Error> {
        // Cloning is shallow BranchPtr copy pointing to same data.
        self.try_map(|event| event.target().clone().into())
    }

    pub fn delta(&self, transaction: &Transaction) -> Result<Robj, Error> {
        self.try_map(|event| {
            transaction
                .try_write()
                .map(|trans| event.delta(trans).extendr())
        })
        .and_then(|r| r) // TODO(MSRV 1.89) .flatten()
        .and_then(|r| r) // TODO(MSRV 1.89) .flatten()
    }

    pub fn path(&self) -> Result<Robj, Error> {
        self.try_map(|event| event.path().extendr()).and_then(|r| r) // TODO(MSRV 1.89) .flatten()
    }

    pub fn inserts(&self, transaction: &Transaction) -> Result<Robj, Error> {
        self.try_map(|event| {
            transaction
                .try_write()
                .map(|trans| event.inserts(trans).extendr())
        })
        .and_then(|r| r) // TODO(MSRV 1.89) .flatten()
        .and_then(|r| r) // TODO(MSRV 1.89) .flatten()
    }

    pub fn removes(&self, transaction: &Transaction) -> Result<Robj, Error> {
        self.try_map(|event| {
            transaction
                .try_write()
                .map(|trans| event.removes(trans).extendr())
        })
        .and_then(|r| r) // TODO(MSRV 1.89) .flatten()
        .and_then(|r| r) // TODO(MSRV 1.89) .flatten()
    }
}

extendr_module! {
    mod array;
    impl ArrayRef;
    impl ArrayEvent;
}
