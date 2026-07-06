use extendr_api::prelude::*;
use yrs::{
    Any as YAny, ArrayPrelim as YArrayPrelim, In as YIn, MapPrelim as YMapPrelim,
    TextPrelim as YTextPrelim,
};

use crate::type_conversion::FromExtendr;
use crate::utils;

#[derive(Clone, Copy)]
pub struct PrelimTypeOptions {
    recursive: bool,
}

#[derive(Clone)]
pub enum PrelimType {
    Map(List, PrelimTypeOptions),
    Array(List, PrelimTypeOptions),
    Text(Strings),
    Any(Robj),
}

impl IntoRobj for PrelimType {
    fn into_robj(self) -> Robj {
        match self {
            Self::Map(list, _) => list.into_robj(),
            Self::Array(list, _) => list.into_robj(),
            Self::Text(str) => str.into_robj(),
            Self::Any(obj) => obj,
        }
    }
}

utils::extendr_struct!(
    #[extendr]
    #[derive(Clone)]
    pub Prelim(PrelimType)
);

impl Prelim {
    fn in_from_text(str: Strings) -> Result<YIn, Error> {
        String::from_extendr(&str)
            .map(YTextPrelim::new)
            .map(Into::into)
    }

    fn in_from_array(list: List, opts: PrelimTypeOptions) -> Result<YIn, Error> {
        let mut out = YArrayPrelim::default();
        if list.is_null() {
            return Ok(out.into());
        }
        out.reserve(list.len());
        for obj in list.as_slice().iter() {
            if opts.recursive {
                out.push(Self::detect(obj.clone(), opts.recursive).to_in()?);
            } else {
                out.push(YAny::from_extendr(obj.clone())?.into());
            }
        }
        Ok(out.into())
    }

    fn in_from_map(map: List, opts: PrelimTypeOptions) -> Result<YIn, Error> {
        let mut out = YMapPrelim::default();
        if map.is_null() {
            return Ok(out.into());
        }
        out.reserve(map.len());
        for (name, obj) in map.iter() {
            if opts.recursive {
                out.insert(
                    name.into(),
                    Self::detect(obj.clone(), opts.recursive).to_in()?,
                );
            } else {
                out.insert(name.into(), YAny::from_extendr(obj.clone())?.into());
            }
        }
        Ok(out.into())
    }

    fn in_from_any(obj: Robj) -> Result<YIn, Error> {
        YAny::from_extendr(obj).map(Into::into)
    }

    pub fn to_in(&self) -> Result<YIn, Error> {
        match self.as_ref() {
            PrelimType::Any(obj) => Self::in_from_any(obj.clone()),
            PrelimType::Text(str) => Self::in_from_text(str.clone()),
            PrelimType::Map(list, opts) => Self::in_from_map(list.clone(), *opts),
            PrelimType::Array(list, opts) => Self::in_from_array(list.clone(), *opts),
        }
    }
}

#[extendr]
impl Prelim {
    fn detect(obj: Robj, #[extendr(default = "FALSE")] recursive: bool) -> Self {
        if let Ok(prelim) = TryInto::<&Prelim>::try_into(&obj) {
            prelim.as_ref().clone().into()
        } else if let Ok(str) = TryInto::<Strings>::try_into(obj.clone()) {
            Self::text(str)
        } else {
            if let Some(list) = obj.as_list() {
                if list.has_names() {
                    Self::map(list, recursive)
                } else {
                    Self::array(list, recursive)
                }
            } else {
                Self::any(obj)
            }
        }
    }

    fn text(#[extendr(default = r#""""#)] obj: Strings) -> Self {
        PrelimType::Text(obj).into()
    }

    fn array(
        #[extendr(default = "NULL")] obj: List,
        #[extendr(default = "FALSE")] recursive: bool,
    ) -> Self {
        PrelimType::Array(obj, PrelimTypeOptions { recursive }).into()
    }

    fn map(
        #[extendr(default = "NULL")] obj: List,
        #[extendr(default = "FALSE")] recursive: bool,
    ) -> Self {
        PrelimType::Map(obj, PrelimTypeOptions { recursive }).into()
    }

    fn any(obj: Robj) -> Self {
        PrelimType::Any(obj).into()
    }

    fn inner(&self) -> Robj {
        self.clone().into_robj()
    }

    fn __yr_print__(&self) -> Result<String, Error> {
        Ok(format!("{:?}", self.to_in()?))
    }

    pub fn is_text(&self) -> bool {
        matches!(self.as_ref(), PrelimType::Text(_))
    }

    pub fn is_array(&self) -> bool {
        matches!(self.as_ref(), PrelimType::Array(_, _))
    }

    pub fn is_map(&self) -> bool {
        matches!(self.as_ref(), PrelimType::Map(_, _))
    }

    pub fn is_any(&self) -> bool {
        matches!(self.as_ref(), PrelimType::Any(_))
    }

    pub fn is_recursive(&self) -> bool {
        match self.as_ref() {
            PrelimType::Any(_) => false,
            PrelimType::Text(_) => false,
            PrelimType::Map(_, opts) => opts.recursive,
            PrelimType::Array(_, opts) => opts.recursive,
        }
    }
}

extendr_module! {
    mod prelim;
    impl Prelim;
}
