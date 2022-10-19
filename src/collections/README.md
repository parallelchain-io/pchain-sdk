# ParallelChain F Smart Contract SDK - Collections

It is a collection of bareboned data structures for CRUD operations in world state. They are designed to get gas consumption to be efficient, consistent and predictable.

- [Vector](#vector)
- [FastMap](#fastmap)
- [IterableMap](#iterablemap)

## Vector

`Vector` can be a Contract Field defined in the contract struct. E.g.

```rust
#[contract]
struct MyContract {
    vector: Vector<u64> // this vector declares to store u64 integers to world state
}
```

`Vector` supports following operations in contract:

```rust
// No read/set in world state happens when executing the below methods. 
pub fn len(&self) -> usize
pub fn push(&mut self, value: &T) 
pub fn pop(&mut self)
// No read to world state can happen when executing the below methods. 
pub fn iter(&'a self) -> VectorInto<'a, T>
pub fn iter_mut(&'a self) -> VectorIntoMut<'a, T>
```

The value can be obtained by indexing operations. E.g.
```rust
let data = self.vector[0]; // can be either a read from cached value or a read from world state 
self.vector[0] = data; // No actual write to world state at this line
```

### Iteration

Iteration may involve read from world state. E.g.

```rust
// Iterate over immutable reference to the data
self.vector.iter().for_each(|item|{
    ...
});

// Iterate over mutable reference to the data
self.vector.iter_mut().for_each(|item|{
    ...
});
```

### Storage Model

World State Key Format:

|Component|WS Key|WS Value (Data type) |
|:---|:---|:---|
|Length|P, 0| u32 |
|Element|P, 1, I| user defined data (borsh-serialized)|

- P: parent key
- I: little endian bytes of index (u32)

### Lazy Write

Trait `Storage` implements the `Vector` so that data can be saved to world state

1. after execution of action method with receiver `&mut self`; or
2. explicitly calling the setter `Self::set()`.

## FastMap

`FastMap` can be a Contract Field defined in the contract struct. E.g.

```rust
#[contract]
struct MyContract {
    /// This FastMap accepts key and value with borsh-serializable data types.
    map: FastMap<String, u64> 
}
```

`FastMap` supports following operations in contract:

```rust
// No read/set in world state happens when executing the below methods, except get and get_mut which either get the value from cache or world state.
pub fn new() -> Self
pub fn get(&self, key: &K) -> Option<Insertable>
pub fn get_mut(&mut self, key: &K) -> Option<&mut Insertable>
pub fn insert(&mut self, key: &K, value: &V) -> Option<&mut Insertable>
pub fn remove(&mut self, key: &K)
```

### Storage Model

World State Key Format:

|Component|WS Key|WS Value (Data type) |
|:---|:---|:---|
|Key-Value|P, E, K| CellContext |

- P: parent key
- E: little endian bytes of edition number (u32)
- K: user defined key

In world state, the key format is `parent key` + `edition` (u32, 4 bytes) + `user defined key`. If nested FastMap is inserted to FastMap as a value, `parent key` would be the key of the FastMap being inserted.

Actual value to be stored into world state is borsh-serialized structure of `Cell` which is either a value (bytes) or information of nested map.

```rust
/// Basic data representation of the format of value being stored in world state.
struct Cell {
    /// edition of this slot.
    edition: u32,
    /// The content is serialized from the value, which depends on implementation of 
    /// different data types in collections. None if data is deleted. 
    data: Option<Vec<u8>>
}
```

In delete operation, data will be stored as `None` in world state (as a tombstone).

The field `edition` in `Cell` is to indicate the version of data representation. This number will be increased whenever the data is being updated. It is useful to provide consistency to the Map because __child elements will not be deleted even if the map is tombstoned__. 

### Lazy Write

Trait `Storage` implements the `FastMap` so that data can be saved to world state

1. after execution of action method with receiver `&mut self`; or
1. explicitly calling the setter `Self::set()`.



## IterableMap

`IterableMap` can be a Contract Field defined in the contract struct. E.g.

```rust
#[contract]
struct MyContract {
    iterable_map: IterableMap<K, V>,
}
```

`IterableMap` supports following operations in contract:

```rust
pub fn get(&self, key: &K) -> Option<V>
pub fn get_mut(&mut self, key: &K) -> Option<&mut Iterable>
pub fn insert(&mut self, key: &K, value: V) -> Option<&mut V>
pub fn remove(&mut self, key: &K)
pub fn keys(&self) -> IterableMapIntoKey
pub fn values(&self) -> IterableMapIntoValue
pub fn values_mut(&mut self) -> IterableMapIntoMutValue
pub fn clear(&mut self)
```

### Storage Model

World State Key Format:

|Component|WS Key|WS Value (Data type) |
|:---|:---|:---|
|Map Info|P, 0|\<MapInfoCell\>|
|Key-Index|P, 1, L, K|\<KeyIndexCell\>|
|Index-Key|P, 2, L, I|\<ValueCell\> (data: K)|
|Index-Value|P, 3, L, I|\<ValueCell\>|

- P: parent key
- L: map level
- I: little endian bytes of index (u32)
- K: user defined key

The structures `MapInfoCell`, `KeyIndexCell` and `ValueCell` are borsh-serializable.

```rust
struct MapInfoCell {
    /// level defines the level of the map by being part of the key of child of this map.
    /// It is part of prefix of the key to child element.
    level: u32,
    /// sequence is an increasing pointer to index of new inserted item.
    /// It is part of prefix of the key to child element.
    sequence: u32
}

struct KeyIndexCell {
    // index of Key-Index mapping. It is part of prefix of the key to child element.
    index: u32
}

struct ValueCell {
    /// indicator of whether the value being stored is a map (nested map)
    is_map: bool,
    /// The content is serialized from the value, which depends on implementation of 
    /// different data types in collections. None if data is deleted. 
    data: Option<Vec<u8>>
}
```

In delete operation, `None` will be stored to Index-Key and Index-Value in world state (as a tombstone).

The Sequence in Map Info keeps track of the count of insertion. It does not decrease in delete operation. But in Map clearing operation, Level in the Map Info is increased by 1 while Sequence in Map Info is set to 0.

### Lazy Write

Trait `Storage` implements the `IterableMap` so that data can be saved to world state

1. after execution of action method with receiver `&mut self`; or
1. explicitly calling the setter `Self::set()`.

### Operaion Sequence

#### Get

1. Get Map_Info
2. Get the index from Key-Index
3. Get the value from index

#### Insert

1. Get Map_Info
2. Set Map_Info.Sequence + 1
3. Set index to Key-Index
4. Set key to Index-Key
5. Set value to Index-Value

#### Update

1. Get index from Key-Index
2. Set key to Index-Key
3. Set value to Index-Value

#### Remove

1. Get index from Key-Index
2. Get value from index
3. Set None to Index-Key
4. Set None to Index-Value
5. Check value, if it is nested map, Clear the nested map

#### Clear Map

1. Get Map_Info
2. Set Map_Info.Level + 1, Map_Info.Sequence = 0
