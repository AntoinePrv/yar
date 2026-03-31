use extendr_api::prelude::*;

impl<T, Y> lifetime::Owner<Y> for ExternalPtr<T>
where
    T: AsRef<lifetime::CheckedRef<Y>> + From<lifetime::CheckedRef<Y>>,
    T: IntoRobj + 'static,
{
    fn wrap(r: lifetime::CheckedRef<Y>) -> Self {
        let t: T = r.into();
        // Converting to Robj first as the converter will set the class symbol attribute,
        // otherwise it will only be seen as an `externalptr` from R.
        let robj = t.into_robj();
        // PANICS: Robj was just created with the proper type
        TryInto::<ExternalPtr<T>>::try_into(robj).unwrap()
    }

    fn inner(&self) -> &lifetime::CheckedRef<Y> {
        self.as_ref().as_ref()
    }
}

pub trait ExtendrRef<Y> {
    type Error;
    type Owner: lifetime::Owner<Y>;

    fn guard<'a>(r: &'a Y) -> lifetime::Guard<'a, Y, Self::Owner>;

    fn try_map<R>(&self, f: impl FnOnce(&Y) -> R) -> Result<R, Self::Error>;
}

impl<T, Y> ExtendrRef<Y> for T
where
    T: AsRef<lifetime::CheckedRef<Y>>,
    ExternalPtr<T>: lifetime::Owner<Y>,
{
    type Error = extendr_api::Error;
    type Owner = ExternalPtr<T>;

    fn guard<'a>(r: &'a Y) -> lifetime::Guard<'a, Y, Self::Owner> {
        lifetime::CheckedRef::new_guarded(r)
    }

    fn try_map<R>(&self, f: impl FnOnce(&Y) -> R) -> Result<R, Error> {
        self.as_ref().map(f).ok_or_else(|| {
            Error::Other(
                concat!(
                    "Object is invalid.",
                    " This happened because you tried to capture an parameter ",
                    " in callback (observe, with_transaction) and use it afterwards."
                )
                .into(),
            )
        })
    }
}

pub(crate) mod lifetime {

    use std::{cell::Cell, marker::PhantomData, ptr::NonNull};

    /// A reference-counted container that can hold a [`CheckedRef`].
    ///
    /// [`Owner`] abstracts over the reference-counting mechanism used to share
    /// a [`CheckedRef`] between a [`Guard`] and its callers. For example,
    /// an R [`Robj`][extendr_api::Robj] (which is internally reference-counted by R's GC)
    /// implements [`Owner`], while tests use [`Rc<CheckedRef<T>>`][std::rc::Rc].
    ///
    /// # Safety contract
    ///
    /// [`wrap`][Self::wrap] and [`inner`][Self::inner] must be inverses:
    /// [`inner`][Self::inner] **must** return a reference to the exact same [`CheckedRef`]
    /// that was passed to [`wrap`][Self::wrap]. This invariant is critical because
    /// [`Guard::drop`] calls `inner().clear()` to invalidate the pointer — if
    /// [`inner`][Self::inner] returns a different [`CheckedRef`], the real one is left
    /// dangling.
    ///
    /// This contract is verified by a `debug_assert` in [`CheckedRef::new_guarded`].
    pub(crate) trait Owner<T> {
        /// Store a [`CheckedRef`] in a new reference-counted container.
        fn wrap(r: CheckedRef<T>) -> Self;

        /// Retrieve the [`CheckedRef`] previously stored by [`wrap`][Self::wrap].
        ///
        /// Must return a reference to the same [`CheckedRef`] instance, not a copy or
        /// a different one.
        fn inner(&self) -> &CheckedRef<T>;
    }

    /// Lifetime erasure utility that converts a compile-time lifetime into a runtime check.
    ///
    /// A `CheckedRef<T>` stores a raw pointer to `T` without carrying the original lifetime.
    /// A [`Guard`] ties the pointer's validity to the original lifetime `'a`: when the guard
    /// drops (at the end of `'a` at the latest), the pointer is cleared.
    ///
    /// Access is only possible through [`map`](CheckedRef::map), whose higher-rank trait bound
    /// (`impl FnOnce(&T) -> R`) prevents the reference from escaping the closure.
    pub struct CheckedRef<T>(Cell<Option<NonNull<T>>>);

    impl<T> CheckedRef<T> {
        unsafe fn from_ref(r: &T) -> Self {
            Self(Some(NonNull::new_unchecked(r as *const T as *mut T)).into())
        }

        pub fn new_guarded<'a, O: Owner<T>>(r: &'a T) -> Guard<'a, T, O> {
            // SAFETY: The raw pointer is valid as long as 'a. The Guard is tied to 'a
            // and clears the pointer on drop. Access is only through `map()`, whose
            // HRTB (Higher-Rank Trait Bounds) prevents the reference from escaping the closure.
            unsafe {
                let reference: O = Owner::wrap(Self::from_ref(r));
                debug_assert!(
                    std::ptr::eq(&reference.inner().0, &reference.inner().0),
                    "Owner::inner() must return a stable reference (same address on repeated calls)"
                );
                debug_assert!(
                    reference.inner().0.get().is_some(),
                    "Owner::inner() must return the CheckedRef that was passed to Owner::wrap()"
                );
                Guard::<'a> {
                    reference,
                    _phantom: PhantomData,
                }
            }
        }

        pub fn map<R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
            // SAFETY: The pointer is valid as long as the option contains a value.
            // The HRTB on F prevents the reference from escaping the closure.
            self.0.get().map(|ptr| f(unsafe { ptr.as_ref() }))
        }

        pub fn clear(&self) {
            self.0.set(None)
        }
    }

    #[must_use]
    pub struct Guard<'a, T, O: Owner<T>> {
        reference: O,
        _phantom: PhantomData<&'a T>,
    }

    impl<'a, T, O: Owner<T>> Drop for Guard<'a, T, O> {
        fn drop(&mut self) {
            self.reference.inner().clear();
        }
    }

    impl<'a, T, O: Owner<T>> Guard<'a, T, O> {
        pub fn get(&self) -> &O {
            &self.reference
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::rc::Rc;

        impl<T> Owner<T> for std::rc::Rc<CheckedRef<T>> {
            fn wrap(r: CheckedRef<T>) -> Self {
                Self::from(r)
            }

            fn inner(&self) -> &CheckedRef<T> {
                self.as_ref()
            }
        }

        #[test]
        fn guard_drop_invalidates_checked_ref() {
            let val = 42i32;
            let guard = CheckedRef::<i32>::new_guarded::<Rc<_>>(&val);
            let owner = guard.get().clone();
            assert_eq!(owner.map(|r| *r), Some(42));
            drop(guard);
            assert_eq!(owner.map(|r| *r), None);
        }
    }
}
