/*
    Copyright © 2023, ParallelChain Lab 
    Licensed under the Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
*/

//! ## Vector
//! 
//! `Vector` can be a Contract Field defined in the contract struct. E.g.
//!
//! ```rust
//! #[contract]
//! struct MyContract {
//!     vector: Vector<u64> // this vector declares to store u64 integers to world state
//! }
//! ```
//!
//! `Vector` supports following operations in contract:
//!
//! ```rust
//! // No read/set in world state happens when executing the below methods. 
//! pub fn len(&self) -> usize
//! pub fn push(&mut self, value: &T) 
//! pub fn pop(&mut self)
//! // No read to world state can happen when executing the below methods. 
//! pub fn iter(&'a self) -> VectorInto<'a, T>
//! pub fn iter_mut(&'a self) -> VectorIntoMut<'a, T>
//! ```
//!
//! The value can be obtained by indexing operations. E.g.
//! ```rust
//! let data = self.vector[0]; // can be either a read from cached value or a read from world state 
//! self.vector[0] = data; // No actual write to world state at this line
//! ```
//!
//! ### Iteration
//!
//! Iteration may involve read from world state. E.g.
//! ```rust
//! // Iterate over immutable reference to the data
//! self.vector.iter().for_each(|item|{
//!     ...
//! });
//! 
//! // Iterate over mutable reference to the data
//! self.vector.iter_mut().for_each(|item|{
//!     //...
//! });
//! ```
//! 
//! ### Storage Model
//! 
//! World State Key Format:
//! 
//! |Component|WS Key|WS Value (Data type) |
//! |:---|:---|:---|
//! |Length|P, 0| u32 |
//! |Element|P, 1, I| user defined data (borsh-serialized)|
//! - P: parent key
//! - I: little endian bytes of index (u32)
//! 
//! ### Lazy Write
//! 
//! Trait `Storage` implements the `Vector` so that data can be saved to world state
//! 1. after execution of action method with receiver `&mut self`; or
//! 2. explicitly calling the setter `Self::set()`.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ops::Index;
use std::ops::IndexMut;
use borsh::{BorshSerialize, BorshDeserialize};

use crate::storage;
use crate::{Storable, StoragePath};

/// `Vector` is a contract-level data structure to provide abstraction by utilizing Get and Set operations associated with Contract Storage.
/// It supports lazy read/write to get gas consumption to be efficient, consistent and predictable.
#[derive(Clone, Default)]
pub struct Vector<T> where T: Storable {
    write_set: RefCell<BTreeMap<usize, T>>,
    read_set: RefCell<BTreeMap<usize, T>>,
    /// runtime length of the vector
    length: usize,
    /// The key of contract field, which is used to formulate the key for elements.
    parent_key: Vec<u8>
}

/// `VectorIter` is Iterator created by `Vector::iter()`
pub struct VectorIter<'a, T> where T: Storable + Clone {
    vector: &'a Vector<T>,
    idx: usize,
}

/// `VectorIter` is mutable Iterator created by `Vector::iter_mut()`
pub struct VectorIterMut<'a, T> where T: Storable + Clone {
    vector: &'a mut Vector<T>,
    idx: usize,
}

