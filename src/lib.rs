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

/// Defines functions for getting the information about the Blockchain that is available in the context of this
/// Transaction. This includes information about the 'current' Block, e.g., its height and timestamp, but also
/// information about preceding Blocks.
pub mod blockchain;

/// Defines functions for getting information about the Transaction that triggered this EtoC call, e.g., the
/// Transaction's `from_address`, `hash`, etc.
pub mod transaction;

/// Defines functions for reading and writing into Contract Storage (a.k.a. 'World State'). It also defines two types:
/// 'Storage' and 'StorageField', that are used in macro-expanded code in a convoluted and hard-to-explain manner. These
/// types will be moved out of this module, or removed entirely, in a future version of the SDK.
pub mod storage;
pub use storage::{Storage, StorageField, Cacher};

/// Defines a data structure, CallData, that Contracts written using the `#[contract]` macro use to unpack
/// those Transaction `data` that contain a serialized `pchain_types::sc_params::CallData`. This structure selects
/// the Method that an EtoC call should enter, as well as provides the arguments for the function call. 
///
/// Developers typically do not need to care about the contents of this module. The `#[contract]` macro handles
/// the creation and consumption of CallData in expanded code. 
pub mod call_data;
pub use call_data::{CallDataBuilder, CallData};

/// Defines a data structure, CallResult, that wraps the return value of CtoC calls. 
/// 
/// Developers typically do not need to care about the contents of this module. The `call_*` set of methods (defined)
/// in this file transparently unpacks CallResults and returns their inner (return) values. 
pub mod call_result;
pub use call_result::CallResult;

/// Defines data structures that can be used as rust standard data types (e.g. vector and map) in which
/// data is backed by contract storage while Read/Write operations are gas-efficient.
pub mod collections;

/// Defines so-called 'precompile' functions. What each precompile have in common is that the operations they implement
/// are: 1. Relatively expensive (often cryptographic), and 2. Relatively common in Contract applications. In order to
/// reduce gas costs, these functions are implemented in native (not-WASM) code that lives outside of the WASM runtime,
/// and exposed to EtoC calls through the handles defined in this module.
pub mod precompiles;

/// Defines a function, `alloc` that is exported from Contract WASM modules so that the ParallelChain F VM can allocate
/// segments in WASM linear memory. The name of this module is rather awkward, since `alloc` *is not* the only function
/// exported from Contract WASM modules. We plan to merge this module with imports in a future version of the SDK.
mod exports;

/// Defines the signatures of the externally-defined functions that Contract WASM modules expect to be linked to the WASM
/// runtime during module instantiation. The definitions (function bodies) of these functions can be found in Fullnode 
/// implementations.
mod imports;

pub use pchain_sdk_macros::{
    contract,
    contract_methods,
    contract_field,
    use_contract,
    view, action, init,
};

use borsh::BorshDeserialize;
use pchain_types::{Serializable, Event};

/// `return_value` places `value` in the receipt of an `ExternalToContract` transaction.
/// This method is not required when `contract_init` macro is being used on the actions()
/// entrypoint function.
pub fn return_value(value: Vec<u8>) {    
    let value_ptr = value.as_ptr();
    let value_len = value.len() as u32;
    unsafe {           
        imports::return_value(value_ptr, value_len);
    }
}

pub fn emit_event(topic: &[u8], value: &[u8]) {
    let event = Event{ 
        topic: topic.to_vec(), 
        value: value.to_vec()
    };
    let serialized_event = Event::serialize(&event);

    let event_ptr= serialized_event.as_ptr();
    let event_len = serialized_event.len() as u32;

    unsafe {
        imports::emit(event_ptr, event_len);
    }
}

/// A call to contract. The caller should already know the data type of return value from the function call
/// It returns Option of T where T is return value from the function. 
/// If data type T is different from the actual return value type of the function, None is returned.
pub fn call_action<T: BorshDeserialize>(address : pchain_types::PublicAddress, method_name:&str, arguments :Vec<u8>, value :u64) -> Option<T> {
    if let Some(ret)= call_action_untyped(address, method_name, arguments, value) {
        let mut ret = ret.as_slice();
        match BorshDeserialize::deserialize(&mut ret) {
            Ok(e) => {
                return Some(e);
            }
            _=>{ return None;}
        }
    }
    None
}

