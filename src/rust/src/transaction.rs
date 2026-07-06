use extendr_api::prelude::*;
use stable_deref_trait::{CloneStableDeref, StableDeref};
use yoke::{Yoke, Yokeable};
use yrs::updates::decoder::Decode as YDecode;
use yrs::{ReadTxn as YReadTxn, Transact as YTransact};

use crate::type_conversion::IntoExtendr;
use crate::utils;
use crate::{Doc, Snapshot, StateVector};

/// Either a read or write transaction with the appropriate lifetime.
///
/// This is [`Yokeable`] and meant to use in a [`Yoke`] along with the
/// [`Robj`] containing the doc.
#[allow(clippy::large_enum_variant)]
#[derive(Yokeable)]
pub enum OwnedTransaction<'doc> {
    Read(yrs::Transaction<'doc>),
    Write(yrs::TransactionMut<'doc>),
}

/// Wrapper around [`ExternalPtr<Doc>`] to apply marker traits.
///
/// This will serve as the cart in the [`Yoke`].
#[derive(Clone)]
pub struct ExtendrDoc(ExternalPtr<Doc>);

impl std::ops::Deref for ExtendrDoc {
    type Target = Doc;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl From<ExternalPtr<Doc>> for ExtendrDoc {
    fn from(value: ExternalPtr<Doc>) -> Self {
        Self(value)
    }
}

// SAFETY: R object memory allocation is stable
unsafe impl StableDeref for ExtendrDoc {}

// SAFETY: Robj clone increase reference counting
unsafe impl CloneStableDeref for ExtendrDoc {}

/// Either an owned or borrowed transaction.
///
/// We heavily rely on enums to avoid having to to bind multiple different classes for
/// transaction and borrowed transaction.
#[extendr]
#[allow(clippy::large_enum_variant)]
pub enum Transaction {
    /// Owned transaction, carrying the [`Doc`] along as it shares a lock with it.
    /// Since this relies on [`Drop`] in rust to free the lock, but the object is
    /// garbage collected in R, it would potentially be slow / deadlock.
    /// As such we rely on [`Option`] to actively drop the object, after which any
    /// operations on it will raise an error.
    /// The user friendly way to call this in R is through `with_transaction`,
    /// which is a callback-based analogous to Python context manager.
    Owned(Option<Yoke<OwnedTransaction<'static>, ExtendrDoc>>),
    // TODO this is unsound: the lifetime is still accessible from safe code.
    /// A reference to a borrowed transaction.
    /// This happens in callbacks where a reference to a transaction is provided.
    /// In this case, R code may theoretically escape the value from the callback
    /// and extend its lifetime beyon that of the reference.
    /// An [`Option`] is used to actively delete the reference after the callback
    /// is used, raising an error if the user tries to use it outside of it.
    Ref(Option<&'static yrs::TransactionMut<'static>>),
}

/// A reference to a readable transaction, for dispatch in the [`try_read`] macro.
pub(crate) enum DynTransactionRef<'a> {
    Read(&'a yrs::Transaction<'a>),
    Write(&'a yrs::TransactionMut<'a>),
}

macro_rules! try_read {
    ($txn:expr, $t:ident => $body:expr) => {
        $txn.try_dyn().map(|txn| match txn {
            crate::transaction::DynTransactionRef::Read($t) => $body,
            crate::transaction::DynTransactionRef::Write($t) => $body,
        })
    };
}

pub(crate) use try_read;

pub(crate) trait ExtendrTransaction {
    fn try_dyn(&self) -> Result<DynTransactionRef<'_>, Error>;

    fn try_write(&self) -> Result<&yrs::TransactionMut<'_>, Error>;

    fn with_write_mut<R: 'static>(
        &mut self,
        f: impl 'static + FnOnce(&mut yrs::TransactionMut<'_>) -> R,
    ) -> Result<R, Error>;

    fn is_mutable(&self) -> Result<bool, Error> {
        Ok(self.try_write().is_ok())
    }

    fn origin(&self) -> Result<Robj, Error> {
        match self.try_write() {
            Ok(trans) => match trans.origin() {
                Some(o) => Ok(Origin(o.clone()).into()),
                None => Ok(r!(NULL)),
            },
            Err(_) => Ok(r!(NULL)),
        }
    }

    fn commit(&mut self) -> Result<(), Error> {
        self.with_write_mut(|trans| trans.commit())
    }

    fn state_vector(&self) -> Result<StateVector, Error> {
        try_read!(self, t => t.state_vector().into())
    }

    fn encode_diff_v1(&self, state_vector: &StateVector) -> Result<Vec<u8>, Error> {
        try_read!(self, t => t.encode_diff_v1(state_vector.as_ref()))
    }

    fn encode_diff_v2(&self, state_vector: &StateVector) -> Result<Vec<u8>, Error> {
        try_read!(self, t => t.encode_diff_v2(state_vector.as_ref()))
    }

    fn encode_state_as_update_v1(&self, state_vector: &StateVector) -> Result<Vec<u8>, Error> {
        try_read!(self, t => t.encode_state_as_update_v1(state_vector.as_ref()))
    }

    fn encode_state_as_update_v2(&self, state_vector: &StateVector) -> Result<Vec<u8>, Error> {
        try_read!(self, t => t.encode_state_as_update_v2(state_vector.as_ref()))
    }

    fn apply_update_v1(&mut self, data: &[u8]) -> Result<(), Error> {
        let update = yrs::Update::decode_v1(data).extendr()?;
        self.with_write_mut(move |trans| trans.apply_update(update).extendr())?
    }

    fn apply_update_v2(&mut self, data: &[u8]) -> Result<(), Error> {
        let update = yrs::Update::decode_v2(data).extendr()?;
        self.with_write_mut(move |trans| trans.apply_update(update).extendr())?
    }

    fn snapshot(&self) -> Result<Snapshot, Error> {
        try_read!(self, t => t.snapshot().into())
    }
}

