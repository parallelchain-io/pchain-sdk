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

### Lazy Write

Trait `Storage` implements the `IterableMap` so that data can be saved to world state

1. after execution of action method with receiver `&mut self`; or
1. explicitly calling the setter `Self::set()`.