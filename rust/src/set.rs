use fst::automaton::Levenshtein;
use fst::{IntoStreamer, Set as FstSet, SetBuilder as FstSetBuilder, Streamer};
use memmap2::Mmap;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use regex_automata::DenseDFA;
use std::fs::File;
use std::io::BufWriter;
use std::sync::Arc;

#[derive(Clone)]
pub enum SetData {
    Vec(Arc<Vec<u8>>),
    Mmap(Arc<Mmap>),
}

impl AsRef<[u8]> for SetData {
    fn as_ref(&self) -> &[u8] {
        match self {
            SetData::Vec(v) => v,
            SetData::Mmap(m) => m,
        }
    }
}

#[pyclass]
#[derive(Clone)]
pub struct Set {
    pub inner: FstSet<SetData>,
}

#[pymethods]
impl Set {
    #[new]
    fn new(path_or_bytes: &PyAny) -> PyResult<Self> {
        if let Ok(path) = path_or_bytes.extract::<String>() {
            let file = File::open(path)?;
            let mmap = unsafe { Mmap::map(&file)? };
            let set = FstSet::new(SetData::Mmap(Arc::new(mmap)))
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
            Ok(Set { inner: set })
        } else if let Ok(bytes) = path_or_bytes.extract::<&[u8]>() {
            let set = FstSet::new(SetData::Vec(Arc::new(bytes.to_vec())))
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
            Ok(Set { inner: set })
        } else {
            Err(PyTypeError::new_err(
                "Argument must be a path (str) or bytes",
            ))
        }
    }

