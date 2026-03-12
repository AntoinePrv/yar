use extendr_api::prelude::*;
use yrs::{GetString as YGetString, Text as YText, Transact as YTransact};

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

#[extendr]
impl Transaction {
    fn new(doc: ExternalPtr<Doc>, #[extendr(default = "FALSE")] readonly: bool) -> Self {
        // Safety: Doc live in R memory and is kept alive in the owner field of this struct
        let transaction: DynTransaction<'static> = if readonly {
            unsafe { DynTransaction::Read(std::mem::transmute(doc.doc.transact())) }
        } else {
            unsafe { DynTransaction::Write(std::mem::transmute(doc.doc.transact_mut())) }
        };
        Transaction {
            owner: doc.into(),
            transaction: Some(transaction),
        }
    }

    fn commit(&mut self) -> Result<(), Error> {
        use DynTransaction::{Read, Write};
        match &mut self.transaction {
            Some(Write(trans)) => {
                trans.commit();
                Ok(())
            }
            Some(Read(_)) => Err(Error::Other("Transaction is readonly".into())),
            None => Err(Error::Other("Transaction was dropped".into())),
        }
    }

    fn drop(&mut self) {
        self.transaction = None;
    }
}

#[extendr]
struct TextRef(yrs::TextRef);

#[extendr]
impl TextRef {
    fn insert(&self, transaction: &mut Transaction, index: u32, chunk: &str) -> Result<(), Error> {
        use DynTransaction::{Read, Write};
        match &mut transaction.transaction {
            Some(Write(trans)) => {
                self.0.insert(trans, index, chunk);
                Ok(())
            }
            Some(Read(_)) => Err(Error::Other("Transaction is readonly".into())),
            None => Err(Error::Other("Transaction was dropped".into())),
        }
    }

    fn get_string(&self, transaction: &Transaction) -> Result<String, Error> {
        use DynTransaction::{Read, Write};
        match &transaction.transaction {
            Some(Write(trans)) => Ok(self.0.get_string(trans)),
            Some(Read(trans)) => Ok(self.0.get_string(trans)),
            None => Err(Error::Other("Transaction was dropped".into())),
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

// Register function with R.
// See corresponding C code in `entrypoint.c`.
extendr_module! {
    mod yar;
    impl Transaction;
    impl TextRef;
    impl Doc;
}
