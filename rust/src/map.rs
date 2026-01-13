use pyo3::prelude::*;
use pyo3::exceptions::{PyKeyError, PyValueError, PyTypeError};
use std::fs::File;
use std::io::BufWriter;
use std::sync::Arc;
use memmap2::Mmap;
use fst::{Map as FstMap, MapBuilder as FstMapBuilder, Streamer, IntoStreamer};
use fst::automaton::Levenshtein;
use regex_automata::DenseDFA;

#[derive(Clone)]
pub enum MapData {
    Vec(Arc<Vec<u8>>),
    Mmap(Arc<Mmap>),
}

impl AsRef<[u8]> for MapData {
    fn as_ref(&self) -> &[u8] {
        match self {
            MapData::Vec(v) => v,
            MapData::Mmap(m) => m,
        }
    }
}

#[pyclass]
#[derive(Clone)]
pub struct Map {
    inner: FstMap<MapData>,
}

#[pymethods]
impl Map {
    #[new]
    fn new(path_or_bytes: &PyAny) -> PyResult<Self> {
        if let Ok(path) = path_or_bytes.extract::<String>() {
            let file = File::open(path)?;
            let mmap = unsafe { Mmap::map(&file)? };
            let map = FstMap::new(MapData::Mmap(Arc::new(mmap)))
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
            Ok(Map { inner: map })
        } else if let Ok(bytes) = path_or_bytes.extract::<&[u8]>() {
            let map = FstMap::new(MapData::Vec(Arc::new(bytes.to_vec())))
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
            Ok(Map { inner: map })
        } else {
            Err(PyTypeError::new_err("Argument must be a path (str) or bytes"))
        }
    }

    fn __contains__(&self, key: &str) -> bool {
        self.inner.contains_key(key)
    }

    fn __getitem__(&self, key: &str) -> PyResult<u64> {
        self.inner.get(key).ok_or_else(|| PyKeyError::new_err(key.to_string()))
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }
    
    fn get(&self, key: &str, default: Option<u64>) -> Option<u64> {
        self.inner.get(key).or(default)
    }

    fn keys(&self) -> MapKeys {
        let stream = self.inner.keys();
        let stream = unsafe { std::mem::transmute(stream) };
        MapKeys {
            _map: self.inner.clone(),
            stream,
        }
    }

    fn values(&self) -> MapValues {
        let stream = self.inner.values();
        let stream = unsafe { std::mem::transmute(stream) };
        MapValues {
            _map: self.inner.clone(),
            stream,
        }
    }

    fn items(&self) -> MapItems {
        let stream = self.inner.stream();
        let stream = unsafe { std::mem::transmute(stream) };
        MapItems {
            _map: self.inner.clone(),
            stream,
        }
    }
    
    fn search_re(&self, regex: &str) -> PyResult<MapRegexStream> {
        let dfa = regex_automata::dense::Builder::new()
            .anchored(true)
            .build(regex)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let stream = self.inner.search(&dfa).into_stream();
        let stream = unsafe { std::mem::transmute(stream) };
        Ok(MapRegexStream {
            _map: self.inner.clone(),
            _dfa: dfa,
            stream,
        })
    }

    fn search_lev(&self, key: &str, max_dist: u32) -> PyResult<MapLevStream> {
        let lev = Levenshtein::new(key, max_dist).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let stream = self.inner.search(&lev).into_stream();
        let stream = unsafe { std::mem::transmute(stream) };
        Ok(MapLevStream {
            _map: self.inner.clone(),
            _lev: lev,
            stream,
        })
    }
}

#[pyclass(unsendable)]
pub struct MapKeys {
    _map: FstMap<MapData>,
    stream: fst::map::Keys<'static>,
}

#[pymethods]
impl MapKeys {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> { slf }
    fn __next__(mut slf: PyRefMut<Self>) -> Option<String> {
        let bytes = slf.stream.next()?;
        Some(String::from_utf8_lossy(bytes).into_owned())
    }
}

