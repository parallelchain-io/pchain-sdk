/*
    Copyright © 2023, ParallelChain Lab 
    Licensed under the Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
*/

//! The ParallelChain Mainnet Contract SDK (pchain-sdk) provides Rust structs, functions, types, and macros that 
//! aid with the development of smart contracts executable in WebAssembly (WASM) engines implementing 
//! the ParallelChain Mainnet Contract Binary Interface Subprotocol.

pub mod blockchain;

pub mod crypto;

mod exports;

mod imports;

pub mod internal;
pub use internal::*;

pub mod method;
pub use method::{ContractMethodInput, ContractMethodOutput};

pub mod network;

pub mod storage;
pub use storage::{Storable, StoragePath, Cacher};

pub mod transaction;

pub mod collections;

pub use pchain_sdk_macros::{
    contract,
    contract_methods,
    contract_field,
    call,
    use_contract,
};
