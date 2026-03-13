use extendr_api::prelude::*;
use yrs::updates::{decoder::Decode as YDecode, encoder::Encode as YEncode};
use yrs::{GetString as YGetString, ReadTxn as YReadTxn, Text as YText, Transact as YTransact};

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
    fn try_dyn_transaction(&self) -> Result<&DynTransaction<'static>, Error> {
        match &self.transaction {
            Some(trans) => Ok(trans),
            None => Err(Error::Other("Transaction was dropped".into())),
        }
    }

    fn try_transaction_mut(&mut self) -> Result<&mut yrs::TransactionMut<'static>, Error> {
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
        self.try_transaction_mut()?.commit();
        Ok(())
    }

    fn drop(&mut self) {
        self.transaction = None;
    }

    fn state_vector(&self) -> Result<StateVector, Error> {
        use DynTransaction::{Read, Write};
        match &self.try_dyn_transaction()? {
            Write(trans) => Ok(StateVector(trans.state_vector())),
            Read(trans) => Ok(StateVector(trans.state_vector())),
        }
    }

    fn encode_diff_v1(&self, state_vector: &StateVector) -> Result<Vec<u8>, Error> {
        use DynTransaction::{Read, Write};
        match &self.try_dyn_transaction()? {
            Write(trans) => Ok(trans.encode_diff_v1(&state_vector.0)),
            Read(trans) => Ok(trans.encode_diff_v1(&state_vector.0)),
        }
    }

    fn encode_diff_v2(&self, state_vector: &StateVector) -> Result<Vec<u8>, Error> {
        use DynTransaction::{Read, Write};
        match &self.try_dyn_transaction()? {
            Write(trans) => Ok(trans.encode_diff_v2(&state_vector.0)),
            Read(trans) => Ok(trans.encode_diff_v2(&state_vector.0)),
        }
    }

    fn apply_update_v1(&mut self, data: &[u8]) -> Result<(), Error> {
        let trans = self.try_transaction_mut()?;
        // FIXME need to properly handle errors coming from yrs
        let update = yrs::Update::decode_v1(data).unwrap();
        trans.apply_update(update).unwrap();
        Ok(())
    }

    fn apply_update_v2(&mut self, data: &[u8]) -> Result<(), Error> {
        let trans = self.try_transaction_mut()?;
        // FIXME need to properly handle errors coming from yrs
        let update = yrs::Update::decode_v2(data).unwrap();
        trans.apply_update(update).unwrap();
        Ok(())
    }
}

#[extendr]
struct Update(yrs::Update);

#[extendr]
impl Update {
    fn decode_v1(data: &[u8]) -> Result<Self, Error> {
        // FIXME need to properly handle errors coming from yrs
        let update = yrs::Update::decode_v1(data).unwrap();
        Ok(Self(update))
    }

    fn decode_v2(data: &[u8]) -> Result<Self, Error> {
        // FIXME need to properly handle errors coming from yrs
        let update = yrs::Update::decode_v2(data).unwrap();
        Ok(Self(update))
    }

    fn new() -> Self {
        Self(yrs::Update::new())
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn encode_v1(&self) -> Vec<u8> {
        self.0.encode_v1()
    }

    fn encode_v2(&self) -> Vec<u8> {
        self.0.encode_v2()
    }
}

#[extendr]
struct TextRef(yrs::TextRef);

#[extendr]
impl TextRef {
    fn insert(&self, transaction: &mut Transaction, index: u32, chunk: &str) -> Result<(), Error> {
        let trans = transaction.try_transaction_mut()?;
        self.0.insert(trans, index, chunk);
        Ok(())
    }

    fn get_string(&self, transaction: &Transaction) -> Result<String, Error> {
        use DynTransaction::{Read, Write};
        match &transaction.try_dyn_transaction()? {
            Write(trans) => Ok(self.0.get_string(trans)),
            Read(trans) => Ok(self.0.get_string(trans)),
        }
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
}

#[extendr]
struct StateVector(yrs::StateVector);

#[extendr]
impl StateVector {
    fn decode_v1(data: &[u8]) -> Result<Self, Error> {
        // FIXME need to properly handle errors coming from yrs
        let sv = yrs::StateVector::decode_v1(data).unwrap();
        Ok(Self(sv))
    }

    fn decode_v2(data: &[u8]) -> Result<Self, Error> {
        // FIXME need to properly handle errors coming from yrs
        let sv = yrs::StateVector::decode_v2(data).unwrap();
        Ok(Self(sv))
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

// Register function with R.
// See corresponding C code in `entrypoint.c`.
extendr_module! {
    mod yar;
    impl Doc;
    impl StateVector;
    impl TextRef;
    impl Transaction;
    impl Update;
}
