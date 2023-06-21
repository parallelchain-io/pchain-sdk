/*
    Copyright © 2023, ParallelChain Lab 
    Licensed under the Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
*/

//! Defines the collection struct [IterableMap].

use std::{marker::PhantomData, collections::BTreeMap};
use borsh::{BorshDeserialize, BorshSerialize};
use crate::{storage::{self}, Storable, StoragePath};

/// [IterableMap] is a contract-level data structure to provide abstraction by utilizing Get and Set operations associated with Contract Storage.
/// It supports lazy read/write on key-value tuples which can also be iterated as a vector.
/// 
/// ## IterableMap
/// 
/// `IterableMap` can be a Contract Field defined in the contract struct. E.g.
/// 
/// ```rust
/// #[contract]
/// struct MyContract {
///     iterable_map: IterableMap<K, V>,
/// }
/// ```
/// 
/// ### Storage Model
/// 
/// Account Storage State Key Format:
/// 
/// |Component|Key|Value (Data type) |
/// |:---|:---|:---|
/// |Map Info|P, 0|`MapInfoCell`|
/// |Key-Index|P, 1, L, K|`KeyIndexCell`|
/// |Index-Key|P, 2, L, I|`ValueCell` (data: K)|
/// |Index-Value|P, 3, L, I|`ValueCell`|
/// 
/// - P: parent key
/// - L: map level
/// - I: little endian bytes of index (u32)
/// - K: user defined key
/// 
/// ### Lazy Write
/// 
/// Trait `Storage` implements the `IterableMap` so that data can be saved to world state
/// 
/// 1. after execution of action method with receiver `&mut self`; or
/// 2. explicitly calling the setter `Self::set()`.
#[derive(Clone)]
pub struct IterableMap<K, V> 
    where K: BorshSerialize + BorshDeserialize,
          V: Iterable + Clone {
    parent_key: Vec<u8>,
    write_set: BTreeMap<Vec<u8>, UpdateOperation<V>>,
    _marker: PhantomData<Box<(K, V)>>
}