    fn __contains__(&self, key: &str) -> bool {
        self.inner.contains(key)
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __iter__(&self) -> SetStream {
        let stream = self.inner.stream();
        let stream = unsafe {
            std::mem::transmute::<fst::set::Stream<'_>, fst::set::Stream<'static>>(stream)
        };
        SetStream {
            _set: self.inner.clone(),
            stream,
        }
    }

    fn search_re(&self, regex: &str) -> PyResult<SetRegexStream> {
        let dfa = regex_automata::dense::Builder::new()
            .anchored(true)
            .build(regex)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let stream = self.inner.search(&dfa).into_stream();
        let stream = unsafe {
            std::mem::transmute::<
                fst::set::Stream<'_, &DenseDFA<Vec<usize>, usize>>,
                fst::set::Stream<'static, &'static DenseDFA<Vec<usize>, usize>>,
            >(stream)
        };
        Ok(SetRegexStream {
            _set: self.inner.clone(),
            _dfa: dfa,
            stream,
        })
    }

    fn search_lev(&self, key: &str, max_dist: u32) -> PyResult<SetLevStream> {
        let lev =
            Levenshtein::new(key, max_dist).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let stream = self.inner.search(&lev).into_stream();
        let stream = unsafe {
            std::mem::transmute::<
                fst::set::Stream<'_, &Levenshtein>,
                fst::set::Stream<'static, &'static Levenshtein>,
            >(stream)
        };
        Ok(SetLevStream {
            _set: self.inner.clone(),
            _lev: lev,
            stream,
        })
    }

    fn is_disjoint(&self, other: &Set) -> bool {
        self.inner.is_disjoint(&other.inner)
    }

    fn is_subset(&self, other: &Set) -> bool {
        self.inner.is_subset(&other.inner)
    }

    fn is_superset(&self, other: &Set) -> bool {
        self.inner.is_superset(&other.inner)
    }

    fn union(&self, other: &Set) -> SetUnion {
        let sets = vec![self.clone(), other.clone()];
        let op = self.inner.op().add(&self.inner).add(&other.inner).union();
        let stream =
            unsafe { std::mem::transmute::<fst::set::Union<'_>, fst::set::Union<'static>>(op) };
        SetUnion {
            _sets: sets,
            stream,
        }
    }

    fn intersection(&self, other: &Set) -> SetIntersection {
        let sets = vec![self.clone(), other.clone()];
        let op = self
            .inner
            .op()
            .add(&self.inner)
            .add(&other.inner)
            .intersection();
        let stream = unsafe {
            std::mem::transmute::<fst::set::Intersection<'_>, fst::set::Intersection<'static>>(op)
        };
        SetIntersection {
            _sets: sets,
            stream,
        }
    }

    fn difference(&self, other: &Set) -> SetDifference {
        let sets = vec![self.clone(), other.clone()];
        let op = self
            .inner
            .op()
            .add(&self.inner)
            .add(&other.inner)
            .difference();
        let stream = unsafe {
            std::mem::transmute::<fst::set::Difference<'_>, fst::set::Difference<'static>>(op)
        };
        SetDifference {
            _sets: sets,
            stream,
        }
    }

    fn symmetric_difference(&self, other: &Set) -> SetSymmetricDifference {
        let sets = vec![self.clone(), other.clone()];
        let op = self
            .inner
            .op()
            .add(&self.inner)
            .add(&other.inner)
            .symmetric_difference();
        let stream = unsafe {
            std::mem::transmute::<
                fst::set::SymmetricDifference<'_>,
                fst::set::SymmetricDifference<'static>,
            >(op)
        };
        SetSymmetricDifference {
            _sets: sets,
            stream,
        }
    }
}

#[pyclass(unsendable)]
pub struct SetStream {
    _set: FstSet<SetData>,
    stream: fst::set::Stream<'static>,
}

#[pymethods]
impl SetStream {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
    fn __next__(mut slf: PyRefMut<Self>) -> Option<String> {
        let bytes = slf.stream.next()?;
        Some(String::from_utf8_lossy(bytes).into_owned())
    }
}

#[pyclass(unsendable)]
pub struct SetRegexStream {
    _set: FstSet<SetData>,
    _dfa: DenseDFA<Vec<usize>, usize>,
    stream: fst::set::Stream<'static, &'static DenseDFA<Vec<usize>, usize>>,
}

#[pymethods]
impl SetRegexStream {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
    fn __next__(mut slf: PyRefMut<Self>) -> Option<String> {
        let bytes = slf.stream.next()?;
        Some(String::from_utf8_lossy(bytes).into_owned())
    }
}

#[pyclass(unsendable)]
pub struct SetLevStream {
    _set: FstSet<SetData>,
    _lev: Levenshtein,
    stream: fst::set::Stream<'static, &'static Levenshtein>,
}

#[pymethods]
impl SetLevStream {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
    fn __next__(mut slf: PyRefMut<Self>) -> Option<String> {
        let bytes = slf.stream.next()?;
        Some(String::from_utf8_lossy(bytes).into_owned())
    }
}

#[pyclass(unsendable)]
pub struct SetUnion {
    _sets: Vec<Set>,
    stream: fst::set::Union<'static>,
}

#[pymethods]
impl SetUnion {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
    fn __next__(mut slf: PyRefMut<Self>) -> Option<String> {
        let bytes = slf.stream.next()?;
        Some(String::from_utf8_lossy(bytes).into_owned())
    }
}

#[pyclass(unsendable)]
pub struct SetIntersection {
    _sets: Vec<Set>,
    stream: fst::set::Intersection<'static>,
}

#[pymethods]
impl SetIntersection {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
    fn __next__(mut slf: PyRefMut<Self>) -> Option<String> {
        let bytes = slf.stream.next()?;
        Some(String::from_utf8_lossy(bytes).into_owned())
    }
}

#[pyclass(unsendable)]
pub struct SetDifference {
    _sets: Vec<Set>,
    stream: fst::set::Difference<'static>,
}

#[pymethods]
impl SetDifference {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
    fn __next__(mut slf: PyRefMut<Self>) -> Option<String> {
        let bytes = slf.stream.next()?;
        Some(String::from_utf8_lossy(bytes).into_owned())
    }
}

#[pyclass(unsendable)]
pub struct SetSymmetricDifference {
    _sets: Vec<Set>,
    stream: fst::set::SymmetricDifference<'static>,
}

#[pymethods]
impl SetSymmetricDifference {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
    fn __next__(mut slf: PyRefMut<Self>) -> Option<String> {
        let bytes = slf.stream.next()?;
        Some(String::from_utf8_lossy(bytes).into_owned())
    }
}

enum BuilderInner {
    Memory(FstSetBuilder<Vec<u8>>),
    File(FstSetBuilder<BufWriter<File>>),
}

#[pyclass]
pub struct SetBuilder {
    inner: Option<BuilderInner>,
}

#[pymethods]
impl SetBuilder {
    #[new]
    fn new(path: Option<String>) -> PyResult<Self> {
        let inner = if let Some(p) = path {
            let file = File::create(p)?;
            let wtr = BufWriter::new(file);
            let builder =
                FstSetBuilder::new(wtr).map_err(|e| PyValueError::new_err(e.to_string()))?;
            BuilderInner::File(builder)
        } else {
            let builder = FstSetBuilder::memory();
            BuilderInner::Memory(builder)
        };
        Ok(SetBuilder { inner: Some(inner) })
    }

    fn insert(&mut self, key: &str) -> PyResult<()> {
        match self.inner.as_mut() {
            Some(BuilderInner::Memory(b)) => b
                .insert(key)
                .map_err(|e| PyValueError::new_err(e.to_string())),
            Some(BuilderInner::File(b)) => b
                .insert(key)
                .map_err(|e| PyValueError::new_err(e.to_string())),
            None => Err(PyValueError::new_err("Builder already finished")),
        }
    }

    fn finish(&mut self) -> PyResult<Option<Set>> {
        match self.inner.take() {
            Some(BuilderInner::Memory(b)) => {
                let bytes = b
                    .into_inner()
                    .map_err(|e| PyValueError::new_err(e.to_string()))?;
                let set = FstSet::new(SetData::Vec(Arc::new(bytes)))
                    .map_err(|e| PyValueError::new_err(e.to_string()))?;
                Ok(Some(Set { inner: set }))
            }
            Some(BuilderInner::File(b)) => {
                b.finish()
                    .map_err(|e| PyValueError::new_err(e.to_string()))?;
                Ok(None)
            }
            None => Err(PyValueError::new_err("Builder already finished")),
        }
    }
}
