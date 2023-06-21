/*
    Copyright © 2023, ParallelChain Lab 
    Licensed under the Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
*/

//! Defines a collection of bareboned data structures for CRUD operations in world state. 
//! They are designed to get gas consumption to be efficient, consistent and predictable.
//! 
//! Collections:
//! - [Vector]
//! - [FastMap]
//! - [IterableMap]

pub(crate) mod vector;
#[allow(unused_imports)]
pub use vector::*;

pub(crate) mod fast_map;
#[allow(unused_imports)]
pub use fast_map::*;

pub(crate) mod iterable_map;
#[allow(unused_imports)]
pub use iterable_map::*;