impl<K, V> IterableMap<K, V>
    where K: BorshSerialize + BorshDeserialize, 
          V: Iterable + Clone {
    
    /// Instantiate new instance of `IterableMap`. It does not interact with world state if it is not inserted into 
    /// contract field.
    /// ### Example
    /// ```no_run
    /// let nested_map: IterableMap<String, u64> = IterableMap::new();
    /// self.iterable_map.insert(&"nested_map".to_string(), nested_map);
    /// ```
    pub fn new() -> Self {
        Self { parent_key: vec![], write_set: BTreeMap::default(), _marker: PhantomData::default() }
    }

    /// Get data either from cached value or world state.
    /// ### Example
    /// ```no_run
    /// match self.get(key) {
    ///    Some(value) => {
    ///        log("GET".as_bytes(), format!("value = {}", value).as_bytes());
    ///    },
    ///    None => {
    ///        log("GET".as_bytes(), "key not found".as_bytes());
    ///    }
    /// }
    /// ```
    pub fn get(&self, key: &K) -> Option<V> {
        let key_bs = key.try_to_vec().unwrap();
        self.get_inner(&key_bs).map(|(v, _)| v)
    }

    fn get_inner(&self, key_bs: &Vec<u8>) -> Option<(V, bool)> {
        // search the cache with last update related to this key
        match self.write_set.get(key_bs) {
            Some(UpdateOperation::Delete) => { None }, // deleted key in cache
            Some(UpdateOperation::Insert(value, is_new_record)) => { Some((value.clone(), *is_new_record)) }, // found key in cache
            None=> { self.get_from_ws_by_key(key_bs).map(|v| (v, false)) } // get from world-state
        }
    }

    /// Get data as mutable reference to `Iterable` either from cached value or world state.
    /// ### Example
    /// ```no_run
    /// match self.iterable_map.get_mut(key) {
    ///     Some(value) => { 
    ///         // value is mutable reference.
    ///         *value += 1; 
    ///         // the change will be updated to world state after contract method execution
    ///     },
    ///     None => {}
    /// }
    /// ```
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let key_bs = key.try_to_vec().unwrap();
        self.get_mut_inner(&key_bs)
    }

    fn get_mut_inner(&mut self, key_bs: &Vec<u8>) -> Option<&mut V> {
        match self.get_inner(key_bs) {
            Some((iterable, is_new_record)) => {
                self.insert_inner(key_bs, iterable, is_new_record);
                match self.write_set.get_mut(key_bs) {
                    Some(UpdateOperation::Insert(mut_value, _)) => Some(mut_value),
                    _=> None
                }
            },
            None => None,
        }
    }

    /// Insert value to `IterableMap`. It returns a mutable reference to the inserted value in cache.
    /// ### Example
    /// ```no_run
    /// self.iterable_map.insert(key, value);
    /// ```
    pub fn insert(&mut self, key: &K, value: V) -> Option<&mut V> {
        let key_bs = key.try_to_vec().unwrap();
        let new_record = !self.is_key_used(&key_bs);
        self.insert_inner(&key_bs, value, new_record)
    }

    fn insert_inner(&mut self, key_bs: &Vec<u8>, value: V, new_record: bool) -> Option<&mut V> {
        self.write_set.insert(key_bs.clone(), UpdateOperation::Insert(value, new_record));
        match self.write_set.get_mut(key_bs) {
            Some(UpdateOperation::Insert(mut_value,  _)) => Some(mut_value),
            _=> None
        }
    }

    /// Remove key from `IterableMap`.
    pub fn remove(&mut self, key: &K) {
        let key_bs = key.try_to_vec().unwrap();
        self.write_set.insert(key_bs, UpdateOperation::Delete);
    }

    /// clear the map. It performs actual Write to world state.
    /// ### Example
    /// ```no_run
    /// // It performs clearance of pending key-value pairs and also world state.
    /// self.iterable_map.clear();
    /// // After this point, no value can be obtained after clear.
    /// self.iterable_map.get(key);
    /// ```
    pub fn clear(&mut self) {
        self.write_set.clear();
        self.new_ws_map_info();
    }

    /// Iterator to iterating keys in the map as `MapKey`. Iterating is a Lazy Read operation.
    /// ### Example
    /// ```no_run
    /// self.iterable_map.keys().for_each(|k|{
    ///     ...
    /// });
    /// ```
    pub fn keys(&self) -> IterableMapKeys<K, V> {
        let map_info_cell = self.get_map_info();
        let extends: Vec<Vec<u8>> = self.write_set.iter().filter_map(|w|{
            match w.1 { UpdateOperation::Insert(_, true) => Some(w.0.clone()), _ => None }
        }).collect();
        IterableMapKeys { iterable_map: self, idx: 0, level: map_info_cell.level, len: map_info_cell.sequence as usize, ext_idx: 0, extends }
    }

    /// Iterator to iterating values in the map as `Iterable`. Iterating is a Lazy Read operation.
    /// ### Example
    /// ```no_run
    /// self.iterable_map.values().for_each(|v|{
    ///     ...
    /// });
    /// ```
    pub fn values(&self) -> IterableMapValues<K, V> {
        let map_info_cell = self.get_map_info();
        let extends: Vec<Vec<u8>> = self.write_set.iter().filter_map(|w|{
            match w.1 { UpdateOperation::Insert(_, true) => Some(w.0.clone()), _ => None }
        }).collect();
        IterableMapValues{ iterable_map: self, idx: 0, level: map_info_cell.level, len: map_info_cell.sequence as usize, extends, ext_idx: 0 }
    }

    /// Mutable Iterator to iterating values in the map as `&mut Iterable`. Iterating is a Lazy Read operation.
    /// It is expensive operation because the values are expected to save back to storage at the end of contract execution.
    /// ### Example
    /// ```no_run
    /// self.iterable_map.values_mut().for_each(|v|{
    ///     ...
    /// });
    /// ```
    pub fn values_mut(&mut self) -> IterableMapValuesMut<K, V> {
        let map_info_cell = self.get_map_info();
        let extends: Vec<Vec<u8>> = self.write_set.iter().filter_map(|w|{
            match w.1 { UpdateOperation::Insert(_, true) => Some(w.0.clone()), _ => None }
        }).collect();
        IterableMapValuesMut{iterable_map: self, idx: 0, level: map_info_cell.level, len: map_info_cell.sequence as usize, ext_idx: 0, extends }
    }

    // Map information
    fn get_map_info(&self) -> MapInfoCell {
        if self.parent_key.is_empty() { // newly inserted map that is not yet save to world state
            return MapInfoCell { level: 0, sequence: 0 };
        }
        let ws_seq = self.wskey_map_info();
        MapInfoCell::load(ws_seq).unwrap()
    }

    // Next level of Map Information
    fn new_ws_map_info(&self) {
        let mut map_info_cell = self.get_map_info();
        map_info_cell.level += 1;
        map_info_cell.sequence = 0;
        let ws_seq = self.wskey_map_info();
        map_info_cell.save(ws_seq);
    }

    /// Get the index from Key-Index, given user-defined Key.
    /// Returns None if key is not found in Key-Index
    fn get_index(&self, key: &[u8] , level: u32) -> Option<u32> {
        let ws_key_index = self.wskey_key_index(key, level);
        KeyIndexCell::load(ws_key_index).map(|ki| ki.index)
    }

    /// Get Value, given user-defined key.
    /// 
    /// Steps:
    /// 1. get the index from Key-Index
    /// 2. get the value from Index-Value
    fn get_from_ws_by_key(&self, key: &[u8]) -> Option<V> {
        let map_info_cell = self.get_map_info();
        let ws_index = self.get_index(key, map_info_cell.level);
        let ws_key_index_value = match ws_index {
            Some(ws_index) if ws_index < map_info_cell.sequence => self.wskey_index_value(map_info_cell.level, &ws_index),
            _ => return None,
        };
        V::load(ws_key_index_value)
    }

    /// Check if the user-defined key has been used. It could be alive or already deleted.
    /// If it is used, new data to insert should reuse the cell (overriding).
    fn is_key_used(&self, key: &[u8]) -> bool {
        // search the cache with last update related to this key
        let map_info_cell = self.get_map_info();
        if map_info_cell.sequence == 0 { return false } // no elements
        let ws_index = match self.get_index(key, map_info_cell.level) {
            Some(ws_index) => ws_index,
            None => return false
        };
        ws_index < map_info_cell.sequence
    }

    /// Add value to world state , given user-defined key and current value of Sequence.
    /// Sequence will be increased afterwards.
    fn add_to_ws(&self, key: &[u8], level: u32, sequence: u32, value: V) {
        // 1. set sequence += 1
        let ws_seq = self.wskey_map_info();
        MapInfoCell {
            level,
            sequence: sequence + 1
        }.save(ws_seq);

        // 2. set key to index (sequence)
        let ws_key_index = self.wskey_key_index(key, level);
        KeyIndexCell {
            index: sequence
        }.save(ws_key_index);

        // 3. set index (sequence) to key
        let ws_index_key = self.wskey_index_key(level, &sequence);
        key.to_owned().save(ws_index_key);
        
        // 4. set to index-value (sequence)
        let ws_index_value = self.wskey_index_value(level, &sequence);
        let mut value = value;
        value.save(ws_index_value);
    }

    /// Lookup Key-Index and Index-Value. Update key and value to world state.
    fn set_to_ws(&self, key: &[u8], level: u32, value: V) {
        if let Some(index) = self.get_index(key, level) {
            // 1. set index to key
            let ws_index_key = self.wskey_index_key(level, &index);
            key.to_owned().save(ws_index_key);

            // 2. set to index-value
            let ws_index_value = self.wskey_index_value(level, &index);
            let mut value = value;
            value.save(ws_index_value);
        }
    }

    fn remove_from_ws(&self, key: &[u8], level: u32) {
        if let Some(index) = self.get_index(key, level) {
            // 1. set Index-Key to None
            let ws_index_key = self.wskey_index_key(level, &index);
            Vec::<u8>::delete(ws_index_key);

            // 2. set Index-Value to None
            let ws_index_value = self.wskey_index_value(level, &index);
            V::delete(ws_index_value.clone());

            // 2a. if it is a nested map, set Sequence of the nested map to 0
            if V::is_map(ws_index_value) {
                self.new_ws_map_info();
            }
        }
    }

    /// Account Storage State Key format for Map Information.
    /// Map Information consists of:
    /// 1. level
    /// 2. sequence
    /// 
    /// - Key: [P, 0]
    /// - Value Data Type: MapInfoCell
    /// 
    /// where
    ///  - P: Parent Key
    fn wskey_map_info(&self) -> Vec<u8> {
        [
            self.parent_key.to_vec(),
            [0u8].to_vec()
        ].concat()
    }

    /// Account Storage State Key format for Key-Index
    /// 
    /// - Key: [P, 1, L, K]
    /// - Value Data Type: KeyIndexCell
    /// 
    /// where
    ///  - P: Parent Key
    ///  - L: Map Level
    ///  - K: User defined Key
    fn wskey_key_index(&self, key: &[u8], level: u32) -> Vec<u8> {
        [
            self.parent_key.to_vec(),
            [1u8].to_vec(),
            level.to_le_bytes().to_vec(),
            key.to_vec()
        ].concat()
    }

    /// Account Storage State Key format for Index-Key
    /// 
    /// - Key: [P, 2, L, I]
    /// - Value Data Type: ValueCell (data: K)
    /// 
    /// where
    ///  - P: Parent Key
    ///  - K: User defined Key
    ///  - L: Map Level
    ///  - I: u32 value of index
    fn wskey_index_key(&self, level: u32, index: &u32) -> Vec<u8> {
        [
            self.parent_key.to_vec(),
            [2u8].to_vec(),
            level.to_le_bytes().to_vec(),
            index.to_le_bytes().to_vec()
        ].concat()
    }

    /// Account Storage State Key format for Index-Value
    /// 
    /// - Key: [P, 3, L, I]
    /// - Value Data Type: ValueCell
    /// 
    /// where
    ///  - P: Parent Key
    ///  - L: Map Level
    ///  - I: u32 value of index
    fn wskey_index_value(&self, level: u32, index: &u32) -> Vec<u8> {
        [
            self.parent_key.to_vec(),
            [3u8].to_vec(),
            level.to_le_bytes().to_vec(),
            index.to_le_bytes().to_vec()
        ].concat()
    }
}