#[pyclass(unsendable)]
pub struct MapValues {
    _map: FstMap<MapData>,
    stream: fst::map::Values<'static>,
}

#[pymethods]
impl MapValues {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> { slf }
    fn __next__(mut slf: PyRefMut<Self>) -> Option<u64> {
        slf.stream.next()
    }
}

#[pyclass(unsendable)]
pub struct MapItems {
    _map: FstMap<MapData>,
    stream: fst::map::Stream<'static>,
}

#[pymethods]
impl MapItems {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> { slf }
    fn __next__(mut slf: PyRefMut<Self>) -> Option<(String, u64)> {
        let (bytes, val) = slf.stream.next()?;
        Some((String::from_utf8_lossy(bytes).into_owned(), val))
    }
}

#[pyclass(unsendable)]
pub struct MapRegexStream {
    _map: FstMap<MapData>,
    _dfa: DenseDFA<Vec<usize>, usize>,
    stream: fst::map::Stream<'static, &'static DenseDFA<Vec<usize>, usize>>,
}

#[pymethods]
impl MapRegexStream {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> { slf }
    fn __next__(mut slf: PyRefMut<Self>) -> Option<(String, u64)> {
        let (bytes, val) = slf.stream.next()?;
        Some((String::from_utf8_lossy(bytes).into_owned(), val))
    }
}

#[pyclass(unsendable)]
pub struct MapLevStream {
    _map: FstMap<MapData>,
    _lev: Levenshtein,
    stream: fst::map::Stream<'static, &'static Levenshtein>,
}

#[pymethods]
impl MapLevStream {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> { slf }
    fn __next__(mut slf: PyRefMut<Self>) -> Option<(String, u64)> {
        let (bytes, val) = slf.stream.next()?;
        Some((String::from_utf8_lossy(bytes).into_owned(), val))
    }
}

enum BuilderInner {
    Memory(FstMapBuilder<Vec<u8>>),
    File(FstMapBuilder<BufWriter<File>>),
}

#[pyclass]
pub struct MapBuilder {
    inner: Option<BuilderInner>,
}

#[pymethods]
impl MapBuilder {
    #[new]
    fn new(path: Option<String>) -> PyResult<Self> {
        let inner = if let Some(p) = path {
            let file = File::create(p)?;
            let wtr = BufWriter::new(file);
            let builder = FstMapBuilder::new(wtr).map_err(|e| PyValueError::new_err(e.to_string()))?;
            BuilderInner::File(builder)
        } else {
            let builder = FstMapBuilder::memory();
            BuilderInner::Memory(builder)
        };
        Ok(MapBuilder { inner: Some(inner) })
    }

    fn insert(&mut self, key: &str, val: u64) -> PyResult<()> {
        match self.inner.as_mut() {
            Some(BuilderInner::Memory(b)) => b.insert(key, val).map_err(|e| PyValueError::new_err(e.to_string())),
            Some(BuilderInner::File(b)) => b.insert(key, val).map_err(|e| PyValueError::new_err(e.to_string())),
            None => Err(PyValueError::new_err("Builder already finished")),
        }
    }

    fn finish(&mut self) -> PyResult<Option<Map>> {
        match self.inner.take() {
            Some(BuilderInner::Memory(b)) => {
                let bytes = b.into_inner().map_err(|e| PyValueError::new_err(e.to_string()))?;
                let map = FstMap::new(MapData::Vec(Arc::new(bytes)))
                    .map_err(|e| PyValueError::new_err(e.to_string()))?;
                Ok(Some(Map { inner: map }))
            },
            Some(BuilderInner::File(b)) => {
                b.finish().map_err(|e| PyValueError::new_err(e.to_string()))?;
                Ok(None)
            },
            None => Err(PyValueError::new_err("Builder already finished")),
        }
    }
}
