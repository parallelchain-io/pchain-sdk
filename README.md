# ParallelChain F Contract SDK

The ParallelChain F Contract SDK (pchain-sdk) provides Rust structs, functions, types, and macros that aid with the development of smart contracts executable in WebAssembly (WASM) engines implementing the ParallelChain F Contract ABI Subprotocol.  

Contracts are the run-time programmability mechanism of ParallelChain F networks. They allow users (Account owners) to implement arbitrary logic in a global, decentralized, and Byzantine Fault Tolerant replicated state machine to support their most business-critical applications. 

Theoretically, any WebAssembly (WASM) module that implements the Contract ABI Subprotocol can be deployed onto a ParallelChain F blockchain. Practically, however, all developers (except perhaps those who like to experiment, or that would like to stretch the limits of the system) will want to use the types and macros in this `pchain-sdk` to write a Contract in Rust, and the commands in `pchain-compile` to compile the Rust source code into WASM bytecode that can be included in a Deploy Transaction. 

## The Contract Programming Model

pchain-sdk enable developers to write Smart Contracts in an intuitive and readable style we call "The Contract Programming Model". The SDK's macros transparently generate lower-level 'boilerplate' code for you, so that you can focus on writing the business logic of your application.

The Contract Programming Model is inspired by Object-Oriented Programming (OOP). In the Model, a Contract can be thought of as a Rust struct that controls access to persistent Storage. Accounts interact with Contracts by making EtoC Transactions calling its public methods, or just Methods for short. The following two sections elaborate on the two essential concepts of programming with the Contract Programming Model: Contract Methods, and Contract Storage.

## Contract Methods

The Model defines three kinds of Methods, each corresponding to an 'Entrypoint' in the Contract ABI Subprotocol.

1. **Action Methods**: can mutate Contract Storage. Can only be called through an on-chain, EtoC Transaction.
2. **View Methods**: can read, but not mutate Contract Storage. Can be called through the 'Contract View' endpoint of the Standard HTTP API, as well as through on-chain EtoC Transactions.
3. **Init Methods**: if defined on a Contract, is called *once* during the Contract's Deploy Transaction. In OOP lingo, this can be thought of as the 'constructor' of a Contract.

In order to produce to appropriate Contract ABI Exports Set bindings that ultimately allow Methods to be called from the outside world, you must write Method definitions inside an `impl Contract` statement marked with the `#[contract_methods]` macro, as illustrated in the following examples.

### Action Methods

```rust
#[contract_methods(meta)]
impl PrinceTheDog {

    #[action]
    pub fn eat_food(&mut self, food: DogFood) -> Poop {
        ...
    }
}
```

Actions Methods may mutate Contract Storage. Note however, that (as specified in the Transaction Subprotocol) mutations to Contract Storage made in an EtoC Transaction only get applied if the Transaction is Successful (e.g., the Transaction must exit with sufficient gas, must have not panicked during execution, etc.).

A function is callable as an Action Method if and only if:
1. It is marked using the `#[action]` macro.
2. It takes a `&mut` receiver in its argument list.
3. Its (zero or more) other arguments implement `BorshDeserialize`.
4. Its return value implements `BorshSerialize`, or it does not have a return value.

### View Methods

```rust
#[contract_methods(meta)]
impl PrinceTheDog {

    #[view]
    pub fn nicknames(&self) -> Vec<String> {
        ...
    }
}
```

View Methods have a consistent read-view into Contract Storage. The special thing about View Methods is that they may be called without incurring gas using the Contract View endpoint of the Standard HTTP API. Note however that calling them as part of a Contract Internal Transaction, does incur gas at normal rates.

A function is callable as a View Method if and only if:
1. It is marked using the `#[view]` macro.
2. It takes a `&self` receiver in its argument list.
3. Its (zero or more) other arguments implement `BorshDeserialize`.
4. Its return value implements `BorshSerialize`, or it does not have a return value. 

### Init Methods

```rust
#[contract_methods(meta)]
impl PrinceTheDog {

    #[init]
    fn init(dog_shelter: DogShelter) -> PrinceTheDog {
        ...
    }
```

The purpose of an Init Method is to initialize a Contract's Storage. A Contract can have zero or one Init Methods defined on it. An Init Method has full access to Contract Storage, just like an Action Method, but unlike an Action Method, it may not take `self` (or borrows of `self`) in its arguments list.

A function is callable as an Init Method if and only if: 
1. It is marked using the `#[init]` macro.
2. It does not take `self` or its borrows in its arguments list.
3. Its (zero or more) arguments implement `BorshDeserialize`.
4. It returns an instance of the Contract type.

## Accepting parameters and returning values

Some of the code snippets provided as examples in this document depict Contract Methods that take in function arguments (besides a borrow of the Contract struct) and/or return a value. In order for a Contract to receive arguments from and return values to the 'outside world' (callers), both Contract and caller need to agree on a serialization format.

