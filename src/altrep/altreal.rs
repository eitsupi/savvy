use std::{
    ffi::CString,
    os::raw::{c_int, c_void},
};

use savvy_ffi::{
    altrep::{
        R_altrep_data2, R_make_altreal_class, R_set_altreal_Elt_method, R_set_altrep_Coerce_method,
        R_set_altrep_Duplicate_method, R_set_altrep_Inspect_method, R_set_altrep_Length_method,
        R_set_altrep_data2, R_set_altvec_Dataptr_method, R_set_altvec_Dataptr_or_null_method,
    },
    R_NaReal, R_NilValue, R_xlen_t, Rboolean, Rboolean_FALSE, Rboolean_TRUE, Rf_coerceVector,
    Rf_duplicate, Rf_protect, Rf_unprotect, Rf_xlength, REAL, REALSXP, REAL_ELT, SEXP, SEXPTYPE,
};

use crate::{IntoExtPtrSexp, RealSexp};

pub trait AltReal: Sized + IntoExtPtrSexp {
    /// Class name to identify the ALTREP class.
    const CLASS_NAME: &'static str;

    /// Package name to identify the ALTREP class.
    const PACKAGE_NAME: &'static str;

    /// Return the length of the data.
    fn length(&mut self) -> usize;

    /// Returns the value of `i`-th element. Note that, it seems R handles the
    /// out-of-bound check, so you don't need to implement it here.
    fn elt(&mut self, i: usize) -> f64;

    /// Copies the specified range of the data into a new memory. This is used
    /// when the ALTREP needs to be materialized.
    ///
    /// For example, you can use `copy_from_slice()` for more efficient copying
    /// of the values.
    fn copy_to(&mut self, new: &mut [f64], offset: usize) {
        // TODO: return error
        if offset + new.len() > self.length() {
            return;
        }

        for (i, v) in new.iter_mut().enumerate() {
            *v = self.elt(i + offset);
        }
    }

    /// What gets printed when `.Internal(inspect(x))` is used.
    fn inspect(&mut self) {
        crate::io::r_print(&format!("({})", Self::CLASS_NAME), false);
    }

    /// Converts the struct into an ALTREP object
    fn into_altrep(self) -> crate::Result<crate::Sexp> {
        super::create_altrep_instance(self, Self::CLASS_NAME).map(crate::Sexp)
    }

    /// Extracts the reference (`&T`) of the underlying data
    fn try_from_altrep_ref(x: &RealSexp) -> crate::Result<&Self> {
        super::assert_altrep_class(x.0, Self::CLASS_NAME)?;
        super::extract_ref_from_altrep(&x.0)
    }

    /// Extracts the mutable reference (`&mut T`) of the underlying data
    fn try_from_altrep_mut(x: &mut RealSexp) -> crate::Result<&mut Self> {
        super::assert_altrep_class(x.0, Self::CLASS_NAME)?;
        super::extract_mut_from_altrep(&mut x.0)
    }