impl<'a, T> Vector<T> where T: Storable + Clone {
    pub fn new() -> Self {
        Self {
            write_set: RefCell::new(BTreeMap::new()),
            read_set: RefCell::new(BTreeMap::new()),
            length:0,
            parent_key: vec![]
        }
    }

    /// length of the vector
    pub fn len(&self) -> usize {
        self.length
    }

    /// `push` adds item to the last of vector, which does not immediately take effect in Contract Storage.
    pub fn push(&mut self, value: &T) {
        self.write_set.get_mut().insert(self.length, value.clone());
        self.length += 1;
    }

    /// `pop` removes the last item in the vector, which does not immediately take effect in Contract Storage. 
    /// Pop doest not return the poped item for saving reading cost.
    pub fn pop(&mut self) {
        if self.length > 0 {
            self.length -= 1;
            self.write_set.get_mut().remove(&(self.length));
            self.read_set.get_mut().remove(&(self.length)); 
        }
    }

    fn get(&self, idx: usize) -> &T {
        self.get_mut(idx)
    }

    /// get mutable reference from read_set
    fn get_mut(&self, idx: usize) -> &mut T {
        if idx >= self.length {
            panic!()
        }

        // get from write set
        if let Some(value) = self.write_set.borrow_mut().get_mut(&idx) {
            // cache to read set and return reference of it
            return self.write_to_read_set(idx, value.clone())
        }
        
        // if not in write set, get from read set
        if let Some(value) = self.read_set.borrow_mut().get_mut(&idx) {
            return unsafe {
                let r = value as * const T;
                &mut *(r as *mut T) 
            }
        }

        // parent key absent, cannot query world state data
        if self.parent_key.is_empty() { panic!() }

        // if not in read set, read from storage
        let value = T::__load_storage(&StoragePath::new().append(Self::wskey_index(self.parent_key.clone(), idx)));

        // cache to read set and return reference of it
        self.write_to_read_set(idx, value)
    }

    fn write_to_read_set(&self, idx: usize, value: T) -> &mut T {
        let mut read_set = self.read_set.borrow_mut();
        read_set.insert(idx, value);

        match read_set.get_mut(&idx) {
            Some(value) => {
                unsafe{
                    let r = value as * const T;
                    &mut *(r as *mut T) 
                }
            }
            None => unreachable!()
        }
    }

    fn write_to_write_set(&mut self, idx: usize, value: T) -> &mut T {
        let mut write_set = self.write_set.borrow_mut();
        write_set.insert(idx, value);

        match write_set.get_mut(&idx) {
            Some(value) => {
                unsafe{
                    let r = value as * const T;
                    &mut *(r as *mut T) 
                }
            }
            None => unreachable!()
        }
    }

    /// `iter` returns `VectorIter` which implements Iterator
    pub fn iter(&'a self) -> VectorIter<'a, T> {
        VectorIter { vector: self, idx: 0 }
    }

    /// `iter_mut` returns `VectorIterMut` which implements Iterator
    pub fn iter_mut(&'a mut self) -> VectorIterMut<'a, T> {
        VectorIterMut { vector: self, idx: 0 }
    }

    /// The length of the vector, which is the data stored in world state.
    fn len_in_ws(parent_key: Vec<u8>) -> usize {
        storage::get(Self::wskey_len(parent_key).as_slice()).map_or(0, |bytes|{
            usize::deserialize(&mut bytes.as_slice()).map_or(0, std::convert::identity)
        })
    }

    /// World State Key for saving the length of vector.
    fn wskey_len(parent_key: Vec<u8>) -> Vec<u8> {
        [
            parent_key,
            [0u8].to_vec()
        ].concat()
    }
    
    /// World State Key for saving the value of vector element, keyed by index of the element.
    fn wskey_index(parent_key: Vec<u8>, idx: usize) -> Vec<u8> {
        [
            parent_key, 
            [1u8].to_vec(), 
            (idx as u32).to_le_bytes().to_vec()
        ].concat()
    }
}

impl<'a, T> Iterator for VectorIter<'a, T> where T: Storable + Clone {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.vector.len() {
            return None
        }
        let value = self.vector.get(self.idx);
        self.idx += 1;
        Some(value)
    }
}

impl<'a, T> Iterator for VectorIterMut<'a, T> where T: Storable + Clone {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.vector.len() {
            return None
        };
        let value = self.vector.get(self.idx);
        let ret = self.vector.write_to_write_set(self.idx, value.clone());
        self.idx += 1;
        return Some(unsafe{
            let r = ret as * const T;
            &mut *(r as *mut T) 
        })
    }
}

impl<T> Index<usize> for Vector<T> where T: Storable + Clone {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index)
    }
}

impl<T> IndexMut<usize> for Vector<T> where T: Storable + Clone {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let value = self.get(index);
        // return reference to write set
        self.write_to_write_set(index, value.clone())
    }
}

impl<T> Storable for Vector<T> where T: Storable + Clone {
    fn __load_storage(field: &StoragePath) -> Self {
        let parent_key = field.get_path().to_vec();
        Self {
            write_set: RefCell::new(BTreeMap::new()),
            read_set: RefCell::new(BTreeMap::new()),
            length: Self::len_in_ws(parent_key.clone()),
            parent_key,
        }
    }

    fn __save_storage(&mut self, field :&StoragePath) {
        let field_path = field.get_path().to_vec();
        // set parent key here for the cases that Vector is instantiated first and then assigned to field in contract struct
        if self.parent_key != field_path {
            self.parent_key = field_path;
        }

        // update new length
        if self.length != Self::len_in_ws(self.parent_key.clone()) {
            storage::set(&Self::wskey_len(self.parent_key.clone()), self.length.try_to_vec().unwrap().as_slice());
        }

        // save changes to world state
        let mut write_set = self.write_set.borrow_mut();
        write_set.iter_mut().for_each(|(idx, v)|{
            v.__save_storage(&StoragePath::new().append(Self::wskey_index(self.parent_key.clone(), *idx)));
        });
    }
}