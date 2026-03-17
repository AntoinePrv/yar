mod type_conversion;

use extendr_api::prelude::*;
use yrs::updates::{decoder::Decode as YDecode, encoder::Encode as YEncode};
use yrs::{
    Array as YArray, ArrayPrelim, GetString as YGetString, Map as YMap, MapPrelim,
    ReadTxn as YReadTxn, Text as YText, TextPrelim, Transact as YTransact,
};

use crate::type_conversion::{FromExtendr, IntoExtendr};

macro_rules! try_read {
    ($txn:expr, $t:ident => $body:expr) => {
        $txn.try_dyn().map(|txn| match txn {
            DynTransaction::Write($t) => $body,
            DynTransaction::Read($t) => $body,
        })
    };
}

// Perhaps we could have two different bindings of Transaction and TransactionMut
// with the same API and use a macro to bind YTransact trait methods.
#[allow(clippy::large_enum_variant)]
enum DynTransaction<'doc> {
    Read(yrs::Transaction<'doc>),
    Write(yrs::TransactionMut<'doc>),
}

#[extendr]
struct Transaction {
    // Transaction auto commits on Drop, and keeps a lock
    // We need to be able to explicitly drop the lock.
    transaction: Option<DynTransaction<'static>>,
    // Keep Document alive while the transaction is alive
    #[allow(dead_code)]
    owner: Robj,
}

impl Transaction {
    fn try_dyn(&self) -> Result<&DynTransaction<'static>, Error> {
        match &self.transaction {
            Some(trans) => Ok(trans),
            None => Err(Error::Other("Transaction was dropped".into())),
        }
    }

    fn try_mut(&mut self) -> Result<&mut yrs::TransactionMut<'static>, Error> {
        use DynTransaction::{Read, Write};
        match &mut self.transaction {
            Some(Write(trans)) => Ok(trans),
            Some(Read(_)) => Err(Error::Other("Transaction is readonly".into())),
            None => Err(Error::Other("Transaction was dropped".into())),
        }
    }
}

#[extendr]
impl Transaction {
    fn new(doc: ExternalPtr<Doc>, #[extendr(default = "FALSE")] mutable: bool) -> Self {
        // Safety: Doc live in R memory and is kept alive in the owner field of this struct
        let transaction: DynTransaction<'static> = if mutable {
            unsafe { DynTransaction::Write(std::mem::transmute(doc.doc.transact_mut())) }
        } else {
            unsafe { DynTransaction::Read(std::mem::transmute(doc.doc.transact())) }
        };
        Transaction {
            owner: doc.into(),
            transaction: Some(transaction),
        }
    }

    fn commit(&mut self) -> Result<(), Error> {
        self.try_mut().map(|trans| trans.commit())
    }

    fn drop(&mut self) {
        self.transaction = None;
    }

    fn state_vector(&self) -> Result<StateVector, Error> {
        try_read!(self, t => t.state_vector().into())
    }

    fn encode_diff_v1(&self, state_vector: &StateVector) -> Result<Vec<u8>, Error> {
        try_read!(self, t => t.encode_diff_v1(state_vector))
    }

    fn encode_diff_v2(&self, state_vector: &StateVector) -> Result<Vec<u8>, Error> {
        try_read!(self, t => t.encode_diff_v2(state_vector))
    }

    fn apply_update_v1(&mut self, data: &[u8]) -> Result<(), Error> {
        let trans = self.try_mut()?;
        let update = yrs::Update::decode_v1(data).extendr()?;
        trans.apply_update(update).extendr()
    }

    fn apply_update_v2(&mut self, data: &[u8]) -> Result<(), Error> {
        let trans = self.try_mut()?;
        let update = yrs::Update::decode_v2(data).extendr()?;
        trans.apply_update(update).extendr()
    }
}

#[extendr]
struct Update(yrs::Update);

impl From<yrs::Update> for Update {
    fn from(value: yrs::Update) -> Self {
        Self(value)
    }
}

#[extendr]
impl Update {
    fn decode_v1(data: &[u8]) -> Result<Self, Error> {
        Ok(Self(yrs::Update::decode_v1(data).extendr()?))
    }

    fn decode_v2(data: &[u8]) -> Result<Self, Error> {
        Ok(Self(yrs::Update::decode_v2(data).extendr()?))
    }

    fn new() -> Self {
        Self(yrs::Update::new())
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn extends(&self, state_vector: &StateVector) -> bool {
        self.0.extends(state_vector)
    }

    fn encode_v1(&self) -> Vec<u8> {
        self.0.encode_v1()
    }

    fn encode_v2(&self) -> Vec<u8> {
        self.0.encode_v2()
    }

    fn state_vector(&self) -> StateVector {
        self.0.state_vector().into()
    }

    fn state_vector_lower(&self) -> StateVector {
        self.0.state_vector_lower().into()
    }
}

#[extendr]
struct TextRef(yrs::TextRef);

impl From<yrs::TextRef> for TextRef {
    fn from(value: yrs::TextRef) -> Self {
        Self(value)
    }
}

impl std::ops::Deref for TextRef {
    type Target = yrs::TextRef;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[extendr]
impl TextRef {
    fn len(&self, transaction: &Transaction) -> Result<u32, Error> {
        try_read!(transaction, t => self.0.len(t))
    }

    fn insert(&self, transaction: &mut Transaction, index: u32, chunk: &str) -> Result<(), Error> {
        transaction
            .try_mut()
            .map(|trans| self.0.insert(trans, index, chunk))
    }

    fn push(&self, transaction: &mut Transaction, chunk: &str) -> Result<(), Error> {
        transaction.try_mut().map(|trans| self.0.push(trans, chunk))
    }