impl<K, V> Iterable for IterableMap<K, V>
    where K: BorshSerialize + BorshDeserialize,
          V: Iterable + Clone {
    
    fn save(&mut self, key: Vec<u8>) {
        
        if self.parent_key.is_empty() {
            self.parent_key = key.clone();
            // If parent key is empty, this IterableMap is inserted as nested map. 
            // This behaviour is treated as creation of a new map and therefore MapInfo should be 
            // updated to clear previous data.
            self.new_ws_map_info();
        }

        let c = ValueCell { is_map: true, data: Some(self.try_to_vec().unwrap()) };
        storage::set(&key, c.try_to_vec().unwrap().as_slice());

        self.write_set.iter().for_each(|(key, ops)| {
            let map_info_cell = self.get_map_info();
            match ops {
                UpdateOperation::Insert(value, true) => {
                    self.add_to_ws(key, map_info_cell.level, map_info_cell.sequence, value.clone());
                },
                UpdateOperation::Insert(value, false) => {
                    self.set_to_ws(key, map_info_cell.level, value.clone());
                },
                UpdateOperation::Delete => {
                    self.remove_from_ws(key, map_info_cell.level);
                },
            }
        });
    }
}

impl<K, V> Storable for IterableMap<K, V> 
    where K: BorshSerialize + BorshDeserialize,
          V: Iterable + Clone {
    
    /// This method is called at the beginning of contract execution, if this `IterableMap` is a field of the Contract Struct.
    fn __load_storage(field: &StoragePath) -> Self {
        Self {
            parent_key: field.get_path().to_vec(),
            write_set: BTreeMap::default(),
            _marker: PhantomData,
        }
    }

    /// This method is called at the end of contract execution, if this `IterableMap` is a field of the Contract Struct.
    fn __save_storage(&mut self, field: &StoragePath) {
        self.save(field.get_path().to_vec());
    }
}

