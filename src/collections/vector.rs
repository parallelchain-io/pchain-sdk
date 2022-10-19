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

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ops::Index;
use std::ops::IndexMut;
use borsh::{BorshSerialize, BorshDeserialize};

use crate::storage;
use crate::{Storage, StorageField};

/// `Vector` is a contract-level data structure to provide abstraction by utilizing Get and Set operations associated with Contract Storage.
/// It supports lazy read/write to get gas consumption to be efficient, consistent and predictable.
#[derive(Clone, Default)]
pub struct Vector<T> where T: Storage {
    write_set: RefCell<BTreeMap<usize, T>>,
    read_set: RefCell<BTreeMap<usize, T>>,
    /// runtime length of the vector
    length: usize,
    /// The key of contract field, which is used to formulate the key for elements.
    parent_key: Vec<u8>
}

/// `VectorIter` is Iterator created by `Vector::iter()`
pub struct VectorIter<'a, T> where T: Storage + Clone {
    vector: &'a Vector<T>,
    idx: usize,
}

/// `VectorIter` is mutable Iterator created by `Vector::iter_mut()`
pub struct VectorIterMut<'a, T> where T: Storage + Clone {
    vector: &'a mut Vector<T>,
    idx: usize,
}

impl<'a, T> Vector<T> where T: Storage + Clone {
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

        // if not in read set,  read from world state
        let value = T::__load_storage(&StorageField::new().append(Self::wskey_index(self.parent_key.clone(), idx)));

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
        match storage::get(Self::wskey_len(parent_key).as_slice()) {
            Some(bytes) => {
                match usize::deserialize(&mut bytes.as_slice()) {
                    Ok(len) => len,
                    Err(_) => 0,
                }
            }
            None => 0
        }
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

impl<'a, T> Iterator for VectorIter<'a, T> where T: Storage + Clone {
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

impl<'a, T> Iterator for VectorIterMut<'a, T> where T: Storage + Clone {
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

impl<T> Index<usize> for Vector<T> where T: Storage + Clone {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index)
    }
}

impl<T> IndexMut<usize> for Vector<T> where T: Storage + Clone {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let value = self.get(index);
        // return reference to write set
        self.write_to_write_set(index, value.clone())
    }
}

impl<T> Storage for Vector<T> where T: Storage + Clone {
    fn __load_storage(field :&StorageField) -> Self {
        let parent_key = field.get_path().to_vec();
        Self {
            write_set: RefCell::new(BTreeMap::new()),
            read_set: RefCell::new(BTreeMap::new()),
            length: Self::len_in_ws(parent_key.clone()),
            parent_key,
        }
    }

    fn __save_storage(&mut self, field :&StorageField) {
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
            v.__save_storage(&StorageField::new().append(Self::wskey_index(self.parent_key.clone(), *idx)));
        });
    }
}