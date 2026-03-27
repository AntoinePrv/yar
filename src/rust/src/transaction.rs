use extendr_api::prelude::*;
use yrs::updates::decoder::Decode as YDecode;
use yrs::{ReadTxn as YReadTxn, Transact as YTransact};

use crate::type_conversion::IntoExtendr;
use crate::{Doc, StateVector};

macro_rules! try_read {
    ($txn:expr, $t:ident => $body:expr) => {
        $txn.try_dyn().map(|txn| match txn {
            crate::transaction::DynTransaction::Write($t) => $body,
            crate::transaction::DynTransaction::Read($t) => $body,
        })
    };
}

pub(crate) use try_read;

// Perhaps we could have two different bindings of Transaction and TransactionMut
// with the same API and use a macro to bind YTransact trait methods.
#[allow(clippy::large_enum_variant)]
pub enum DynTransaction<'doc> {
    Read(yrs::Transaction<'doc>),
    Write(yrs::TransactionMut<'doc>),
}

#[extendr]
pub struct Transaction {
    // Transaction auto commits on Drop, and keeps a lock onto the Doc.
    transaction: std::mem::ManuallyDrop<Option<DynTransaction<'static>>>,
    // Keeps the Doc alive while the transaction is alive.
    #[allow(dead_code)]
    owner: Robj,
}

impl Drop for Transaction {
    fn drop(&mut self) {
        // Safety: transaction must be dropped before owner so that the Doc is still
        // alive when the transaction releases its borrow/lock on it.
        unsafe { std::mem::ManuallyDrop::drop(&mut self.transaction) };
        // owner drops here, potentially allowing the Doc to be freed by R's GC.
    }
}

impl Transaction {
    pub(crate) fn try_dyn(&self) -> Result<&DynTransaction<'static>, Error> {
        match &*self.transaction {
            Some(trans) => Ok(trans),
            None => Err(Error::Other("Transaction was dropped".into())),
        }
    }

    pub(crate) fn try_mut(&mut self) -> Result<&mut yrs::TransactionMut<'static>, Error> {
        use DynTransaction::{Read, Write};
        match &mut *self.transaction {
            Some(Write(trans)) => Ok(trans),
            Some(Read(_)) => Err(Error::Other("Transaction is readonly".into())),
            None => Err(Error::Other("Transaction was dropped".into())),
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
        let transaction = match (mutable, origin) {
            (true, Nullable::NotNull(o)) => {
                DynTransaction::Write(doc.transact_mut_with(o.0.clone()))
            }
            (true, Nullable::Null) => DynTransaction::Write(doc.transact_mut()),
            (false, _) => DynTransaction::Read(doc.transact()),
        };

        // Safety: Doc lives in R memory and is kept alive by the `owner` field of this struct.
        // R's GC is non-moving, so the pointer inside the transaction remains valid as long as
        // the Doc is not freed. `owner: Robj` prevents collection via R_PreserveObject semantics.
        let transaction = unsafe {
            std::mem::transmute::<DynTransaction<'_>, DynTransaction<'static>>(transaction)
        };
        Transaction {
            owner: doc.into(),
            transaction: std::mem::ManuallyDrop::new(Some(transaction)),
        }
    }

    pub fn origin(&self) -> Result<Robj, Error> {
        match self.try_dyn()? {
            DynTransaction::Write(trans) => match trans.origin() {
                Some(o) => Ok(Origin(o.clone()).into()),
                None => Ok(r!(NULL)),
            },
            DynTransaction::Read(_) => Ok(r!(NULL)),
        }
    }

    pub fn commit(&mut self) -> Result<(), Error> {
        self.try_mut().map(|trans| trans.commit())
    }

    pub fn unlock(&mut self) {
        *self.transaction = None;
    }

    pub fn state_vector(&self) -> Result<StateVector, Error> {
        try_read!(self, t => t.state_vector().into())
    }

    pub fn encode_diff_v1(&self, state_vector: &StateVector) -> Result<Vec<u8>, Error> {
        try_read!(self, t => t.encode_diff_v1(state_vector))
    }

    pub fn encode_diff_v2(&self, state_vector: &StateVector) -> Result<Vec<u8>, Error> {
        try_read!(self, t => t.encode_diff_v2(state_vector))
    }

    pub fn apply_update_v1(&mut self, data: &[u8]) -> Result<(), Error> {
        let trans = self.try_mut()?;
        let update = yrs::Update::decode_v1(data).extendr()?;
        trans.apply_update(update).extendr()
    }

    pub fn apply_update_v2(&mut self, data: &[u8]) -> Result<(), Error> {
        let trans = self.try_mut()?;
        let update = yrs::Update::decode_v2(data).extendr()?;
        trans.apply_update(update).extendr()
    }
}

#[extendr]
pub struct Origin(yrs::Origin);

#[extendr]
impl Origin {
    fn new(data: &Robj) -> Result<Self, Error> {
        if let Ok(n) = TryInto::<i64>::try_into(data) {
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

    fn equal(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn less_than(&self, other: &Self) -> bool {
        self.0 < other.0
    }

    fn less_than_equal(&self, other: &Self) -> bool {
        self.0 <= other.0
    }

    fn to_string(&self) -> String {
        self.0.to_string()
    }

    fn to_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }

    fn to_hex(&self) -> String {
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

extendr_module! {
    mod transaction;
    impl Transaction;
    impl Origin;
}