impl<K, V> BorshSerialize for IterableMap<K, V>
    where K: BorshSerialize + BorshDeserialize,
          V: Iterable + Clone {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // Serialization of `IterableMap` itself takes only parent_key to be stored.
        self.parent_key.serialize(writer)
    }
}

impl<K, V> BorshDeserialize for IterableMap<K, V>
    where K: BorshSerialize + BorshDeserialize,
          V: Iterable + Clone {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let parent_key = Vec::<u8>::deserialize_reader(reader)?;
        Ok(Self{
            parent_key,
            write_set: BTreeMap::default(),
            _marker: PhantomData,
        })
    }
}

/// Return data type for `IterableMap::keys()`
pub struct IterableMapKeys<'a, K, V>
    where K: BorshSerialize + BorshDeserialize,
          V: Iterable + Clone {
    iterable_map: &'a IterableMap<K, V>,
    idx: usize,
    level: u32,
    len: usize,
    ext_idx: usize,
    extends: Vec<Vec<u8>>,
}

impl<'a, K, V> Iterator for IterableMapKeys<'a, K, V>
    where K: BorshSerialize + BorshDeserialize,
          V: Iterable + Clone {
    type Item = K;

    fn next(&mut self) -> Option<K> {
        loop {
            if self.idx >= self.len  {
                if let Some(bytes) = self.extends.get(self.ext_idx) {
                    self.ext_idx += 1;
                    return Some(K::deserialize(&mut bytes.as_slice()).unwrap())
                }
                return None
            } else {
                let ws_index_key = self.iterable_map.wskey_index_key(self.level, &(self.idx as u32));
                if let Some(bytes) = Vec::<u8>::load(ws_index_key) {
                    self.idx += 1;
                    return Some(K::deserialize(&mut bytes.as_slice()).unwrap())
                }
            }
            self.idx += 1;
        }
    }
}