    /// Takes the underlying data. After this operation, the external pointer is
    /// replaced with a null pointer.
    fn try_from_altrep(x: RealSexp) -> crate::Result<Self> {
        super::assert_altrep_class(x.0, Self::CLASS_NAME)?;
        super::extract_from_altrep(x.0)
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn register_altreal_class<T: AltReal>(
    dll_info: *mut crate::ffi::DllInfo,
) -> crate::error::Result<()> {
    let class_name = CString::new(T::CLASS_NAME).unwrap_or_default();
    let package_name = CString::new(T::PACKAGE_NAME).unwrap_or_default();
    let class_t =
        unsafe { R_make_altreal_class(class_name.as_ptr(), package_name.as_ptr(), dll_info) };

    #[allow(clippy::mut_from_ref)]
    #[inline]
    fn get_materialized_sexp<T: AltReal>(x: &mut SEXP, allow_allocate: bool) -> Option<SEXP> {
        let data = unsafe { R_altrep_data2(*x) };
        if unsafe { data != R_NilValue } {
            return Some(data);
        }

        // If the allocation is unpreferable, give up here.
        if !allow_allocate {
            return None;
        }

        let self_: &mut T = match super::extract_mut_from_altrep(x) {
            Ok(self_) => self_,
            Err(_) => return None,
        };

        let len = self_.length();

        let new = crate::alloc_vector(REALSXP, len).unwrap();
        unsafe { Rf_protect(new) };

        self_.copy_to(unsafe { std::slice::from_raw_parts_mut(REAL(new), len) }, 0);

        // Cache the materialized data in data2.
        unsafe { R_set_altrep_data2(*x, new) };

        // new doesn't need protection because it's used as long as this ALTREP exists.
        unsafe { Rf_unprotect(1) };

        Some(new)
    }

    unsafe extern "C" fn altrep_duplicate<T: AltReal>(mut x: SEXP, _deep_copy: Rboolean) -> SEXP {
        let materialized = get_materialized_sexp::<T>(&mut x, true).expect("Must have result");
        unsafe { Rf_duplicate(materialized) }
    }

    unsafe extern "C" fn altrep_coerce<T: AltReal>(mut x: SEXP, sexp_type: SEXPTYPE) -> SEXP {
        let materialized = get_materialized_sexp::<T>(&mut x, true).expect("Must have result");
        unsafe { Rf_coerceVector(materialized, sexp_type) }
    }

    fn altvec_dataptr_inner<T: AltReal>(mut x: SEXP, allow_allocate: bool) -> *mut c_void {
        match get_materialized_sexp::<T>(&mut x, allow_allocate) {
            Some(materialized) => unsafe { REAL(materialized) as _ },
            // Returning C NULL (not R NULL!) is the convention
            None => std::ptr::null_mut(),
        }
    }

    unsafe extern "C" fn altvec_dataptr<T: AltReal>(x: SEXP, _writable: Rboolean) -> *mut c_void {
        altvec_dataptr_inner::<T>(x, true)
    }

    unsafe extern "C" fn altvec_dataptr_or_null<T: AltReal>(x: SEXP) -> *const c_void {
        altvec_dataptr_inner::<T>(x, false)
    }

    unsafe extern "C" fn altrep_length<T: AltReal>(mut x: SEXP) -> R_xlen_t {
        if let Some(materialized) = get_materialized_sexp::<T>(&mut x, false) {
            unsafe { Rf_xlength(materialized) }
        } else {
            match super::extract_mut_from_altrep::<T>(&mut x) {
                Ok(self_) => self_.length() as _,
                Err(_) => 0,
            }
        }
    }

    unsafe extern "C" fn altrep_inspect<T: AltReal>(
        mut x: SEXP,
        _: c_int,
        _: c_int,
        _: c_int,
        _: Option<unsafe extern "C" fn(SEXP, c_int, c_int, c_int)>,
    ) -> Rboolean {
        match super::extract_mut_from_altrep::<T>(&mut x) {
            Ok(self_) => {
                self_.inspect();
                Rboolean_TRUE
            }
            Err(_) => Rboolean_FALSE,
        }
    }

    unsafe extern "C" fn altreal_elt<T: AltReal>(mut x: SEXP, i: R_xlen_t) -> f64 {
        if let Some(materialized) = get_materialized_sexp::<T>(&mut x, false) {
            unsafe { REAL_ELT(materialized, i) }
        } else {
            match super::extract_mut_from_altrep::<T>(&mut x) {
                Ok(self_) => self_.elt(i as _) as _,
                Err(_) => unsafe { R_NaReal },
            }
        }
    }

    unsafe {
        R_set_altrep_Length_method(class_t, Some(altrep_length::<T>));
        R_set_altrep_Inspect_method(class_t, Some(altrep_inspect::<T>));
        R_set_altrep_Duplicate_method(class_t, Some(altrep_duplicate::<T>));
        R_set_altrep_Coerce_method(class_t, Some(altrep_coerce::<T>));
        R_set_altvec_Dataptr_method(class_t, Some(altvec_dataptr::<T>));
        R_set_altvec_Dataptr_or_null_method(class_t, Some(altvec_dataptr_or_null::<T>));
        R_set_altreal_Elt_method(class_t, Some(altreal_elt::<T>));
    }

    super::register_altrep_class(T::CLASS_NAME, class_t)?;
    Ok(())
}