/// A view to contract. The caller should already know the data type of return value from the function view
/// It returns Option of T where T is return value from the function. 
/// If data type T is different from the actual return value type of the function, None is returned.
pub fn call_view<T: BorshDeserialize>(address : pchain_types::PublicAddress, method_name:&str, arguments :Vec<u8>) -> Option<T> {
    if let Some(ret)= call_view_untyped(address, method_name, arguments) {
        let mut ret = ret.as_slice();
        match BorshDeserialize::deserialize(&mut ret) {
            Ok(e) => {
                return Some(e);
            }
            _=>{ return None;}
        }
    }
    None
}

/// calling action method function which handled by blockchain executor
/// It returns Option of Vec of bytes. Interpretation on the bytes depends on caller
pub fn call_action_untyped(contract_address : pchain_types::PublicAddress, method_name:&str, arguments :Vec<u8>, value :u64) -> Option<Vec<u8>> {
    let contract_address_ptr : *const u8 = contract_address.as_ptr();

    let value_bs = value.to_le_bytes().to_vec();
    let value_ptr :*const u8 = value_bs.as_ptr();

    let mut val_ptr: u32 = 0;
    let val_ptr_ptr = &mut val_ptr;

    let is_multiple_methods_contract = method_name.len() > 0;

    let call_data = CallData::to_raw_call_data(method_name, arguments.clone());
    let call_data_ptr :*const u8 = call_data.as_ptr();
    let call_data_len = call_data.len();

    let value;
    unsafe {
        let val_len = imports::call_action(contract_address_ptr, call_data_ptr, call_data_len as u32, value_ptr as *const u8, val_ptr_ptr);

        // This Vec<u8> takes ownership of the segment of memory, letting the Rust ownership
        // system to Drop it later.
        value = Vec::<u8>::from_raw_parts(val_ptr as *mut u8, val_len as usize, val_len as usize);
    }

    match value {
        vec if vec.is_empty() => {
            None
        },
        return_value => {
            // [Ricky] This need to be fixed, all contract should have multiple methods for now
            // Raw call function should return the values set by tx.return_value
            // If mutiple methods is called, the return value type should be CallResult
            if is_multiple_methods_contract {
                match CallResult::deserialize(&mut return_value.as_slice()) {
                    Ok(call_result) => call_result.get(),
                    Err(_) => None,
                }
            } else { // Otherwise, raw bytes should be returned (e.g. for those contracts using contract_init)
                Some(return_value)
            }
        }
    }
}

  /// view contract by accessing view entrypoint of the contract
  pub fn call_view_untyped(contract_address : pchain_types::PublicAddress, method_name:&str, arguments :Vec<u8>) -> Option<Vec<u8>> {
    let contract_address_ptr : *const u8 = contract_address.as_ptr();
    let mut val_ptr: u32 = 0;
    let val_ptr_ptr = &mut val_ptr;

    let is_multiple_methods_contract = method_name.len() > 0;

    let call_data = CallData::to_raw_call_data(method_name, arguments.clone());
    let call_data_ptr :*const u8 = call_data.as_ptr();
    let call_data_len = call_data.len();

    let value;
    unsafe {
        let val_len = imports::call_view(contract_address_ptr, call_data_ptr, call_data_len as u32, val_ptr_ptr);

        // This Vec<u8> takes ownership of the segment of memory, letting the Rust ownership
        // system to Drop it later.
        value = Vec::<u8>::from_raw_parts(val_ptr as *mut u8, val_len as usize, val_len as usize);
    }

    match value {
        vec if vec.is_empty() => {
            None
        },
        return_value => {
            // Raw function should return the values set by tx.return_value
            // If mutiple methods is called, the return value type should be CallResult
            if is_multiple_methods_contract {
                match CallResult::deserialize(&mut return_value.as_slice()) {
                    Ok(call_result) => call_result.get(),
                    Err(_) => None,
                }
            } else { // Otherwise, raw bytes should be returned (e.g. for those contracts using contract_init)
                Some(return_value)
            }
        }
    }
}


/// pay() calls the raw_pay() that 
/// runs a ctoe call to transfer credit to another address. 
/// Return the remaining balance of the receiver's account
pub fn pay(address : pchain_types::PublicAddress, value : u64) -> u64 {
    let contract_address_ptr : *const u8 = address.as_ptr();
    let value_vs = value.to_le_bytes().to_vec();
    let value_ptr :*const u8 = value_vs.as_ptr();
    unsafe {
        imports::pay(contract_address_ptr, value_ptr as *const u8)
    }
}