/// Return data type for `IterableMap::values()`
pub struct IterableMapValues<'a, K, V>
    where K: BorshSerialize + BorshDeserialize,
          V: Iterable + Clone {
    iterable_map: &'a IterableMap<K, V>,
    idx: usize,
    level: u32,
    len: usize,
    ext_idx: usize,
    extends: Vec<Vec<u8>>,
}

impl<'a, K, V> Iterator for IterableMapValues<'a, K, V> 
    where K: BorshSerialize + BorshDeserialize,
          V: Iterable + Clone {
    type Item = V;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.idx >= self.len {
                // keys that are newly inserted
                if let Some(bytes) = self.extends.get(self.ext_idx) {
                    return match self.iterable_map.write_set.get(bytes) {
                        Some(UpdateOperation::Insert(value, _)) => {
                            self.ext_idx += 1;
                            Some(value.clone())
                        },
                        _=> None
                    }
                }
                return None;
            } else {
                // keys that can be found in world state
                let ws_index_key = self.iterable_map.wskey_index_key(self.level, &(self.idx as u32));
                if let Some(bytes) = Vec::<u8>::load(ws_index_key) {
                    if let Some((value, _)) = self.iterable_map.get_inner(&bytes) {
                        self.idx += 1;
                        return Some(value);
                    }
                }
            }
            self.idx += 1;
        }
    }
}

/// Mutable iterator created by `IterableMap::values_mut()`
pub struct IterableMapValuesMut<'a, K, V> 
    where K: BorshSerialize + BorshDeserialize,
          V: Iterable + Clone {
    iterable_map: &'a mut IterableMap<K, V>,
    idx: usize,
    level: u32,
    len: usize,
    ext_idx: usize,
    extends: Vec<Vec<u8>>
}

impl<'a, K, V> Iterator for IterableMapValuesMut<'a, K, V> 
    where K: BorshSerialize + BorshDeserialize,
          V: Iterable + Clone {
    type Item = &'a mut V;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.idx >= self.len {
                // keys that are newly inserted
                if let Some(bytes) = self.extends.get(self.ext_idx) {
                    return match self.iterable_map.write_set.get_mut(bytes) {
                        Some(UpdateOperation::Insert(mut_value, _)) => {
                            self.ext_idx += 1;
                            Some(unsafe{
                                let r = mut_value as * const V;
                                &mut *(r as *mut V)
                            })
                        },
                        _ => None
                    };
                };
                return None;
            } else {
                // keys that can be found in world state
                let ws_index_key = self.iterable_map.wskey_index_key(self.level, &(self.idx as u32));
                if let Some(bytes) = Vec::<u8>::load(ws_index_key) {
                    if let Some(mut_value) = self.iterable_map.get_mut_inner(&bytes) {
                        self.idx += 1;
                        return Some(unsafe{
                            let r = mut_value as * const V;
                            &mut *(r as *mut V)
                        });
                    }
                }
            }
            self.idx += 1;
        }
    }
}

