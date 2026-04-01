use extendr_api::prelude::*;
use yrs::types::{Attrs as YAttrs, Delta as YDelta, EntryChange as YEntryChange};
use yrs::{Any as YAny, Out as YOut};

pub trait IntoExtendr<T> {
    fn extendr(self) -> extendr_api::Result<T>;
}

impl<T, E: ToString> IntoExtendr<T> for Result<T, E> {
    fn extendr(self) -> extendr_api::Result<T> {
        self.map_err(|e| Error::Other(e.to_string()))
    }
}

impl<T: IntoExtendr<Robj>> IntoExtendr<Robj> for Option<T> {
    fn extendr(self) -> extendr_api::Result<Robj> {
        match self {
            None => Ok(Robj::from(())),
            Some(v) => v.extendr(),
        }
    }
}

impl<K, V> IntoExtendr<Robj> for &std::collections::HashMap<K, V>
where
    K: AsRef<str>,
    for<'a> &'a V: IntoExtendr<Robj>,
{
    fn extendr(self) -> extendr_api::Result<Robj> {
        let n = self.len();
        let mut keys = Strings::new(n);
        let mut values = List::new(n);
        for (i, (k, v)) in self.iter().enumerate() {
            keys.set_elt(i, k.as_ref().into());
            values.set_elt(i, v.extendr()?)?;
        }
        if n > 0 {
            values.set_names(keys.as_slice())?;
        }
        Ok(values.into_robj())
    }
}

impl<T> IntoExtendr<Robj> for &[T]
where
    for<'a> &'a T: IntoExtendr<Robj>,
{
    fn extendr(self) -> extendr_api::Result<Robj> {
        Ok(List::from_values(self.iter().map(|e| e.extendr())).into())
    }
}

impl IntoExtendr<Robj> for &YAny {
    fn extendr(self) -> extendr_api::Result<Robj> {
        match self {
            YAny::Null | YAny::Undefined => Ok(().into()),
            YAny::Bool(v) => Ok(v.into()),
            YAny::Number(v) => Ok(v.into()),
            // R has no native i64; use i32 if it fits, otherwise error
            YAny::BigInt(v) => {
                let v = i32::try_from(*v)
                    .map_err(|_| Error::Other(format!("{v} does not fit in i32")))?;
                Ok(v.into())
            }
            YAny::String(v) => Ok(v.as_ref().into()),
            YAny::Buffer(v) => Ok(Raw::from_bytes(v.as_ref()).into()),
            YAny::Array(v) => v.extendr(),
            YAny::Map(v) => v.extendr(),
        }
    }
}

impl IntoExtendr<Robj> for YOut {
    fn extendr(self) -> extendr_api::Result<Robj> {
        match self {
            YOut::Any(v) => v.extendr(),
            YOut::YText(v) => Ok(crate::TextRef::from(v).into()),
            YOut::YArray(v) => Ok(crate::ArrayRef::from(v).into()),
            YOut::YMap(v) => Ok(crate::MapRef::from(v).into()),
            YOut::YDoc(v) => Ok(crate::Doc::from(v).into()),
            YOut::YXmlElement(_) => {
                Err(Error::Other("YXmlElement is not yet supported".to_string()))
            }
            YOut::YXmlFragment(_) => Err(Error::Other(
                "YXmlFragment is not yet supported".to_string(),
            )),
            YOut::YXmlText(_) => Err(Error::Other("YXmlText is not yet supported".to_string())),
            YOut::UndefinedRef(_) => Err(Error::Other("UndefinedRef is not supported".to_string())),
        }
    }
}

impl IntoExtendr<Robj> for &YDelta<YOut> {
    fn extendr(self) -> extendr_api::Result<Robj> {
        match self {
            YDelta::Inserted(content, attrs) => Ok(List::from_names_and_values(
                ["insert", "attributes"],
                [content.clone().extendr()?, attrs.as_deref().extendr()?],
            )?
            .into()),
            YDelta::Deleted(n) => {
                let n = i32::try_from(*n)
                    .map_err(|_| Error::Other(format!("{n} does not fit in i32")))?;
                Ok(List::from_names_and_values(["delete"], [Robj::from(n)])?.into())
            }
            YDelta::Retain(n, attrs) => {
                let n = i32::try_from(*n)
                    .map_err(|_| Error::Other(format!("{n} does not fit in i32")))?;
                Ok(List::from_names_and_values(
                    ["retain", "attributes"],
                    [Robj::from(n), attrs.as_deref().extendr()?],
                )?
                .into())
            }
        }
    }
}

