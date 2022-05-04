use enum_dispatch::enum_dispatch;
use indexmap::map::Iter;

use pyo3::types::{PyAny, PyDict, PyFrozenSet, PyList, PySet, PyTuple};
use pyo3::{ffi, AsPyPointer};

use super::parse_json::{JsonArray, JsonInput, JsonObject};
use super::Input;

#[enum_dispatch]
pub enum GenericSequence<'data> {
    List(&'data PyList),
    Tuple(&'data PyTuple),
    Set(&'data PySet),
    FrozenSet(&'data PyFrozenSet),
    JsonArray(&'data JsonArray),
}

#[enum_dispatch(GenericSequence)]
pub trait SequenceLenIter<'data> {
    fn length(&self) -> usize;
    fn iter(&self) -> GenericSequenceIter<'data>;
}

impl<'data> SequenceLenIter<'data> for &'data PyList {
    fn length(&self) -> usize {
        self.len()
    }
    fn iter(&self) -> GenericSequenceIter<'data> {
        GenericSequenceIter::List(PyListIterator { sequence: self, index: 0 })
    }
}

impl<'data> SequenceLenIter<'data> for &'data PyTuple {
    fn length(&self) -> usize {
        self.len()
    }
    fn iter(&self) -> GenericSequenceIter<'data> {
        GenericSequenceIter::Tuple(PyTupleIterator {
            sequence: self,
            index: 0,
            length: self.len(),
        })
    }
}

impl<'data> SequenceLenIter<'data> for &'data PySet {
    fn length(&self) -> usize {
        self.len()
    }
    fn iter(&self) -> GenericSequenceIter<'data> {
        GenericSequenceIter::Set(PySetIterator { sequence: self, index: 0 })
    }
}

impl<'data> SequenceLenIter<'data> for &'data PyFrozenSet {
    fn length(&self) -> usize {
        self.len()
    }
    fn iter(&self) -> GenericSequenceIter<'data> {
        GenericSequenceIter::FrozenSet(PyFrozenSetIterator { sequence: self, index: 0 })
    }
}

impl<'data> SequenceLenIter<'data> for &'data JsonArray {
    fn length(&self) -> usize {
        self.len()
    }

    fn iter(&self) -> GenericSequenceIter<'data> {
        GenericSequenceIter::JsonArray(JsonArrayIterator { sequence: self, index: 0 })
    }
}

#[enum_dispatch]
pub enum GenericSequenceIter<'data> {
    List(PyListIterator<'data>),
    Tuple(PyTupleIterator<'data>),
    Set(PySetIterator<'data>),
    FrozenSet(PyFrozenSetIterator<'data>),
    JsonArray(JsonArrayIterator<'data>),
}

#[enum_dispatch(GenericSequenceIter)]
pub trait SequenceNext<'data> {
    fn _next(&mut self) -> Option<&'data dyn Input>;
}

impl<'data> Iterator for GenericSequenceIter<'data> {
    type Item = &'data dyn Input;

    #[inline]
    fn next(&mut self) -> Option<&'data dyn Input> {
        self._next()
    }
}

pub struct PyListIterator<'data> {
    sequence: &'data PyList,
    index: usize,
}

impl<'data> SequenceNext<'data> for PyListIterator<'data> {
    #[inline]
    fn _next(&mut self) -> Option<&'data dyn Input> {
        if self.index < self.sequence.len() {
            let item = unsafe { self.sequence.get_item_unchecked(self.index) };
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }
}

pub struct PyTupleIterator<'data> {
    sequence: &'data PyTuple,
    index: usize,
    length: usize,
}

impl<'data> SequenceNext<'data> for PyTupleIterator<'data> {
    #[inline]
    fn _next(&mut self) -> Option<&'data dyn Input> {
        if self.index < self.length {
            let item = unsafe { self.sequence.get_item_unchecked(self.index) };
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }
}

pub struct PySetIterator<'data> {
    sequence: &'data PySet,
    index: isize,
}

impl<'data> SequenceNext<'data> for PySetIterator<'data> {
    #[inline]
    fn _next(&mut self) -> Option<&'data dyn Input> {
        unsafe {
            let mut key: *mut ffi::PyObject = std::ptr::null_mut();
            let mut hash: ffi::Py_hash_t = 0;
            if ffi::_PySet_NextEntry(self.sequence.as_ptr(), &mut self.index, &mut key, &mut hash) != 0 {
                // _PySet_NextEntry returns borrowed object; for safety must make owned (see #890)
                let item: &PyAny = self.sequence.py().from_owned_ptr(ffi::_Py_NewRef(key));
                Some(item)
            } else {
                None
            }
        }
    }
}

