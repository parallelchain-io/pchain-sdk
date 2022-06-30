# ParallelChain F Smart Contract SDK

ParallelChain F Smart Contract SDK (pchain-sdk) provide Rust structs, functions, types, and macros that aid with the development of smart contracts executable in WebAssembly (WASM) engines implementing the ParallelChain F Smart Contract Virtual Machine (VM) specification.  

pchain-sdk enable developers to write Smart Contracts in a idiomatic and highly-readable style. The SDK's macros  transparently generate lower-level 'boilerplate' code for you, so that you can focus on writing the business logic of your application.

In particular, pchain-sdk is inspired by the Object-Oriented Programming (OOP) style. In the pchain-sdk model, a contract is simply a Rust struct that is associated with a set of functions and methods that are callable from the outside world (entrypoints).A quick glossary of the most important terms used in this README:

- **Entrypoint**: a starting point of contract execution. There are few kinds of entrypoints:
  - **Init Method**: an (optional) entrypoint that is called during contract deployment transaction.
  - **Action Method**: an entrypoint that is called in EtoC transactions, and that can read from and write into the blockchain World State.
  - **Views Method**: entrypoint that is limited to executing read-only operaions.
- **Contract Metadata**: a descriptive string to describe the entrypoint methods inside a contract. pchain-sdk exposes a macro that can automatically generate Contract Metadata for you.
- **Contract Storage**: persistent set of data that is accessible to a contract and its methods. It is internally implemented as a key-value storage in the blockchain World State.

## Entrypoint

ParallelChain smart contract execution engine recognizes certain functions defined in a contract as entrypoint. The recognized functions with designated function name are called **Entrypoints**:

|Function name | Entrypoint | SDK Support |
| :--- | :--- | :---|
|init| An entrance that will be entered once during the contract deployment.| Optional |
|actions| Starting point of execution when contract is being called in EtoC transaction.| Required|
|views| An entrance that can be entered in EtoC transaction or request from REST API | Optional |

## Define Contract Entrypoint

A very basic type of entrypoint is a function with name "contract". The macro `#[contract_init]` transforms the "contract" function to an extern "C" function 
so that it is callable to fullnode executor when the contract deployed as wasm file.

Before transforming,
```rust
#[contract_init]
pub fn actions(tx: Transaction) {
    ...

```


After transforming,
```rust
#[no_mangle]
pub extern "C" fn actions() {
    ...
```

## Contract Storage as a Struct 

Contract can perform set and get of data to its own world-state just like an object can access and modify its own fields in common programming languages.
The concept is applied here to provide a structure that is familiar to contract developers. The macro `#[contract]` applied to a struct will create getter and setter methods for the fields inside.

```rust
#[contract]
struct MyContract{
    data: i32
}
```

In the body of smart contract, it can access the data and update it without explicity calling `smart_contract::Transaction::get` and `smart_contract::Transaction::set`, take care of name of the key and arguments parsing. 

In this model,
- Key is an index in u8 integer format (hence, the maximum number of fields is 256). The above example, the key will be [0x0]. The order of intex is as same as the order of fields defined in the struct. 
- Value are borsh-serializable and borsh-deserializable
- Value is primitive types (i8, u8, i16, u16, i32, u32, i64, u64, i128, u128, usize, String, bool, Vec\<T\> where T is a primitive type)

### Nested struct

Nested struct is supported by deriving macro trait named `ContractField`. The nested struct also follows key-value conditions as same as Contract struct.

```rust
#[derive(ContractField)]
struct MyField {
    data1: u64,
    data2: i32
}

#[contract]
struct MyContract {
    my_field :MyField
}
```
In the above example, the key for storing in world-state will be [0x0, 0x0] for `data1` and [0x0, 0x1] for `data2` while the value stored in world-state will be borse-serialized u64 data.

### Getter and Setter

SDK provides convenience to access data in Contract Storage by creating getter and setting methods of fields defined in the contract struct.
If field name is `data`, then associate methods `get_data` and `set_data` could be called to obtain data stored in the Contract Storage.

For example,
```rust
#[contract]
struct MyContract {
    my_field :String
}

// Example
...
MyContract::set_my_field("store to storage".to_string());
let stored_data: String = MyContract::get_my_field();
assert_eq!("store to storage".to_string(), stored_data);
```

__note__: Getter and setter methods are not generated for Contract Field. Getting and setting this field will always load/save all data from/to Contract Storage.

