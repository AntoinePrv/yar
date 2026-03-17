use extendr_api::prelude::*;

pub(crate) trait IntoExtendr<T> {
    fn extendr(self) -> extendr_api::Result<T>;
}

impl<T, E: ToString> IntoExtendr<T> for Result<T, E> {
    fn extendr(self) -> extendr_api::Result<T> {
        self.map_err(|e| Error::Other(e.to_string()))
    }
}

impl IntoExtendr<Robj> for yrs::Any {
    fn extendr(self) -> extendr_api::Result<Robj> {
        Ok(match self {
            yrs::Any::Null | yrs::Any::Undefined => Robj::from(()),
            yrs::Any::Bool(v) => Robj::from(v),
            yrs::Any::Number(v) => Robj::from(v),
            // R has no native i64; use i32 if it fits, otherwise error
            yrs::Any::BigInt(v) => {
                let v = i32::try_from(v)
                    .map_err(|_| Error::Other(format!("{v} does not fit in i32")))?;
                Robj::from(v)
            }
            yrs::Any::String(v) => Robj::from(v.as_ref()),
            yrs::Any::Buffer(v) => Raw::from_bytes(v.as_ref()).into(),
            yrs::Any::Array(v) => {
                let values: Vec<Robj> = v
                    .iter()
                    .map(|e| e.clone().extendr())
                    .collect::<extendr_api::Result<_>>()?;
                List::from_values(values).into()
            }
            yrs::Any::Map(v) => {
                let (keys, values): (Vec<&str>, Vec<Robj>) = v
                    .iter()
                    .map(|(k, v)| Ok((k.as_str(), v.clone().extendr()?)))
                    .collect::<extendr_api::Result<_>>()?;
                List::from_names_and_values(keys, values).unwrap().into()
            }
        })
    }
}

pub(crate) trait FromExtendr<T>: Sized {
    fn from_extendr(value: T) -> extendr_api::Result<Self>;
}