impl Transaction {
    pub(crate) fn from_ref(transaction: &yrs::TransactionMut<'_>) -> Self {
        // TODO Safety: None, unlock must be called while the original ref is valid
        let transaction = unsafe {
            std::mem::transmute::<&'_ yrs::TransactionMut<'_>, &'static yrs::TransactionMut<'static>>(
                transaction,
            )
        };
        Transaction::Ref(Some(transaction))
    }
}

impl ExtendrTransaction for Transaction {
    fn try_dyn(&self) -> Result<DynTransactionRef<'_>, Error> {
        match self {
            Transaction::Owned(Some(t)) => match t.get() {
                OwnedTransaction::Read(t) => Ok(DynTransactionRef::Read(t)),
                OwnedTransaction::Write(t) => Ok(DynTransactionRef::Write(t)),
            },
            Transaction::Ref(Some(t)) => Ok(DynTransactionRef::Write(t)),
            Transaction::Owned(None) | Transaction::Ref(None) => {
                Err(Error::Other("Transaction was dropped".into()))
            }
        }
    }

    fn try_write(&self) -> Result<&yrs::TransactionMut<'_>, Error> {
        match self {
            Transaction::Owned(Some(t)) => match t.get() {
                OwnedTransaction::Write(t) => Ok(t),
                OwnedTransaction::Read(_) => Err(Error::Other("Transaction is readonly".into())),
            },
            Transaction::Ref(Some(t)) => Ok(t),
            Transaction::Owned(None) | Transaction::Ref(None) => {
                Err(Error::Other("Transaction was dropped".into()))
            }
        }
    }

    fn with_write_mut<R: 'static>(
        &mut self,
        f: impl 'static + FnOnce(&mut yrs::TransactionMut<'_>) -> R,
    ) -> Result<R, Error> {
        match self {
            Transaction::Owned(Some(t)) => t.with_mut_return(|t| match t {
                OwnedTransaction::Write(t) => Ok(f(t)),
                OwnedTransaction::Read(_) => Err(Error::Other("Transaction is readonly".into())),
            }),
            Transaction::Ref(Some(_)) => Err(Error::Other("Transaction is readonly".into())),
            Transaction::Owned(None) | Transaction::Ref(None) => {
                Err(Error::Other("Transaction was dropped".into()))
            }
        }
    }
}