pub struct PyFrozenSetIterator<'data> {
    sequence: &'data PyFrozenSet,
    index: isize,
}

impl<'data> SequenceNext<'data> for PyFrozenSetIterator<'data> {
    #[inline]
    fn _next(&mut self) -> Option<&'data dyn Input> {
        unsafe {
            let mut key: *mut ffi::PyObject = std::ptr::null_mut();
            let mut hash: ffi::Py_hash_t = 0;
            if ffi::_PySet_NextEntry(self.sequence.as_ptr(), &mut self.index, &mut key, &mut hash) != 0 {
                // _PySet_NextEntry returns borrowed object; for safety must make owned (see #890)
                let item: &PyAny = self.sequence.py().from_owned_ptr(ffi::_Py_NewRef(key));
                Some(item)
            } else {
                None
            }
        }
    }
}

pub struct JsonArrayIterator<'data> {
    sequence: &'data JsonArray,
    index: usize,
}

impl<'data> SequenceNext<'data> for JsonArrayIterator<'data> {
    #[inline]
    fn _next(&mut self) -> Option<&'data dyn Input> {
        match self.sequence.get(self.index) {
            Some(item) => {
                self.index += 1;
                Some(item)
            }
            None => None,
        }
    }
}

pub enum GenericMapping<'data> {
    PyDict(&'data PyDict),
    JsonObject(&'data JsonObject),
}

impl<'data> From<&'data PyDict> for GenericMapping<'data> {
    fn from(dict: &'data PyDict) -> Self {
        Self::PyDict(dict)
    }
}

impl<'data> From<&'data JsonObject> for GenericMapping<'data> {
    fn from(dict: &'data JsonObject) -> Self {
        Self::JsonObject(dict)
    }
}

impl<'data> GenericMapping<'data> {
    pub fn len(&self) -> usize {
        match self {
            Self::PyDict(dict) => dict.len(),
            Self::JsonObject(dict) => dict.len(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&'data dyn Input> {
        match self {
            Self::PyDict(dict) => dict.get_item(key).map(|v| v as &dyn Input),
            Self::JsonObject(dict) => dict.get(key).map(|v| v as &dyn Input),
        }
    }

    pub fn iter(&self) -> GenericMappingIter<'data> {
        match self {
            Self::PyDict(dict) => GenericMappingIter::PyDict(PyDictIterator { dict, index: 0 }),
            Self::JsonObject(dict) => GenericMappingIter::JsonObject(JsonObjectIterator { iter: dict.iter() }),
        }
    }
}

#[enum_dispatch]
pub enum GenericMappingIter<'data> {
    PyDict(PyDictIterator<'data>),
    JsonObject(JsonObjectIterator<'data>),
}

/// helper trait implemented by all types in GenericMappingIter which is used when for the shared implementation of
/// `Iterator` for `GenericMappingIter`
#[enum_dispatch(GenericMappingIter)]
pub trait DictNext<'data> {
    fn _next(&mut self) -> Option<(&'data dyn Input, &'data dyn Input)>;
}

impl<'data> Iterator for GenericMappingIter<'data> {
    type Item = (&'data dyn Input, &'data dyn Input);

    #[inline]
    fn next(&mut self) -> Option<(&'data dyn Input, &'data dyn Input)> {
        self._next()
    }
}

pub struct PyDictIterator<'data> {
    dict: &'data PyDict,
    index: isize,
}

impl<'data> DictNext<'data> for PyDictIterator<'data> {
    #[inline]
    fn _next(&mut self) -> Option<(&'data dyn Input, &'data dyn Input)> {
        unsafe {
            let mut key: *mut ffi::PyObject = std::ptr::null_mut();
            let mut value: *mut ffi::PyObject = std::ptr::null_mut();
            if ffi::PyDict_Next(self.dict.as_ptr(), &mut self.index, &mut key, &mut value) != 0 {
                let py = self.dict.py();
                // PyDict_Next returns borrowed values; for safety must make them owned (see #890)
                let key: &PyAny = py.from_owned_ptr(ffi::_Py_NewRef(key));
                let value: &PyAny = py.from_owned_ptr(ffi::_Py_NewRef(value));
                Some((key, value))
            } else {
                None
            }
        }
    }
}

pub struct JsonObjectIterator<'a> {
    iter: Iter<'a, String, JsonInput>,
}

impl<'data> DictNext<'data> for JsonObjectIterator<'data> {
    #[inline]
    fn _next(&mut self) -> Option<(&'data dyn Input, &'data dyn Input)> {
        self.iter.next().map(|(k, v)| (k as &dyn Input, v as &dyn Input))
    }
}
