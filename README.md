# ParallelChain Mainnet Contract SDK

The ParallelChain Mainnet Contract SDK (`pchain-sdk`) provides Rust structs, functions, types, and macros that aid with the development of smart contracts executable in WebAssembly (WASM) engines implementing the ParallelChain Mainnet Contract Binary Interface (CBI) Subprotocol.  

Contracts are the run-time programmability mechanism of ParallelChain Mainnet networks. They allow users (Account owners) to implement arbitrary logic in a global, decentralized, and Byzantine Fault Tolerant replicated state machine to support their most business-critical applications. 

Theoretically, any WebAssembly (WASM) module that implements the CBI Subprotocol can be deployed onto a ParallelChain Mainnet blockchain. Practically, however, all developers (except perhaps those who like to experiment, or that would like to stretch the limits of the system) will want to use the types and macros in this `pchain-sdk` to write a Contract in Rust, and the commands in `pchain-compile` to compile the Rust source code into WASM bytecode that can be included in a Deploy command. 

## The Contract Programming Model

`pchain-sdk` enables developers to write Smart Contracts in an intuitive and readable style we call "The Contract Programming Model". The SDK's macros transparently generate lower-level 'boilerplate' code for you, so that you can focus on writing the business logic of your application.

The Contract Programming Model is inspired by Object-Oriented Programming (OOP). In the Model, a Contract can be thought of as a Rust struct that controls access to persistent Storage. Accounts interact with Contracts by submitting Transactions with Call commands to invoke Methods of contracts. 

## Contract struct

The `#[contract]` attribute macro turns any Rust struct into a Contract struct, as long as all of the struct's fields implement the `Storable` trait.

```rust
#[contract]
struct PrinceTheDog {
    age: u8,
    breed: String,
    hungry: bool,
    toy: DogToy
}
```

Out of the box, types that implement `Storable` includes all Rust primitive types, as well as as other commonly used types like `Option<T>`, `Result<T>`, `Vec<T>`, etc. In addition, structs defined by the developer can be made to implement Storage by applying the `#[contract_field]` macro on their definitions, as long as all of *their* fields implement Storable. For example:

```rust
#[contract_field]
struct DogToy {
    fun_score: u8,
}
```

Fields are the basic persistence mechanism of a contract. The SDK transparently generates code that loads all fields from the contract account's storage trie before the execution of contract methods, *and* saves all fields into the contract account's storage trie after execution finishes.

## Contract Methods

Marking an impl block for a contract struct with the `#[contract_methods]` attribute macro allows us to define contract methods in the impl block. Not every function defined in the impl block will become a contract method, rather, only those marked with the `#[call]` attribute macro will be. Functions not marked with `call` can still be used internally, they just won't be directly callable from a Call command or through the `view` RPC.

```rust
#[contract_methods]
impl PrinceTheDog {
    #[call]
    pub fn eat_food(&mut self, food: DogFood) -> Bark {
        /* method body omitted */
    }
}
```

### Accepting parameters and returning values

Contract methods can accept parameters (included in a Call command in the `arguments` field) and return values (included in a command receipt in the `return_value` field). For example, the `eat_food` method in the above example accepts a single parameter (of type `DogFood`) and returns a single value (of type `Bark`). 

Minimally, in order for a method's function signature to be valid, all arguments must implement `BorshDeserialize`, and all return values must implement `BorshSerialize`. 

In practice, however, arguments should also implement `BorshSerialize`, and return values should also implement `BorshDeserialize`, because respectively:
- The `arguments` field of the Call command has type `Option<Vec<Vec<u8>>>`, so each non-self method argument has to be borsh-serialized into `Vec<u8>` in order to be placed in a Call command.
- The `return_value` field of a command receipt has type `Vec<u8>`, so code consuming the return value must be able deserialize the bytes vector into the specific return value type in order to do anything really useful on it.

### Method receivers

A contract method may have a `&self` receiver, a `&mut self` receiver, or no receiver at all. It cannot have a `self` receiver. The body of the contract method can use the fields of the contract just like a regular Rust function with the same kind of receiver. This means that one can only mutate the contract's fields through the receiver if the receiver is `&mut self`.

Note that having a `&self` receiver or no receiver at all only prevents mutations from being done from the receiver, it doesn't prevent mutations from being done generally. So for example, a method with a `&self` receiver could still mutate the world state by calling `pchain_sdk::storage::set`.

## More about Contract Storage

### Cacher

As explained previously, by default, the SDK loads *all* of a contract's fields from storage before executing a method, and saves all of them into storage after the execution completes.

This default could be ideal for contracts that do not keep much in storage, or whose methods *always* read and write into every field in every call, but will in general result in contract calls that are not very gas-efficient.

```rust
#[contract]
struct PrinceTheDog {
    // `Cacher` can wrap around any type that implements `Storable`.
    age: Cacher<u8>,
    breed: Cacher<String>,
    hungry: Cacher<bool>,
    toy: Cacher<DogToy> 
}
```

Wrapping a contract field with the `Cacher<T>` struct (`pchain_sdk::storage::Cacher`) overrides this default behavior. Fields wrapped inside a `Cacher` are instead loaded "lazily", i.e., only if the specific call accesses them. In addition, `Cacher<T>` implements `Deref<Target=T>`, and so can be used in much of the same way as `T` with little extra syntax.

### Collections

Some fields will contain types (e.g., list types) that are comprised of a large "collection" of parts. Accesses to such collection fields will be inefficient even if they are wrapped inside `Cacher`, because `Cacher` loads *entire* fields. Efficient access of collections, therefore, require types that enable lazy loading of *parts* of fields, not only entire fields like `Cacher`.

The `collections` module (`pchain_sdk::collections`) provide exactly these kinds of types.  

#### Lazy lists: `Vector<T>`

Vector (`Vector<T>`) lazily stores a list of items in storage. Vector implements `Index`, `IndexMut`, and has an `iter` method, so most of the things you can do with `std::vec::Vec`, you can probably do with `Vector` too.

#### Lazy maps: `FastMap<K, V>` and `IterableMap<K, V>`

Collections include two types that store statically typed mapping between keys and values. The difference between these two types is that `IterableMap` is, as its name suggests, iterable. i.e., it has the standard library's HashMap's `keys`, `iter`, and `values` sets of methods. This functionality comes at the cost of storing slightly more data in Storage than `FastMap`. 

Like-typed maps can be nested together, but unlike-maps cannot, so for example `FastMap<T, FastMap<K, V>>` is permissible, but `FastMap<T, IterableMap<K, V>>` or `IterableMap<T, FastMap<K, V>>` are not. 

You should use `IterableMap` if your application needs to iterate through stored items, otherwise, use `FastMap`.

### Setting and getting directly

## Accessing information about the Blockchain

Contract Methods can be written to not only depend on call arguments and the contract's storage, but also on information about the Blockchain, e.g., the previous block hash, or the identity of the External Account that originated the Transaction with Call Command. 

Functions for getting information about the Transaction that triggered a Contract call and information about the larger Blockchain in general are defined in `pchain_sdk::transaction` and `pchain_sdk::blockchain` respectively. Internally, these functions are thin wrappers around functions defined in the Imports Set of the CBI.

## Calling other Contracts

The SDK includes a pair of functions to make Contract-To-Contract internal calls:
- `call` and `call_untyped`

It does the obvious: to call a method in a specified Contract with the given arguments.

## Transferring balance

`pchain_sdk::transfer` transfers balance from the Contract Account to another Account and returns the balance of the recipient after the transfer.