impl IntoExtendr<Robj> for &YEntryChange {
    fn extendr(self) -> extendr_api::Result<Robj> {
        match self {
            YEntryChange::Inserted(new) => {
                Ok(List::from_names_and_values(["inserted"], [new.clone().extendr()?])?.into())
            }
            YEntryChange::Updated(old, new) => Ok(List::from_names_and_values(
                ["removed", "inserted"],
                [old.clone().extendr()?, new.clone().extendr()?],
            )?
            .into()),
            YEntryChange::Removed(old) => {
                Ok(List::from_names_and_values(["removed"], [old.clone().extendr()?])?.into())
            }
        }
    }
}

pub trait FromExtendr<T>: Sized {
    fn from_extendr(value: T) -> extendr_api::Result<Self>;
}

impl FromExtendr<Robj> for YAny {
    fn from_extendr(robj: Robj) -> extendr_api::Result<Self> {
        if robj.is_null() {
            Ok(YAny::Null)
        } else if let Some(v) = robj.as_bool() {
            Ok(YAny::Bool(v))
        } else if let Some(v) = robj.as_integer() {
            Ok(YAny::BigInt(v as i64))
        } else if let Some(v) = robj.as_real() {
            Ok(YAny::Number(v))
        } else if let Some(v) = robj.as_str() {
            Ok(YAny::String(std::sync::Arc::from(v)))
        } else if robj.is_raw() {
            let raw = Raw::try_from(robj).unwrap();
            Ok(YAny::Buffer(std::sync::Arc::from(raw.as_slice())))
        } else if robj.is_list() {
            let list = robj.as_list().unwrap();
            if robj.names().is_some() {
                let map = std::collections::HashMap::<String, Robj>::try_from(list)
                    .unwrap()
                    .into_iter()
                    .map(|(k, v)| Ok((k, YAny::from_extendr(v)?)))
                    .collect::<extendr_api::Result<_>>()?;
                Ok(YAny::Map(std::sync::Arc::new(map)))
            } else {
                let arr = list
                    .values()
                    .map(YAny::from_extendr)
                    .collect::<extendr_api::Result<Vec<_>>>()?;
                Ok(YAny::Array(std::sync::Arc::from(arr.as_slice())))
            }
        } else {
            Err(Error::Other(format!(
                "Cannot convert {:?} to YAny",
                robj.rtype()
            )))
        }
    }
}

