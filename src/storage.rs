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

use std::ops::{Deref, DerefMut};
use std::cell::UnsafeCell;
use borsh::{BorshSerialize, BorshDeserialize};
use crate::imports;

/// Gets the value, if any, associated with the provided key in this Contract Storage.
///
/// If get fails, the smart contract terminates and the sets this invocation made
/// are not committed.
pub fn get(key: &[u8]) -> Option<Vec<u8>> {

    let key_ptr = key.as_ptr();

    // `get` needs to get two things from `raw_get`:
    //   * A WASM linear memory offset pointing to where the host process
    //     wrote the UTF-8 encoded result of the DB get: `val_ptr`.
    //   * The byte-wise length of the gotten value: `val_len`.
    // 
    // `val_len` is returned directly, but the problem is that WASM does not
    // yet support multiple return values, so we can't return `val_ptr` alongside it.
    // Instead, we allocate a space in the heap for the host to write `val_ptr` in,
    // and then tell the host to write `val` there by passing it `val_ptr_ptr` through
    // `raw_get`.
    // `val_len` is negative if the key cannot be found in world-state.
    //
    // When val_ptr leaves this scope, it is deallocated (we have no further use
    // for it).
    let mut val_ptr: u32 = 0;
    let val_ptr_ptr = &mut val_ptr;

    unsafe {
        match imports::get(key_ptr, key.len() as u32, val_ptr_ptr) {
            val_len if val_len < 0 => {
                None
            },
            val_len => {
                // If module execution reaches this point, we can assume that the `get` has succeeded.
                //
                // This Vec<u8> takes ownership of the segment of memory, letting the Rust ownership
                // system to Drop it later.
                Some(Vec::<u8>::from_raw_parts(val_ptr as *mut u8, val_len as usize, val_len as usize))
            }
        }
    }
} 

/// Binds the provided key to the provided value in this Contract's Storage.
pub fn set(key: &[u8], value: &[u8]) {
    let key_ptr = key.as_ptr();
    let val_ptr = value.as_ptr();
    unsafe {       
        imports::set(key_ptr, key.len() as u32, val_ptr, value.len() as u32);
    } 
}

/// StorageField defines the key format in canonical path for fields in contract storage
#[derive(Clone)]
pub struct StorageField {
    path :Vec<u8>
}

impl StorageField {

    pub fn new() -> Self{
        Self { path: vec![] }
    }

    pub fn add(&self, child :u8) -> Self {
        let mut path = self.path.clone();
        path.push(child);
        Self { path }
    }

    pub fn append(&self, path: Vec<u8>) -> Self {
        Self { path: [self.path.clone(), path].concat() }
    }

    pub fn get_path(&self) -> &[u8] {
        self.path.as_slice()
    }
}

macro_rules! define_primitives {
    ($($t:ty),*) => {
        $(
            impl Storage for $t {
                fn __load_storage(field :&StorageField) -> Self {
                    Self::__get(field.get_path())
                }
                fn __save_storage(&mut self, field :&StorageField) {
                    Self::__set(field.get_path(), self)
                }
            }
        )*
    };
}
macro_rules! define_generics {
    ($($t:ty),*) => {
        $(
            impl<T> Storage for $t where T: BorshSerialize + BorshDeserialize{
                fn __load_storage(field :&StorageField) -> Self {
                    Self::__get(field.get_path())
                }
                fn __save_storage(&mut self, field :&StorageField) {
                    Self::__set(field.get_path(), self)
                }
            }
        )*
    };
}

define_primitives!(i8, u8, i16, u16, i32, u32, i64, u64, i128, u128, usize);
define_primitives!(String, bool);
define_generics!(Vec<T>, Option<T>);

/// Storage trait provides functions as wrapper to getter and setter to the key-value storage in world-state.
/// Impl of this trait is generated by macro. To avoid conflict with user function, function names in this trait are prefix with two underscores.
pub trait Storage {
    fn __get<T: BorshDeserialize + Default>(key_name: &[u8]) -> T {
        if let Some(bytes) = get(key_name) {
            if let Ok(value) = T::deserialize(&mut bytes.as_slice()) {
                return value
            }
        }
        T::default()
    }
    fn __set<T: BorshSerialize>(key_name: &[u8], data :&T) {
        set(key_name, data.try_to_vec().unwrap().as_slice());
    }
    /// generic implementation on method load_storage(), which depends on the type of caller
    fn __load_storage_field<T: Storage>(field :&StorageField) -> T {
        T::__load_storage(field)
    }
    /// generic implementation on method save_storage(), which depends on the type of caller
    fn __save_storage_field<T: Storage>(caller :&mut T, field :&StorageField) {
        caller.__save_storage(field)
    }
    /// the implementation should eventually call get() to obtain data from world-state and assign the value to the fields of struct
    fn __load_storage(field :&StorageField) -> Self;
    /// the implementation should eventually call set() to obtain fields' value of struct and save it to world-state
    fn __save_storage(&mut self, field :&StorageField);
}

/// `Cacher` is data wrapper to support Lazy Read and Lazy Write to Contract Storage.
/// 
/// ### Example
/// ```no_run
/// // Use default() to instantiate Cacher
/// let cacher: Cacher<u64> = Cacher::default();
/// // Defer-ed on behalf of the u64 data. Actual world state read happens once
/// let b = cacher.saturating_add(123);
/// // Value assignment after Defer-ed. No world state write. Actually write is handled afterwards by SDK.
/// *cacher = 123_u64;
/// ```
pub struct Cacher<T> where T: Storage {
    /// `scoped` defines the key format to store data T into world state
    scoped: StorageField,
    // None if Cacher is never Deref-ed into.
    inner: UnsafeCell<Option<T>>,
}

impl<T> Cacher<T> where T: Storage {

    pub fn default() -> Self {
        Self { scoped: StorageField::new(), inner: UnsafeCell::new(None) }
    }

    /// lazy read from world state
    fn load(&self) {
        let inner_ptr = self.inner.get() as *mut Option<T>;

        unsafe {
            let inner = &mut *inner_ptr;
            if inner.is_none() {
                *inner = Some(
                    T::__load_storage_field(&self.scoped)
                );
            }
        }
    } 

    pub fn get(&self) ->  &T {
        self.deref()
    }

    pub fn get_mut(&mut self) ->  &mut T {
        self.deref_mut()
    }

    pub fn set(&mut self, value: T) {
        *self.deref_mut() = value;
    }
}

impl<T> Deref for Cacher<T> where T: Storage {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.load();

        let inner_ptr = self.inner.get();
        unsafe { 
            let inner = &*inner_ptr;
            inner.as_ref().unwrap()
        }
    }
}

impl<T> DerefMut for Cacher<T> where T: Storage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.load();

        let inner_ptr = self.inner.get();
        unsafe { 
            let inner = &mut *inner_ptr;
            inner.as_mut().unwrap()
        }
    }
}

impl<T> Storage for Cacher<T> where T: Storage{
    fn __load_storage(field :&StorageField) -> Self {
        Cacher {
            scoped: field.clone(),
            inner: UnsafeCell::new(None),
        }
    }

    fn __save_storage(&mut self, field :&StorageField) {
        let inner_ptr = self.inner.get();
        unsafe {
            match &mut *inner_ptr {
                Some(inner) => {
                    inner.__save_storage(field);
                },
                None => {},
            }
        }
    }
}