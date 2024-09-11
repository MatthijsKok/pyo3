#![cfg(feature = "jiff")]
//! Conversions to and from [jiff](https://docs.rs/jiff/)'s `Date`
//!
//! Other types are TODO

use crate::conversion::IntoPyObject;
use crate::exceptions::{PyTypeError, PyUserWarning, PyValueError};
#[cfg(Py_LIMITED_API)]
use crate::sync::GILOnceCell;
use crate::types::any::PyAnyMethods;
#[cfg(not(Py_LIMITED_API))]
use crate::types::datetime::timezone_from_offset;
use crate::types::PyNone;
#[cfg(not(Py_LIMITED_API))]
use crate::types::{
    timezone_utc, PyDate, PyDateAccess, PyDateTime, PyDelta, PyDeltaAccess, PyTime, PyTimeAccess,
    PyTzInfo, PyTzInfoAccess,
};
use crate::{
    ffi, Bound, FromPyObject, IntoPy, PyAny, PyErr, PyObject, PyResult, Python, ToPyObject,
};
#[cfg(Py_LIMITED_API)]
use crate::{intern, DowncastError};

use jiff::civil::Date;

impl ToPyObject for Date {
    #[inline]
    fn to_object(&self, py: Python<'_>) -> PyObject {
        self.into_pyobject(py).unwrap().into_any().unbind()
    }
}

impl IntoPy<PyObject> for Date {
    #[inline]
    fn into_py(self, py: Python<'_>) -> PyObject {
        self.into_pyobject(py).unwrap().into_any().unbind()
    }
}

impl<'py> IntoPyObject<'py> for Date {
    #[cfg(Py_LIMITED_API)]
    type Target = PyAny;
    #[cfg(not(Py_LIMITED_API))]
    type Target = PyDate;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        let DateArgs { year, month, day } = (&self).into();
        #[cfg(not(Py_LIMITED_API))]
        {
            PyDate::new(py, year, month, day)
        }
        #[cfg(Py_LIMITED_API)]
        {
            todo!()
            // DatetimeTypes::try_get(py).and_then(|dt| dt.date.bind(py).call1((year, month, day)))
        }
    }
}

impl<'py> IntoPyObject<'py> for &Date {
    #[cfg(Py_LIMITED_API)]
    type Target = PyAny;
    #[cfg(not(Py_LIMITED_API))]
    type Target = PyDate;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    #[inline]
    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        (*self).into_pyobject(py)
    }
}

impl FromPyObject<'_> for Date {
    fn extract_bound(ob: &Bound<'_, PyAny>) -> PyResult<Date> {
        #[cfg(not(Py_LIMITED_API))]
        {
            let date = ob.downcast::<PyDate>()?;
            py_date_to_naive_date(date)
        }
        #[cfg(Py_LIMITED_API)]
        {
            check_type(ob, &DatetimeTypes::get(ob.py()).date, "PyDate")?;
            py_date_to_naive_date(ob)
        }
    }
}

// utils below ... ?

struct DateArgs {
    year: i32,
    month: i8,
    day: i8,
}

impl From<&Date> for DateArgs {
    fn from(value: &Date) -> Self {
        Self {
            year: value.year() as i32,
            month: value.month() as i8,
            day: value.day() as i8,
        }
    }
}

#[cfg(not(Py_LIMITED_API))]
fn py_date_to_civil_date(py_date: &impl PyDateAccess) -> PyResult<Date> {
    use std::i16;

    Date::new(
        py_date.get_year().try_into().unwrap_or(i16::MAX),
        py_date.get_month().try_into()?,
        py_date.get_day().try_into()?,
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))
}

#[cfg(Py_LIMITED_API)]
fn py_date_to_civil_date(py_date: &Bound<'_, PyAny>) -> PyResult<Date> {
    Date::new(
        py_date.getattr(intern!(py_date.py(), "year"))?.extract()?,
        py_date.getattr(intern!(py_date.py(), "month"))?.extract()?,
        py_date.getattr(intern!(py_date.py(), "day"))?.extract()?,
    )
    .ok_or_else(|| PyValueError::new_err("invalid or out-of-range date"))
}

#[cfg(Py_LIMITED_API)]
fn check_type(value: &Bound<'_, PyAny>, t: &PyObject, type_name: &'static str) -> PyResult<()> {
    if !value.is_instance(t.bind(value.py()))? {
        return Err(DowncastError::new(value, type_name).into());
    }
    Ok(())
}