impl FromExtendr<Robj> for yrs::Any {
    fn from_extendr(robj: Robj) -> extendr_api::Result<Self> {
        if robj.is_null() {
            Ok(yrs::Any::Null)
        } else if let Some(v) = robj.as_bool() {
            Ok(yrs::Any::Bool(v))
        } else if let Some(v) = robj.as_integer() {
            Ok(yrs::Any::BigInt(v as i64))
        } else if let Some(v) = robj.as_real() {
            Ok(yrs::Any::Number(v))
        } else if let Some(v) = robj.as_str() {
            Ok(yrs::Any::String(std::sync::Arc::from(v)))
        } else if robj.is_raw() {
            let raw = Raw::try_from(robj).unwrap();
            Ok(yrs::Any::Buffer(std::sync::Arc::from(raw.as_slice())))
        } else if robj.is_list() {
            let list = robj.as_list().unwrap();
            if robj.names().is_some() {
                let map = std::collections::HashMap::<String, Robj>::try_from(list)
                    .unwrap()
                    .into_iter()
                    .map(|(k, v)| Ok((k, yrs::Any::from_extendr(v)?)))
                    .collect::<extendr_api::Result<_>>()?;
                Ok(yrs::Any::Map(std::sync::Arc::new(map)))
            } else {
                let arr = list
                    .values()
                    .map(yrs::Any::from_extendr)
                    .collect::<extendr_api::Result<Vec<_>>>()?;
                Ok(yrs::Any::Array(std::sync::Arc::from(arr.as_slice())))
            }
        } else {
            Err(Error::Other(format!(
                "Cannot convert {:?} to yrs::Any",
                robj.rtype()
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use super::*;

    #[test]
    fn test_null_extendr() {
        extendr_api::test! {
            assert_eq!(yrs::Any::Null.extendr().unwrap(), r!(NULL));
            assert_eq!(yrs::Any::Undefined.extendr().unwrap(), r!(NULL));
        }
    }

    #[test]
    fn test_bool_extendr() {
        extendr_api::test! {
            assert_eq!(yrs::Any::Bool(true).extendr().unwrap(), r!(true));
            assert_eq!(yrs::Any::Bool(false).extendr().unwrap(), r!(false));
        }
    }

    #[test]
    fn test_number_extendr() {
        extendr_api::test! {
            assert_eq!(yrs::Any::Number(1.5).extendr().unwrap(), r!(1.5));
        }
    }

    #[test]
    fn test_bigint_extendr() {
        extendr_api::test! {
            assert_eq!(yrs::Any::BigInt(42).extendr().unwrap(), r!(42i32));
            assert!(yrs::Any::BigInt(i64::MAX).extendr().is_err());
        }
    }

    #[test]
    fn test_string_extendr() {
        extendr_api::test! {
            let s: Arc<str> = Arc::from("hello");
            assert_eq!(yrs::Any::String(s).extendr().unwrap(), r!("hello"));
        }
    }

    #[test]
    fn test_buffer_extendr() {
        extendr_api::test! {
            let buf: Arc<[u8]> = Arc::from([1u8, 2, 3].as_slice());
            let robj = yrs::Any::Buffer(buf).extendr().unwrap();
            assert!(robj.is_raw());
            assert_eq!(robj.len(), 3);
        }
    }

    #[test]
    fn test_array_extendr() {
        extendr_api::test! {
            let arr: Arc<[yrs::Any]> = Arc::from([yrs::Any::Bool(true), yrs::Any::Number(1.0)].as_slice());
            let robj = yrs::Any::Array(arr).extendr().unwrap();
            assert!(robj.is_list());
            assert_eq!(robj.len(), 2);
        }
    }

    #[test]
    fn test_map_extendr() {
        extendr_api::test! {
            let map: Arc<HashMap<String, yrs::Any>> =
                Arc::new(HashMap::from([("x".to_string(), yrs::Any::Number(1.0))]));
            let robj = yrs::Any::Map(map).extendr().unwrap();
            assert!(robj.is_list());
            assert_eq!(robj.len(), 1);
        }
    }

    #[test]
    fn test_null_from_extendr() {
        extendr_api::test! {
            assert!(matches!(yrs::Any::from_extendr(r!(NULL)).unwrap(), yrs::Any::Null));
        }
    }

    #[test]
    fn test_bool_from_extendr() {
        extendr_api::test! {
            assert!(matches!(yrs::Any::from_extendr(r!(true)).unwrap(), yrs::Any::Bool(true)));
            assert!(matches!(yrs::Any::from_extendr(r!(false)).unwrap(), yrs::Any::Bool(false)));
        }
    }

    #[test]
    fn test_integer_from_extendr() {
        extendr_api::test! {
            assert!(matches!(yrs::Any::from_extendr(r!(42i32)).unwrap(), yrs::Any::BigInt(42)));
        }
    }

    #[test]
    fn test_number_from_extendr() {
        extendr_api::test! {
            assert!(matches!(yrs::Any::from_extendr(r!(1.5)).unwrap(), yrs::Any::Number(v) if v == 1.5));
        }
    }

    #[test]
    fn test_string_from_extendr() {
        extendr_api::test! {
            assert!(matches!(yrs::Any::from_extendr(r!("hello")).unwrap(), yrs::Any::String(ref s) if s.as_ref() == "hello"));
        }
    }

    #[test]
    fn test_buffer_from_extendr() {
        extendr_api::test! {
            let robj: Robj = Raw::from_bytes(&[1, 2, 3]).into();
            assert!(matches!(yrs::Any::from_extendr(robj).unwrap(), yrs::Any::Buffer(ref b) if b.len() == 3));
        }
    }

    #[test]
    fn test_array_from_extendr() {
        extendr_api::test! {
            let robj: Robj = List::from_values([r!(true), r!(1.5)]).into();
            assert!(matches!(yrs::Any::from_extendr(robj).unwrap(), yrs::Any::Array(ref a) if a.len() == 2));
        }
    }

    #[test]
    fn test_map_from_extendr() {
        extendr_api::test! {
            let robj: Robj = List::from_names_and_values(["x"], [r!(1.5)]).unwrap().into();
            assert!(matches!(yrs::Any::from_extendr(robj).unwrap(), yrs::Any::Map(ref m) if m.len() == 1));
        }
    }
}
