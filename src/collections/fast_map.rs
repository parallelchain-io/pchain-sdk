/*
    Copyright © 2023, ParallelChain Lab 
    Licensed under the Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
*/

//! fastmap defines data structure represening Map in contract and data could be gas-efficiently 
//! load from or save to world state lazily. It behaves as map that supports `get`, `insert`, `remove`, etc.
//! 
//! ### Storage Model
//! 
//! World State Key Format:
//! 
//! |Component|WS Key|WS Value (Data type) |
//! |:---|:---|:---|
//! |Key-Value|P, E, K| CellContext |
//! 
//! - P: parent key
//! - E: little endian bytes of edition number (u32)
//! - K: user defined key
//! 
//! In world state, the key format is `parent key` + `edition` (u32, 4 bytes) + `user defined key`. If nested FastMap is 
//! inserted to FastMap as a value, `parent key` would be the key of the FastMap being inserted. Actual value to be stored
//! into world state is borsh-serialized structure of `Cell` which is either a value (bytes) or information of nested map.

use std::{marker::PhantomData, collections::BTreeMap};
use borsh::{BorshSerialize, BorshDeserialize};
use crate::{storage::{self}, Storable, StoragePath};

/// `FastMap` is a contract-level data structure to provide abstraction by utilizing Get and Set operations associated with Contract Storage.
/// It supports lazy read/write to get gas consumption to be efficient, consistent and predictable.
pub struct FastMap<K, V> 
    where K: BorshSerialize, 
          V: Insertable {
    parent_key: Vec<u8>,
    write_set: BTreeMap<Vec<u8>, UpdateOperation<V>>,
    _marker: PhantomData<Box<(K, V)>>
}

impl<K, V> FastMap<K, V> 
    where K: BorshSerialize, 
          V: Insertable {

    /// New instance of `FastMap` detached to world state, which is mainly used for being a nested map as a value of parent `FastMap`
    /// (by calling `insert` from parent `FastMap`). It does not interact with world state if it is not inserted into contract field.
    /// ### Example
    /// ```no_run
    /// let fast_map: FastMap<String, u64> = FastMap::new();
    /// self.fast_map.insert(&"fast_map".to_string(), fast_map);
    /// ```
    pub fn new() -> Self {
        Self { parent_key: vec![], write_set: BTreeMap::default(), _marker: PhantomData }
    }

    /// Get data either from cached value or world state.
    /// ### Example
    /// ```no_run
    /// match self.fast_map.get(key) {
    ///    Some(value) => {
    ///        log("GET".as_bytes(), format!("value = {}", value).as_bytes());
    ///    },
    ///    None => {
    ///        log("GET".as_bytes(), "key not found".as_bytes());
    ///    }
    /// }
    /// ```
    pub fn get(&self, key: &K) -> Option<V> {
        let key_bs = key.clone().try_to_vec().unwrap();

        match self.write_set.get(&key_bs) {
            Some(UpdateOperation::Delete) => return None,
            Some(UpdateOperation::Insert(v,_)) => {
                let v_serialized = v.try_to_vec().unwrap();
                return Some(V::deserialize(&mut v_serialized.as_slice()).unwrap());
            }
            None => {},
        }

        // Load from world state
        let ws_key = self.child_key(key_bs);
        V::load(ws_key)
    }

    /// Get data as mutable reference to the data that is obtained either from cached value or world state.
    /// ### Example
    /// ```no_run
    /// match self.fast_map.get_mut(key) {
    ///     Some(value) => { 
    ///         // value is mutable reference.
    ///         *value += 1; 
    ///         // the change will be updated to world state after contract method execution
    ///     },
    ///     None => {}
    /// }
    /// ```
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        match self.get(key) {
            Some(v) => {
                self.insert_inner(key, v, false);
                let key_bs = key.try_to_vec().unwrap();
                match self.write_set.get_mut(&key_bs) {
                    Some(UpdateOperation::Insert(mut_v, _)) => Some(mut_v),
                    _=> None
                }
            },
            None => None,
        }
    }

    /// Insert data to the cache of the `FastMap`. The value will be stored to world state after contract execution.
    /// ### Example
    /// ```no_run
    /// self.fast_map.insert(key, value);
    /// ```
    pub fn insert(&mut self, key: &K, value: V) {
        self.insert_inner(key, value, true);
    }

    fn insert_inner(&mut self, key: &K, value: V, new_record: bool) -> Option<&mut V> {
        let key_bs = key.try_to_vec().unwrap();
        self.write_set.insert(key_bs.clone(), UpdateOperation::Insert(value, new_record));
        match self.write_set.get_mut(&key_bs) {
            Some(UpdateOperation::Insert(mut_insertable, _)) => Some(mut_insertable),
            _=> None
        }
    }

    /// Remove key from `FastMap`. The delete will take effective to world state after contract execution.
    /// ### Example
    /// ```no_run
    /// self.fast_map.remove(key);
    /// ```
    pub fn remove(&mut self, key: &K) {
        let key_bs = key.try_to_vec().unwrap();
        self.write_set.insert(key_bs.clone(), UpdateOperation::Delete);
    }

    fn child_key(&self, key: Vec<u8>) -> Vec<u8> {
        let edition = Self::edition(&self.parent_key);
        Self::make_child_key(self.parent_key.to_vec(), edition, key)
    }
    fn make_child_key(parent_key: Vec<u8>, edition: u32, key: Vec<u8>) -> Vec<u8> {
        [
            parent_key.to_vec(),
            edition.to_le_bytes().to_vec(),
            key
        ].concat()
    }

}

