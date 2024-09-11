// #![cfg(feature = "jiff")]
//! Conversions to and from [jiff](https://docs.rs/jiff/)'s `Date`
//!
//! Other types are TODO

use crate::conversion::IntoPyObject;
use crate::types::any::PyAnyMethods;
use crate::types::{
    PyDate, PyDateAccess, PyDateTime, PyDelta, PyDeltaAccess, PyNone, PyTimeAccess, PyTzInfo,
    PyTzInfoAccess,
};
use crate::{Bound, FromPyObject, IntoPy, PyAny, PyErr, PyObject, PyResult, Python, ToPyObject};
use eyre::ContextCompat;

use crate::exceptions::{PyTypeError, PyValueError};
use jiff::civil::{Date, DateTime};
use jiff::tz::Offset;
use jiff::{Timestamp, Zoned};

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
        #[cfg(not(Py_LIMITED_API))]
        {
            PyDate::new(
                py,
                self.year().try_into()?,
                self.month().try_into()?,
                self.day().try_into()?,
            )
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
        let date = *self;
        (*self).into_pyobject(py)
    }
}

impl FromPyObject<'_> for Date {
    fn extract_bound(ob: &Bound<'_, PyAny>) -> PyResult<Date> {
        #[cfg(not(Py_LIMITED_API))]
        {
            let date = ob.downcast::<PyDate>()?;
            Date::new(
                date.get_year().try_into()?,
                date.get_month().try_into()?,
                date.get_day().try_into()?,
            )
            .map_err(|_| PyErr::new::<PyAny, _>("invalid or out-of-range date"))
        }
        #[cfg(Py_LIMITED_API)]
        {
            check_type(ob, &DatetimeTypes::get(ob.py()).date, "PyDate")?;
            py_date_to_naive_date(ob)
        }
    }
}

impl FromPyObject<'_> for DateTime {
    fn extract_bound(ob: &Bound<'_, PyAny>) -> PyResult<DateTime> {
        #[cfg(not(Py_LIMITED_API))]
        {
            let datetime = ob.downcast::<PyDateTime>()?;
            DateTime::new(
                datetime.get_year().try_into()?,
                datetime.get_month().try_into()?,
                datetime.get_day().try_into()?,
                datetime.get_hour().try_into()?,
                datetime.get_minute().try_into()?,
                datetime.get_second().try_into()?,
                datetime.get_microsecond().try_into()?, // TODO convert microsecond to nanosecond
            )
            .map_err(|_| PyErr::new::<PyAny, _>("invalid or out-of-range date"))
        }
        #[cfg(Py_LIMITED_API)]
        {
            check_type(ob, &DatetimeTypes::get(ob.py()).date, "PyDate")?;
            py_date_to_naive_date(ob)
        }
    }
}

impl FromPyObject<'_> for Offset {
    fn extract_bound(ob: &Bound<'_, PyAny>) -> PyResult<Offset> {
        #[cfg(not(Py_LIMITED_API))]
        let ob = ob.downcast::<PyTzInfo>()?;
        #[cfg(Py_LIMITED_API)]
        check_type(ob, &DatetimeTypes::get(ob.py()).tzinfo, "PyTzInfo")?;

        let py_timedelta = ob.call_method1("utcoffset", (PyNone::get(ob.py()),))?;
        if py_timedelta.is_none() {
            return Err(PyTypeError::new_err(format!(
                "{:?} is not a fixed offset timezone",
                ob
            )));
        }
        Offset::from_seconds(py_timedelta.downcast::<PyDelta>()?.get_seconds())
            .map_err(|_| PyValueError::new_err("fixed offset out of bounds"))
    }
}

impl FromPyObject<'_> for Zoned {
    fn extract_bound(ob: &Bound<'_, PyAny>) -> PyResult<Zoned> {
        #[cfg(not(Py_LIMITED_API))]
        {
            let datetime = ob.downcast::<PyDateTime>()?;
            let timezone = datetime
                .get_tzinfo()
                .ok_or_else(|| PyErr::new::<PyAny, _>("missing timezone"))?
                .extract::<Offset>()?
                .to_time_zone();
            datetime
                .extract::<DateTime>()?
                .to_zoned(timezone)
                .map_err(|_| PyErr::new::<PyAny, _>("invalid or out-of-range date"))
        }
        #[cfg(Py_LIMITED_API)]
        {
            check_type(ob, &DatetimeTypes::get(ob.py()).date, "PyDate")?;
            py_date_to_naive_date(ob)
        }
    }
}

impl FromPyObject<'_> for Timestamp {
    fn extract_bound(ob: &Bound<'_, PyAny>) -> PyResult<Timestamp> {
        #[cfg(not(Py_LIMITED_API))]
        {
            ob.extract().map(Zoned::timestamp)
        }
        #[cfg(Py_LIMITED_API)]
        {
            check_type(ob, &DatetimeTypes::get(ob.py()).date, "PyDate")?;
            py_date_to_naive_date(ob)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn test_pyo3_date_topyobject() {
        let eq_ymd = |name: &'static str, year, month, day| {
            Python::with_gil(|py| {
                let date = Date::new(year, month, day).unwrap().to_object(py);
                let py_date = PyDate::new(py, year as i32, month as u8, day as u8).unwrap();
                assert_eq!(
                    date.bind(py).compare(&py_date).unwrap(),
                    Ordering::Equal,
                    "{}: {} != {}",
                    name,
                    date,
                    py_date
                );
            })
        };

        eq_ymd("past date", 2012, 2, 29);
        eq_ymd("min date", 1, 1, 1);
        eq_ymd("future date", 3000, 6, 5);
        eq_ymd("max date", 9999, 12, 31);
    }
}