    fn remove_range(
        &self,
        transaction: &mut Transaction,
        index: u32,
        len: u32,
    ) -> Result<(), Error> {
        transaction
            .try_mut()
            .map(|trans| self.0.remove_range(trans, index, len))
    }

    fn get_string(&self, transaction: &Transaction) -> Result<String, Error> {
        try_read!(transaction, t => self.0.get_string(t))
    }
}

#[extendr]
struct Doc {
    doc: yrs::Doc,
}

#[extendr]
impl Doc {
    fn new() -> Self {
        Self {
            doc: yrs::Doc::new(),
        }
    }

    fn client_id(&self) -> u64 {
        self.doc.client_id()
    }

    fn guid(&self) -> Strings {
        (*self.doc.guid()).into()
    }

    fn get_or_insert_text(&self, name: &str) -> TextRef {
        TextRef(self.doc.get_or_insert_text(name))
    }

    fn get_or_insert_map(&self, name: &str) -> MapRef {
        MapRef(self.doc.get_or_insert_map(name))
    }
}

#[extendr]
struct StateVector(yrs::StateVector);

impl From<yrs::StateVector> for StateVector {
    fn from(value: yrs::StateVector) -> Self {
        Self(value)
    }
}

impl std::ops::Deref for StateVector {
    type Target = yrs::StateVector;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[extendr]
impl StateVector {
    fn decode_v1(data: &[u8]) -> Result<Self, Error> {
        Ok(Self(yrs::StateVector::decode_v1(data).extendr()?))
    }

    fn decode_v2(data: &[u8]) -> Result<Self, Error> {
        Ok(Self(yrs::StateVector::decode_v2(data).extendr()?))
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn contains_client(&self, client_id: yrs::block::ClientID) -> bool {
        self.0.contains_client(&client_id)
    }

    fn encode_v1(&self) -> Vec<u8> {
        self.0.encode_v1()
    }

    fn encode_v2(&self) -> Vec<u8> {
        self.0.encode_v2()
    }
}

#[extendr]
struct ArrayRef(yrs::ArrayRef);

impl From<yrs::ArrayRef> for ArrayRef {
    fn from(value: yrs::ArrayRef) -> Self {
        Self(value)
    }
}

impl std::ops::Deref for ArrayRef {
    type Target = yrs::ArrayRef;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[extendr]
impl ArrayRef {
    fn len(&self, transaction: &Transaction) -> Result<u32, Error> {
        try_read!(transaction, t => self.0.len(t))
    }

    fn insert_any(
        &self,
        transaction: &mut Transaction,
        index: u32,
        obj: Robj,
    ) -> Result<(), Error> {
        let trans = transaction.try_mut()?;
        let any = yrs::Any::from_extendr(obj)?;
        self.0.insert(trans, index, any);
        Ok(())
    }

    fn insert_text(&self, transaction: &mut Transaction, index: u32) -> Result<TextRef, Error> {
        transaction
            .try_mut()
            .map(|trans| TextRef::from(self.0.insert(trans, index, TextPrelim::default())))
    }

    fn insert_array(&self, transaction: &mut Transaction, index: u32) -> Result<ArrayRef, Error> {
        transaction
            .try_mut()
            .map(|trans| ArrayRef::from(self.0.insert(trans, index, ArrayPrelim::default())))
    }

    fn insert_map(&self, transaction: &mut Transaction, index: u32) -> Result<MapRef, Error> {
        transaction
            .try_mut()
            .map(|trans| MapRef::from(self.0.insert(trans, index, MapPrelim::default())))
    }
}

#[extendr]
struct MapRef(yrs::MapRef);

impl From<yrs::MapRef> for MapRef {
    fn from(value: yrs::MapRef) -> Self {
        Self(value)
    }
}

impl std::ops::Deref for MapRef {
    type Target = yrs::MapRef;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[extendr]
impl MapRef {
    fn len(&self, transaction: &Transaction) -> Result<u32, Error> {
        try_read!(transaction, t => self.0.len(t))
    }

    fn contains_key(&self, transaction: &Transaction, key: &str) -> Result<bool, Error> {
        try_read!(transaction, t => self.0.contains_key(t, key))
    }

    fn insert_any(&self, transaction: &mut Transaction, key: &str, obj: Robj) -> Result<(), Error> {
        let trans = transaction.try_mut()?;
        let any = yrs::Any::from_extendr(obj)?;
        self.0.insert(trans, key, any);
        Ok(())
    }

    fn insert_text(&self, transaction: &mut Transaction, key: &str) -> Result<TextRef, Error> {
        transaction
            .try_mut()
            .map(|trans| TextRef::from(self.0.insert(trans, key, TextPrelim::default())))
    }

    fn insert_array(&self, transaction: &mut Transaction, key: &str) -> Result<ArrayRef, Error> {
        transaction
            .try_mut()
            .map(|trans| ArrayRef::from(self.0.insert(trans, key, ArrayPrelim::default())))
    }

    fn insert_map(&self, transaction: &mut Transaction, key: &str) -> Result<MapRef, Error> {
        transaction
            .try_mut()
            .map(|trans| MapRef::from(self.0.insert(trans, key, MapPrelim::default())))
    }

    fn remove(&self, transaction: &mut Transaction, key: &str) -> Result<(), Error> {
        transaction.try_mut().map(|trans| {
            self.0.remove(trans, key);
        })
    }

    fn clear(&self, transaction: &mut Transaction) -> Result<(), Error> {
        transaction.try_mut().map(|trans| self.0.clear(trans))
    }
}

// Register function with R.
// See corresponding C code in `entrypoint.c`.
extendr_module! {
    mod yar;
    impl ArrayRef;
    impl Doc;
    impl MapRef;
    impl StateVector;
    impl TextRef;
    impl Transaction;
    impl Update;
}