impl FromExtendr<Robj> for YAttrs {
    fn from_extendr(robj: Robj) -> extendr_api::Result<Self> {
        if !robj.is_list() {
            return Err(Error::Other(format!(
                "Expected a named list for Attrs, got {:?}",
                robj.rtype()
            )));
        }
        let list = robj.as_list().unwrap();
        if list.is_empty() {
            return Ok(Self::default());
        }
        // In R, a partially named list has names() return Some but with empty
        // strings for unnamed elements.
        let fully_named = list
            .names()
            .map(|mut ns| ns.all(|n| !n.is_empty()))
            .unwrap_or(false);
        if !fully_named {
            return Err(Error::Other(
                "Expected a fully named list for Attrs".to_string(),
            ));
        }
        list.names()
            .unwrap()
            .zip(list.values())
            .map(|(k, v)| Ok((std::sync::Arc::from(k), YAny::from_extendr(v)?)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use super::*;

    #[test]
    fn test_to_any_null() {
        extendr_api::test! {
            assert_eq!(YAny::Null.extendr().unwrap(), r!(NULL));
            assert_eq!(YAny::Undefined.extendr().unwrap(), r!(NULL));
        }
    }

    #[test]
    fn test_to_any_bool() {
        extendr_api::test! {
            assert_eq!(YAny::Bool(true).extendr().unwrap(), r!(true));
            assert_eq!(YAny::Bool(false).extendr().unwrap(), r!(false));
        }
    }

    #[test]
    fn test_to_any_number() {
        extendr_api::test! {
            assert_eq!(YAny::Number(1.5).extendr().unwrap(), r!(1.5));
        }
    }

    #[test]
    fn test_to_any_bigint() {
        extendr_api::test! {
            assert_eq!(YAny::BigInt(42).extendr().unwrap(), r!(42i32));
            assert!(YAny::BigInt(i64::MAX).extendr().is_err());
        }
    }

    #[test]
    fn test_to_any_string() {
        extendr_api::test! {
            assert_eq!(YAny::String(Arc::from("hello")).extendr().unwrap(), r!("hello"));
        }
    }

    #[test]
    fn test_to_any_buffer() {
        extendr_api::test! {
            let buf: Arc<[u8]> = Arc::from([1u8, 2, 3].as_slice());
            let robj = YAny::Buffer(buf).extendr().unwrap();
            assert!(robj.is_raw());
            assert_eq!(robj.len(), 3);
        }
    }

    #[test]
    fn test_to_any_array() {
        extendr_api::test! {
            let arr: Arc<[YAny]> = Arc::from([YAny::Bool(true), YAny::Number(1.0)].as_slice());
            assert_eq!(YAny::Array(arr).extendr().unwrap(), R!(r#"list(TRUE, 1.0)"#).unwrap());
        }
    }

    #[test]
    fn test_to_any_map() {
        extendr_api::test! {
            let map: Arc<HashMap<String, YAny>> =
                Arc::new(HashMap::from([("x".to_string(), YAny::Number(1.0))]));
            assert_eq!(YAny::Map(map).extendr().unwrap(), R!(r#"list(x=1.0)"#).unwrap());
        }
    }

    #[test]
    fn test_to_delta_inserted() {
        extendr_api::test! {
            let delta = YDelta::Inserted(YOut::Any(YAny::String(std::sync::Arc::from("hello"))), None);
            let robj = delta.extendr().unwrap();
            let expected = R!(r#"list(insert="hello", attributes=NULL)"#).unwrap();
            assert_eq!(robj , expected);
        }
    }

    #[test]
    fn test_to_delta_inserted_with_attrs() {
        extendr_api::test! {
            let attrs = Box::new(YAttrs::from_iter([
                (std::sync::Arc::from("bold"), YAny::Bool(true)),
            ]));
            let delta = YDelta::Inserted(YOut::Any(YAny::String(std::sync::Arc::from("hi"))), Some(attrs));
            assert_eq!(delta.extendr().unwrap(), R!(r#"list(insert="hi", attributes=list(bold=TRUE))"#).unwrap());
        }
    }

    #[test]
    fn test_to_delta_deleted() {
        extendr_api::test! {
            let delta: YDelta<YOut> = YDelta::Deleted(3);
            assert_eq!(delta.extendr().unwrap(), R!(r#"list(delete=3L)"#).unwrap());
        }
    }

    #[test]
    fn test_to_delta_retain() {
        extendr_api::test! {
            let delta: YDelta<YOut> = YDelta::Retain(5, None);
            assert_eq!(delta.extendr().unwrap(), R!(r#"list(retain=5L, attributes=NULL)"#).unwrap());
        }
    }

    #[test]
    fn test_to_out_any() {
        extendr_api::test! {
            assert_eq!(YOut::Any(YAny::Null).extendr().unwrap(), r!(NULL));
            assert_eq!(YOut::Any(YAny::Number(1.5)).extendr().unwrap(), r!(1.5));
        }
    }

    #[test]
    fn test_to_out_ytext() {
        extendr_api::test! {
            let doc = yrs::Doc::new();
            let text_ref = doc.get_or_insert_text("test");
            let robj = YOut::YText(text_ref).extendr().unwrap();
            assert!(robj.is_external_pointer());
        }
    }

    #[test]
    fn test_to_out_yarray() {
        extendr_api::test! {
            let doc = yrs::Doc::new();
            let array_ref = doc.get_or_insert_array("test");
            let robj = YOut::YArray(array_ref).extendr().unwrap();
            assert!(robj.is_external_pointer());
        }
    }

    #[test]
    fn test_to_out_ymap() {
        extendr_api::test! {
            let doc = yrs::Doc::new();
            let map_ref = doc.get_or_insert_map("test");
            let robj = YOut::YMap(map_ref).extendr().unwrap();
            assert!(robj.is_external_pointer());
        }
    }

    #[test]
    fn test_to_out_ydoc() {
        extendr_api::test! {
            let subdoc = yrs::Doc::new();
            let robj = YOut::YDoc(subdoc).extendr().unwrap();
            assert!(robj.is_external_pointer());
        }
    }

    #[test]
    fn test_to_attrs() {
        extendr_api::test! {
            let attrs: YAttrs = HashMap::from([
                (Arc::from("bold"), YAny::Bool(true)),
                (Arc::from("color"), YAny::String(Arc::from("red"))),
            ]);
            let robj = attrs.extendr().unwrap();
            assert!(robj.is_list());
            assert_eq!(robj.len(), 2);
            assert!(robj.names().is_some());
        }
    }

    #[test]
    fn test_to_attrs_empty() {
        extendr_api::test! {
            let attrs: YAttrs = HashMap::new();
            assert_eq!(attrs.extendr().unwrap(), R!(r#"list()"#).unwrap());
        }
    }

    #[test]
    fn test_to_entry_change_inserted() {
        extendr_api::test! {
            let change = YEntryChange::Inserted(YOut::Any(YAny::Number(1.5)));
            let robj = change.extendr().unwrap();
            assert_eq!(robj, R!(r#"list(inserted=1.5)"#).unwrap());
        }
    }

    #[test]
    fn test_to_entry_change_removed() {
        extendr_api::test! {
            let change = YEntryChange::Removed(YOut::Any(YAny::Bool(true)));
            let robj = change.extendr().unwrap();
            assert_eq!(robj, R!(r#"list(removed=TRUE)"#).unwrap());
        }
    }

    #[test]
    fn test_to_entry_change_updated() {
        extendr_api::test! {
            let change = YEntryChange::Updated(
                YOut::Any(YAny::String(Arc::from("old"))),
                YOut::Any(YAny::String(Arc::from("new"))),
            );
            let robj = change.extendr().unwrap();
            assert_eq!(robj, R!(r#"list(removed="old", inserted="new")"#).unwrap());
        }
    }

    #[test]
    fn test_from_any_null() {
        extendr_api::test! {
            assert!(matches!(YAny::from_extendr(r!(NULL)).unwrap(), YAny::Null));
        }
    }

    #[test]
    fn test_from_any_bool() {
        extendr_api::test! {
            assert!(matches!(YAny::from_extendr(r!(true)).unwrap(), YAny::Bool(true)));
            assert!(matches!(YAny::from_extendr(r!(false)).unwrap(), YAny::Bool(false)));
        }
    }

    #[test]
    fn test_from_any_integer() {
        extendr_api::test! {
            assert!(matches!(YAny::from_extendr(r!(42i32)).unwrap(), YAny::BigInt(42)));
        }
    }

    #[test]
    fn test_from_any_number() {
        extendr_api::test! {
            assert!(matches!(YAny::from_extendr(r!(1.5)).unwrap(), YAny::Number(v) if v == 1.5));
        }
    }

    #[test]
    fn test_from_any_string() {
        extendr_api::test! {
            assert!(matches!(YAny::from_extendr(r!("hello")).unwrap(), YAny::String(ref s) if s.as_ref() == "hello"));
        }
    }

    #[test]
    fn test_from_any_buffer() {
        extendr_api::test! {
            let robj: Robj = Raw::from_bytes(&[1, 2, 3]).into();
            assert!(matches!(YAny::from_extendr(robj).unwrap(), YAny::Buffer(ref b) if b.len() == 3));
        }
    }

    #[test]
    fn test_from_any_array() {
        extendr_api::test! {
            assert!(matches!(YAny::from_extendr(R!(r#"list(TRUE, 1.5)"#).unwrap()).unwrap(), YAny::Array(ref a) if a.len() == 2));
        }
    }

    #[test]
    fn test_from_any_map() {
        extendr_api::test! {
            assert!(matches!(YAny::from_extendr(R!(r#"list(x=1.5)"#).unwrap()).unwrap(), YAny::Map(ref m) if m.len() == 1));
        }
    }

    #[test]
    fn test_from_attrs() {
        extendr_api::test! {
            let attrs = YAttrs::from_extendr(R!(r#"list(bold=TRUE, size=12.0)"#).unwrap()).unwrap();
            assert_eq!(attrs.len(), 2);
            assert!(matches!(attrs.get("bold"), Some(YAny::Bool(true))));
            assert!(matches!(attrs.get("size"), Some(YAny::Number(v)) if v == &12.0));
        }
    }

    #[test]
    fn test_from_attrs_empty() {
        extendr_api::test! {
            let attrs = YAttrs::from_extendr(R!(r#"list()"#).unwrap()).unwrap();
            assert_eq!(attrs.len(), 0);
        }
    }

    #[test]
    fn test_from_attrs_not_a_list() {
        extendr_api::test! {
            assert!(YAttrs::from_extendr(r!(42.0)).is_err());
        }
    }

    #[test]
    fn test_from_attrs_unnamed_list() {
        extendr_api::test! {
            assert!(YAttrs::from_extendr(R!(r#"list(TRUE)"#).unwrap()).is_err());
        }
    }

    #[test]
    fn test_from_attrs_partially_named_list() {
        extendr_api::test! {
            // "bold" is named but second element is not (name = "")
            assert!(YAttrs::from_extendr(R!(r#"list(bold=TRUE, 1.0)"#).unwrap()).is_err());
        }
    }
}