## Entrypoint Methods as functions in an Impl

Contract can be invoked to carry out different operations just like accessing methods of an object or class in common programming languages. 
The concept is applied here to allow developers specify functions in an impl as entrypoint methods.
The macro `#[contract]` applied to an impl with macro `action` will generate a skeleton code that executes a method according to the function name in input arguments from caller.


Example:
```rust
#[contract]
impl MyContract {
    
    #[actoon]
    fn get_data(&self) -> i32 { 
        ...
    }
    
    #[action]
    fn set_data(&mut self, d1 :i32) {
        ...
    }
}
```


The generated skeleton consists of a `match` selector to find a matching `str` to the function name, and then execute it.

Example
```rust
match ctx.get_method_name() {
    "get_data" => {
        // code block to execute the function
        ...
    }
    ...
    _=>{ unimplemented!() }    
}

```

__note__:
- avoid defining method name with prefix `get_` or `set_` followed by field name because it causes conflicts with getter and setting methods that are auto-generated by SDK.
- if the method takes the immutable receiver `&self` as first argument, data is loaded from storage before execution. It is __expensive__ operation.
- if the method takes the mutable receiver `&mut self` as first argument, data is loaded from storage before execution and saved back to storage after execution. It is __expsnsive__ operation.


#### Input arguments from caller

The arguement inputted to the method is in format of `Vec<u8>`. It is composited of leading 4 bytes as format version number and the raw bytes representing a borsh-serialized struct.

```
[format version (4 bytes)][raw content (bytes)]
```
- Format version is u32 integer and converted to 4 bytes in little endian format. It determines the way to decode raw content. Current version = 0.
- Raw content refers to the struct CallData:
```rust
pub struct CallData{
    method_name :String,
    arguments :Vec<u8>,
}
```
- entrypoint is the name of the function of impl.
- arguments Vec\<u8\> which is also borsh-deserializable to **Vector of Vec\<u8\>**. For example,

```rust
fn entrypoint_1(data :i32, name :String) { ...
```
Then, **Vector of Vec\<u8\>** is a vector = {"data", "name"} where "data" and "name" are Borsh-Serialized bytes.

CallData is parsed from the transaction arguement. The parsing logics follows the procedure below:
- first 4 bytes are version bytes which match CALLDATA_VERSION
- if it is less than 4 bytes, it assumes version = 0 and CallData is returned with "empty" data
- panic if version does not match with CALLDATA_VERSION
- the rest of the bytes are borsh-serialized from the structure CallData


#### Return Value to Contract Call

The return value from a contract call is in format of `Vec<u8>`. It is borsh-seralizable so that it is up to developer to design the response data structure.

```rust
pub struct Callback {
    return_val :Vec<u8>
}
```

## Contract MetaData

Contract MetaData is descriptive information about the contract's entrypoint methods. It is represented as a trait of the contract.

Example:
```rust
pub trait HelloContract {
    fn hello();
    fn hello_from(name:String) -> u32;
}
```

Smart contract developers can share or even publish this information to the public so that others can interact with the contract in a proper way.

To enable this feature in contract, add keyword "meta" as attribute to the contract macro.

```rust
#[contract(meta)]
impl MyContract {
    ...
}
```

Under the hood, SDK generates a static slice variable `__contract_metadata__` terminated by character '\0'. Its data resides in the memory section of the wasm code, which can be recognized by ParallelChain mainnet nodes.

## Init Entrypoint

The Init entrypoint is optional in the contract. It is enabled if the contract defines a constructor in contract Impl.

```rust
#[contract]
impl MyContract {

    /// Init entrypoint method 
    #[init]
    fn init_my_contract() {
        ...
```

Tne init entrypoint methods are recognized in the same way of actions entrypoint methods except that 
- macro `init` is applied on the method
- must be associate method (no recevier, i.e. self, as method argument)
- there should be only one `init` entrypoint method

## View Entrypoint

View entrypoint is optional in the contract. It is enabled by applying macro `view` on a method inside contract impl.

```rust
#[contract]
impl MyContract {

    #[view]
    pub fn view_my_data() -> i32 {
        ...
    }
}
```

Tne view entrypoint methods are recognized in the same way of actions entrypoint methods except that 
- macro `view` is applied on the method
- must be associate method (no recevier, i.e. self, as method argument)