`pchain-sdk` expects callers to serialize Method arguments using the [borsh](https://github.com/near/borsh) serialization standard, and generates code to serialize values into borsh for inclusion in a Transaction's Receipt, or transmission over the wire as part of the response for the Contract View endpoint of the Standard HTTP API. To be precise, EtoC Transactions specify the Contract Method to call and provide the arguments for the call by including a borsh-serialized `CallData` struct in its `data` field, and contracts include a borsh-serialized `CallResult` struct. The former type is defined in `pchain-types`, while the latter is defined in `pchain-sdk`. In the future, we plan to move both into the SDK. 

## Contract Storage

Contracts can use Storage to persist data between calls. The simplest way to read and write data into Storage is to add fields to the Contract struct:
```rust
#[contract]
struct PrinceTheDog {
    age: u8, 
    breed: String,
    hungry: bool,
    toy: DogToy,
}
```

The `#[contract]` macro transparently generates code that loads all of a Contract struct's fields from Storage before the execution of an Action or View method, and saves those fields into Contract Storage after an Action or View method returns. All types that implement the `Storable` trait can be used as a Contract field. Out of the box, this includes all Rust primitive types, as well as other commonly used types like `Option<T>`, `Result<T>`, `Vec<T>`, etc. In addition, structs defined by the developer can be made to implement `Storable` by applying the `#[contract_field]` macro on their definitions, provided that all of *their* fields implement Storable:
```rust
#[contract_field]
struct DogToy {
    ...
}
```

If your Contract struct contains fields, then the Contract should export an Init Method to initialize those fields.

### Storage and Collections

Because Storage is so gas-expensive, loading all of a Contract's fields before Method execution and writing them all into Storage after execution typically results in Contracts that are not very economical. For Contracts that do not keep much in Storage, or whose Methods *always* read and write into most fields, this maybe okay, or even ideal, and in general, developers ought to try and program contracts to minimize Storage use.

However, some applications cannot avoid keeping a lot of on-chain state, and for these applications eagerly loading and saving fields in every call may be unacceptably expensive. To solve this, the SDK includes a `pchain_sdk::collections` module. All of the types defined in this module 'lazily' load Storage: they only incur a read or write gas cost when the exact item in the collection is read from or written to. They also offer an API that can make working with large collections of data more convenient.

If your Contract struct contains `collections` fields, then its Init Method must initialize these fields by calling the relevant collection type's `new` function. An example with Cacher:

```rust
struct PrinceTheDog {
    name: Cacher<String>
}

impl PrinceTheDog {
    #[init]
    fn init() -> PrinceTheDog {
        PrinceTheDog {
            name: Cacher::new(),
        }
    }
}
```

#### <u>Cacher (`Cacher<T>`)</u>

Wraps over any non-collections type that implements `Storage` and makes them lazy (all other `collections` types are already lazy without Cacher). Cacher implements `Deref`, so `Cacher<T>` can be used *almost* everywhere `T` can be used without any special syntax. 

#### <u>Vector (`Vector<T>`)</u>

Lazily stores a list of items in `Storage`. Vector implements `Index`, `IndexMut`, and has an `iter` method, so most of the things you can do with `std::vec::Vec`, you can probably do with `Vector` too.

A temporary limitation is that you cannot nest Vectors (e.g., `Vector<Vector<T>>`). This will be enabled in a near-future update to the SDK.

#### <u>Maps (`FastMap<K, V>` and `IterableMap<K, V>`)</u>

Collections include two types that store statically-typed mapping between keys and values. The difference between these two types is that IterableMap is, as its name suggests, iterable. i.e., it has the standard library's HashMap's `keys`, `iter`, and `values` sets of methods. This functionality comes at the cost of storing slightly more data in Storage than FastMap. Both types function identically otherwise, down to being able to nest like-Maps together (e.g., `FastMap<T, FastMap<K, V>>`).

You should use IterableMap if your application absolutely needs to iterate through stored items, otherwise, use FastMap.

## Accessing information about the Blockchain

Contract Methods can be written to not only depend on call arguments and the contract's storage, but also on information about the Blockchain, e.g., the previous block hash, or the identity of the External Account that originated the EtoC Transaction. 

Functions for getting information about the Transaction that triggered a Contract call and information about the larger Blockchain in general are defined in `pchain_sdk::transaction` and `pchain_sdk::blockchain` respectively. Internally, these functions are thin wrappers around functions defined in the Imports Set of the Contract ABI.

## Calling other Contracts (unstable)

The SDK includes two pairs of functions to make CtoC (contract-to-contract) internal calls:
- `call_action` and `call_action_untyped`, and
- `call_view` and `call_view_untyped`.

Each pair does the obvious: the former calls an Action method in a specified Contract with the given arguments, the latter does the same with View methods.

## Transferring balance

`pchain_sdk::pay` transfers balance from the Contract Account to another Account and returns the balance of the recipient after the transfer.

## Contract Metadata

Contracts live in the World State as WASM bytecode. Like most bytecode, these are not designed to be human-readable. This means that it's virtually impossible for potential users of your Contract to discover the callable interface (set of exported Methods) of your Contract by querying for your Contract's code in the World State. This is where Contract Metadata comes in:

```rust
#[contract_methods(meta)]
impl PrinceTheDog {
    ...
}
```

Passing the 'meta' option to the `#[contract_methods]` attribute causes code to be generated during expansion that enables users to get a String from the 'get metadata' route of the Standard HTTP API that describes the contract's callable interface.