/// The trait that applies to most of the data types used as value of `IterableMap`.
/// Actual data stored to world state is in format of `ValueCell`.
pub trait Iterable : BorshSerialize + BorshDeserialize {
    fn is_map(key: Vec<u8>) -> bool {
        storage::get(&key).map_or(false, |bytes|{
            ValueCell::deserialize(&mut bytes.as_slice()).map_or(false, |c| c.is_map)
        })
    }
    
    fn load(key: Vec<u8>) -> Option<Self> {
        let bytes = storage::get(&key)?;
        let c = ValueCell::deserialize(&mut bytes.as_slice()).ok()?;
        let data = c.data?;
        Self::deserialize(&mut data.as_slice()).map_or(None, |s| Some(s))
    }

    fn save(&mut self, key: Vec<u8>) {
        let c = ValueCell { is_map: false, data: Some(self.try_to_vec().unwrap()) };
        storage::set(&key, c.try_to_vec().unwrap().as_slice());
    }

    fn delete(key: Vec<u8>) {
        let c = ValueCell { is_map: false,  data: None };
        storage::set(&key, c.try_to_vec().unwrap().as_slice());
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
struct ValueCell {
    /// indicator of whether the value being stored is a map (nested map)
    is_map: bool,
    /// The content is serialized from the value, which depends on implementation of 
    /// different data types in collections. None if data is deleted. 
    data: Option<Vec<u8>>
}

/// MapInfoCell defines the map information that is useful in constructing the storage model of `IterableMap`
#[derive(BorshSerialize, BorshDeserialize)]
struct MapInfoCell {
    /// level defines the level of the map by being part of the key of child of this map.
    /// It is part of prefix of the key to child element.
    level: u32,
    /// sequence is an increasing pointer to index of new inserted item.
    /// It is part of prefix of the key to child element.
    sequence: u32
}

impl Iterable for MapInfoCell {
    fn load(key: Vec<u8>) -> Option<Self> {
        if let Some(bytes) = storage::get(&key) {
            if let Ok(c) = MapInfoCell::deserialize(&mut bytes.as_slice()) {
                return Some(c)
            }
        }
        Some(Self { level: 0, sequence: 0 })
    }
    fn save(&mut self, key: Vec<u8>) { storage::set(&key, self.try_to_vec().unwrap().as_slice()) }
    fn delete(_key: Vec<u8>) { unreachable!() }
}

/// KeyIndexCell defines the data stored for Key-Index mapping in storage model of [IterableMap]
#[derive(BorshSerialize, BorshDeserialize)]
struct KeyIndexCell {
    // index of Key-Index mapping. It is part of prefix of the key to child element.
    index: u32
}

impl Iterable for KeyIndexCell {
    fn load(key: Vec<u8>) -> Option<Self> {
        let bytes = storage::get(&key)?;
        KeyIndexCell::deserialize(&mut bytes.as_slice()).ok()
    }
    fn save(&mut self, key: Vec<u8>) { storage::set(&key, self.try_to_vec().unwrap().as_slice()) }
    fn delete(_key: Vec<u8>) { unreachable!() }
}

// Defines Iterable to data types that supported from Borsh Serialization

macro_rules! define_primitives {
    ($($t:ty),*) => {
        $(
            impl Iterable for $t {}
        )*
    }
}
define_primitives!(
    u8, u16, u32, u64, u128,
    i8, i16, i32, i64, i128,
    String, bool, usize,
    [u8;32]
);
impl<T> Iterable for Option<T> where T: BorshSerialize + BorshDeserialize {}
impl<T> Iterable for Vec<T> where T: BorshSerialize + BorshDeserialize {}
macro_rules! impl_tuple {
    ($($idx:tt $name:ident)+) => {
      impl<$($name),+> Iterable for ($($name),+)
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