impl<K, V> Insertable for  FastMap<K, V> 
    where K: BorshSerialize, 
          V: Insertable {
    /// Save to world state by `FastMap`'s storage model
    fn save(&mut self, key: Vec<u8>, is_new: bool){ 
        if self.parent_key.is_empty() {
            self.parent_key = key.clone();
        }

        let edition = match storage::get(&self.parent_key) {
            Some(bytes) => {
                match Cell::deserialize(&mut bytes.as_slice()) {
                    Ok(c) => c.edition + if is_new { 1 } else { 0 },
                    Err(_) => 0,
                }
            },
            None => 0
        };

        let c = Cell { edition, data: Some(self.parent_key.try_to_vec().unwrap()) };
        storage::set(&self.parent_key, c.try_to_vec().unwrap().as_slice());

        self.write_set.iter_mut().for_each(|(k, v)| {
            let vkey = Self::make_child_key(self.parent_key.to_vec(), edition, k.clone());
            match v {
                UpdateOperation::Insert(v, is_new) => {
                    v.save(vkey, *is_new);
                },
                UpdateOperation::Delete => {
                    V::delete(vkey);
                },
            }
        });
    }
}

impl<K, V> BorshSerialize for FastMap<K, V> 
    where K: BorshSerialize, 
          V: Insertable {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // Serialization of `FastMap` itself takes only parent_key to be stored.
        self.parent_key.serialize(writer)
    }
}

impl<K, V> BorshDeserialize for FastMap<K, V>
    where K: BorshSerialize, 
          V: Insertable {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let parent_key = Vec::<u8>::deserialize_reader(reader)?;
        Ok(Self{
            parent_key,
            write_set: BTreeMap::default(),
            _marker: PhantomData,
        })
    }
}

impl<K, V> Storable for FastMap<K, V> 
    where K: BorshSerialize, 
          V: Insertable {
    
    /// This method is called at the beginning of contract execution, if this `FastMap` is a field of the Contract Struct.
    fn __load_storage(field: &StoragePath) -> Self {
        Self {
            parent_key: field.get_path().to_vec(),
            write_set: BTreeMap::default(),
            _marker: PhantomData,
        }
    }

    /// This method is called at the end of contract execution, if this `FastMap` is a field of the Contract Struct.
    fn __save_storage(&mut self, field: &StoragePath) {
        self.save(field.get_path().to_vec(), false);
    }
}

/// `UpdateOpertaion` defines the runtime level update operations for Map.
#[derive(Clone)]
pub(crate) enum UpdateOperation<T> {
    /// Data for update, new record indicator
    Insert(T, bool),
    Delete
}

/// Basic data representation of the format of value being stored in world state.
#[derive(BorshSerialize, BorshDeserialize)]
struct Cell {
    /// edition of this slot.
    edition: u32,
    /// The content is serialized from the value, which depends on implementation of 
    /// different data types in collections. None if data is deleted. 
    data: Option<Vec<u8>>
}

/// `Insertable` defines default IO implementation between data types and world state.
pub trait Insertable : BorshSerialize + BorshDeserialize {
    fn edition(key: &Vec<u8>) -> u32 {
        storage::get(key).map_or(0, |bytes|{
            Cell::deserialize(&mut bytes.as_slice()).map_or(0, |c| c.edition)
        })
    }

    fn load(key: Vec<u8>) -> Option<Self> {
        let bytes = storage::get(&key)?;
        let c =  Cell::deserialize(&mut bytes.as_slice()).map_or(None, |c| Some(c))?;
        let data = c.data?;
        Self::deserialize(&mut data.as_slice()).map_or(None, |s| Some(s))
    }

    fn save(&mut self, key: Vec<u8>, is_new: bool) {
        let edition = storage::get(&key).map_or(0, |bytes| {
            Cell::deserialize(&mut bytes.as_slice()).map_or(0, |c| 
                c.edition + if is_new { 1 } else { 0 }
            )
        });
        let c = Cell { edition, data: Some(self.try_to_vec().unwrap()) };
        storage::set(&key, c.try_to_vec().unwrap().as_slice());
    }

    fn delete(key: Vec<u8>) {
        let edition = storage::get(&key).map_or(0, |bytes| {
            Cell::deserialize(&mut bytes.as_slice()).map_or(0, |c| c.edition + 1)
        });
        let c = Cell { edition, data: None };
        storage::set(&key, c.try_to_vec().unwrap().as_slice());
    }
}


// Defines Storable to data types that supported from Borsh Serialization

macro_rules! define_primitives {
    ($($t:ty),*) => {
        $(
            impl Insertable for $t {}
        )*
    }
}
define_primitives!(
    u8, u16, u32, u64, u128,
    i8, i16, i32, i64, i128,
    String, bool, usize,
    [u8;32]
);
impl<T> Insertable for Option<T> where T: BorshSerialize + BorshDeserialize {}
impl<T> Insertable for Vec<T> where T: BorshSerialize + BorshDeserialize {}
macro_rules! impl_tuple {
    ($($idx:tt $name:ident)+) => {
      impl<$($name),+> Insertable for ($($name),+)
      where $($name: BorshSerialize + BorshDeserialize,)+
      {}
    };
}
impl_tuple!(0 T0 1 T1);
impl_tuple!(0 T0 1 T1 2 T2);
impl_tuple!(0 T0 1 T1 2 T2 3 T3);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15 16 T16);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15 16 T16 17 T17);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15 16 T16 17 T17 18 T18);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15 16 T16 17 T17 18 T18 19 T19);