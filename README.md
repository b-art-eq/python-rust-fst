# python-rust-fst

[![appveyor](https://ci.appveyor.com/api/projects/status/github/jbaiter/python-rust-fst)](https://ci.appveyor.com/project/jbaiter/python-rust-fst)
[![travis](https://travis-ci.org/jbaiter/python-rust-fst.svg)](https://travis-ci.org/jbaiter/python-rust-fst)
[![pypi downloads](https://img.shields.io/pypi/dm/rust_fst.svg?maxAge=2592000)](https://pypi.python.org/pypi/rust-fst)
[![pypi version](https://img.shields.io/pypi/v/rust_fst.svg?maxAge=2592000)](https://pypi.python.org/pypi/rust_fst)
[![pypi wheel](https://img.shields.io/pypi/wheel/rust_fst.svg?maxAge=2592000)](https://pypi.python.org/pypi/rust_fst)

Python bindings for [burntsushi's][1] [fst crate][2] ([rustdocs][3]) for FST-backed sets and maps.

This library allows you to:
- Work with larger-than-memory sets and maps.
- Perform fuzzy search using Levenshtein automata.
- Search using regular expressions.

## Installation

### From PyPI
```bash
pip install rust-fst
```

### From Source
You need to have Rust installed (stable version is fine).

```bash
git clone https://github.com/jbaiter/python-rust-fst.git
cd python-rust-fst
pip install .
```

## Usage

### Sets

#### Building a Set
You can build a set in memory or directly to a file. **Important: Keys must be inserted in lexicographical order.**

```python
from rust_fst import SetBuilder

# Build in memory
builder = SetBuilder(None) # None means in-memory
keys = ["bar", "baz", "foo", "möö"] # Must be sorted!
for key in keys:
    builder.insert(key)

# Finish building and get the Set object
s = builder.finish()

print(len(s)) # 4
print("foo" in s) # True
```

#### Building a Set to Disk
```python
from rust_fst import SetBuilder, Set

builder = SetBuilder("my_set.fst")
keys = ["bar", "baz", "foo"]
for key in keys:
    builder.insert(key)

# finish() returns None when building to a file
builder.finish()

# Load the set from disk
s = Set("my_set.fst")
```

#### Searching
```python
# Fuzzy search (Levenshtein distance)
# Find keys within distance 1 of "bam"
matches = list(s.search_lev("bam", 1))
# matches: ['bar', 'baz']

# Regular expression search
matches = list(s.search_re(r'ba.*'))
# matches: ['bar', 'baz']
```

#### Set Operations
Supported operations: `union`, `intersection`, `difference`, `symmetric_difference`, `is_subset`, `is_superset`, `is_disjoint`.

```python
s1 = SetBuilder(None)
s1.insert("a")
s1.insert("b")
set1 = s1.finish()

s2 = SetBuilder(None)
s2.insert("b")
s2.insert("c")
set2 = s2.finish()

# Union
print(list(set1.union(set2))) # ['a', 'b', 'c']

# Intersection
print(list(set1.intersection(set2))) # ['b']
```

### Maps

Maps associate a key (string) with a value (unsigned 64-bit integer).

#### Building a Map
```python
from rust_fst import MapBuilder

# Build in memory
builder = MapBuilder(None)
builder.insert("bar", 1)
builder.insert("foo", 2)
m = builder.finish()

print(m["bar"]) # 1
```

#### Iterating
```python
# Keys
print(list(m.keys())) # ['bar', 'foo']

# Values
print(list(m.values())) # [1, 2]

# Items
print(list(m.items())) # [('bar', 1), ('foo', 2)]
```

## Development

1. Install Rust (via [rustup](https://rustup.rs/)).
2. Install Python dependencies:
   ```bash
   pip install -r test-requirements.txt
   ```
3. Run tests:
   ```bash
   pytest
   ```

[1]: http://burntsushi.net
[2]: https://github.com/BurntSushi/fst
[3]: http://burntsushi.net/rustdoc/fst/
