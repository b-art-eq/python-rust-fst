use pyo3::prelude::*;

mod map;
mod set;
mod util;

#[pymodule]
fn _native(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<map::Map>()?;
    m.add_class::<map::MapBuilder>()?;
    m.add_class::<map::MapKeys>()?;
    m.add_class::<map::MapValues>()?;
    m.add_class::<map::MapItems>()?;
    m.add_class::<map::MapRegexStream>()?;
    m.add_class::<map::MapLevStream>()?;
    
    m.add_class::<set::Set>()?;
    m.add_class::<set::SetBuilder>()?;
    m.add_class::<set::SetStream>()?;
    m.add_class::<set::SetRegexStream>()?;
    m.add_class::<set::SetLevStream>()?;
    m.add_class::<set::SetUnion>()?;
    m.add_class::<set::SetIntersection>()?;
    m.add_class::<set::SetDifference>()?;
    m.add_class::<set::SetSymmetricDifference>()?;
    
    Ok(())
}
