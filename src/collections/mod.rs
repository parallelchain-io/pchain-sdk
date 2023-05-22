/*
    Copyright © 2023, ParallelChain Lab 
    Licensed under the Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
*/

//! collections defines bareboned data structures for CRUD operations in world state. 
//! They are designed to get gas consumption to be efficient, consistent and predictable.
//! 
//! This mod defines data structures that can be used as rust standard data types (e.g. vector and map) in which
//! data is backed by contract storage while Read/Write operations are gas-efficient.

pub(crate) mod vector;
#[allow(unused_imports)]
pub use self::vector::*;

pub(crate) mod fast_map;
#[allow(unused_imports)]
pub use self::fast_map::*;

pub(crate) mod iterable_map;
#[allow(unused_imports)]
pub use self::iterable_map::*;