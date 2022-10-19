/*
 Copyright 2022 ParallelChain Lab

 Licensed under the Apache License, Version 2.0 (the "License");
 you may not use this file except in compliance with the License.
 You may obtain a copy of the License at

     http://www.apache.org/licenses/LICENSE-2.0

 Unless required by applicable law or agreed to in writing, software
 distributed under the License is distributed on an "AS IS" BASIS,
 WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 See the License for the specific language governing permissions and
 limitations under the License.
 */

//! collections defines bareboned data structures for CRUD operations in world state. 
//! They are designed to get gas consumption to be efficient, consistent and predictable.

/// vector defines data structure representing Vector in contract and data could be gas-efficiently 
/// load from or save to world state lazily. It behaves as `Vec` that supports `push`, `pop`, `iter`, 
/// indexing, etc.
pub(crate) mod vector;
#[allow(unused_imports)]
pub use self::vector::*;

/// fastmap defines data structure represening Map in contract and data could be gas-efficiently 
/// load from or save to world state lazily. It behaves as map that supports `get`, `insert`, `remove`, etc.
pub(crate) mod fast_map;
#[allow(unused_imports)]
pub use self::fast_map::*;

/// iterablemap defines data structure representing Map in contract and data could be gas-efficiently 
/// load from or save to world state lazily.It behaves as map that supports `get`, `insert`, `remove` and
/// iteration on keys or values.
pub(crate) mod iterable_map;
#[allow(unused_imports)]
pub use self::iterable_map::*;