#[extendr]
impl Transaction {
    pub fn lock(
        doc: ExternalPtr<Doc>,
        #[extendr(default = "FALSE")] mutable: bool,
        #[extendr(default = "NULL")] origin: Nullable<&Origin>,
    ) -> Self {
        // Safety: Doc lives in R memory and is kept alive by the `owner` Robj in
        // OwnedInnerTransaction. R's GC is non-moving, so the pointer inside the transaction
        // remains valid as long as the Doc is not freed. The Robj prevents collection via
        // R_PreserveObject semantics. The transaction field drops before the owner (declaration
        // order), so the Doc is still alive when the transaction releases its lock.
        // let transaction = unsafe {
        //     std::mem::transmute::<OwnedTransaction<'_>, OwnedTransaction<'static>>(transaction)
        // };
        Transaction::Owned(Some(
            Yoke::<OwnedTransaction<'static>, ExtendrDoc>::attach_to_cart(
                doc.clone().into(),
                |data: &Doc| match (mutable, origin) {
                    (true, Nullable::NotNull(o)) => {
                        OwnedTransaction::Write(data.as_ref().transact_mut_with(o.0.clone()))
                    }
                    (true, Nullable::Null) => OwnedTransaction::Write(data.as_ref().transact_mut()),
                    (false, _) => OwnedTransaction::Read(data.as_ref().transact()),
                },
            ),
        ))
    }

    pub fn is_mutable_ec(&self) -> Result<bool, Error> {
        ExtendrTransaction::is_mutable(self)
    }

    pub fn origin_ec(&self) -> Result<Robj, Error> {
        ExtendrTransaction::origin(self)
    }

    pub fn commit_ec(&mut self) -> Result<(), Error> {
        ExtendrTransaction::commit(self)
    }

    pub fn unlock(&mut self) {
        match self {
            Transaction::Owned(opt) => *opt = None,
            Transaction::Ref(opt) => *opt = None,
        }
    }

    pub fn state_vector_ec(&self) -> Result<StateVector, Error> {
        ExtendrTransaction::state_vector(self)
    }

    pub fn encode_diff_v1_ec(&self, state_vector: &StateVector) -> Result<Vec<u8>, Error> {
        ExtendrTransaction::encode_diff_v1(self, state_vector)
    }

    pub fn encode_diff_v2_ec(&self, state_vector: &StateVector) -> Result<Vec<u8>, Error> {
        ExtendrTransaction::encode_diff_v2(self, state_vector)
    }

    pub fn encode_state_as_update_v1_ec(
        &self,
        state_vector: &StateVector,
    ) -> Result<Vec<u8>, Error> {
        ExtendrTransaction::encode_state_as_update_v1(self, state_vector)
    }

    pub fn encode_state_as_update_v2_ec(
        &self,
        state_vector: &StateVector,
    ) -> Result<Vec<u8>, Error> {
        ExtendrTransaction::encode_state_as_update_v2(self, state_vector)
    }

    pub fn apply_update_v1_ec(&mut self, data: &[u8]) -> Result<(), Error> {
        ExtendrTransaction::apply_update_v1(self, data)
    }

    pub fn apply_update_v2_ec(&mut self, data: &[u8]) -> Result<(), Error> {
        ExtendrTransaction::apply_update_v2(self, data)
    }

    pub fn snapshot_ec(&self) -> Result<Snapshot, Error> {
        ExtendrTransaction::snapshot(self)
    }
}

utils::extendr_struct!(#[extendr] pub Origin(yrs::Origin));

#[extendr]
impl Origin {
    pub fn new_ec(data: &Robj) -> Result<Self, Error> {
        if let Ok(origin) = TryInto::<&Origin>::try_into(data) {
            Ok(Self(origin.0.clone()))
        } else if let Ok(n) = TryInto::<i64>::try_into(data) {
            Ok(Self(n.into()))
        } else if let Ok(n) = TryInto::<u64>::try_into(data) {
            Ok(Self(n.into()))
        } else if let Ok(b) = TryInto::<&[u8]>::try_into(data) {
            Ok(Self(b.into()))
        } else if let Ok(s) = TryInto::<&str>::try_into(data) {
            Ok(Self(s.into()))
        } else {
            Err(Error::Other("Invalid bytes for Origin".into()))
        }
    }

    pub fn equal(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }

    pub fn not_equal(&self, other: &Self) -> bool {
        self.0.ne(&other.0)
    }

    pub fn less_than(&self, other: &Self) -> bool {
        self.0.lt(&other.0)
    }

    pub fn less_than_equal(&self, other: &Self) -> bool {
        self.0.le(&other.0)
    }

    pub fn greater_than(&self, other: &Self) -> bool {
        self.0.gt(&other.0)
    }

    pub fn greater_than_equal(&self, other: &Self) -> bool {
        self.0.ge(&other.0)
    }

    // Method must be explicitly visible by extendr to be bound to R.
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        self.0.to_string()
    }

    pub fn __yr_print__(&self) -> String {
        self.0.to_string()
    }

    pub fn to_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }

    pub fn to_hex(&self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";

        self.0
            .as_ref()
            .iter()
            .flat_map(|&b| {
                [
                    HEX[(b >> 4) as usize] as char,
                    HEX[(b & 0x0f) as usize] as char,
                ]
            })
            .collect()
    }
}

impl Origin {
    pub(crate) fn new(data: &Robj) -> Result<Self, Error> {
        Self::new_ec(data)
    }
}

impl From<Origin> for yrs::Origin {
    fn from(value: Origin) -> Self {
        value.0
    }
}

extendr_module! {
    mod transaction;
    impl Transaction;
    impl Origin;
}
