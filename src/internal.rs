/*
    Copyright © 2023, ParallelChain Lab 
    Licensed under the Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
*/

//! Internal functions that interacts within this transaction context. For example, setting data to receipts,
//! calling other contracts, transfer to other account.

use pchain_types::{cryptography::PublicAddress, serialization::{Serializable, Deserializable}, blockchain::Log, runtime::CallInput};

use crate::imports;

/// `return_value` places `value` in the receipt of a transaction.
pub fn return_value(value: Vec<u8>) {    
    let value_ptr = value.as_ptr();
    let value_len = value.len() as u32;
    unsafe {           
        imports::return_value(value_ptr, value_len);
    }
}

/// `log` saves message with a topic to receipt of a transaction.
pub fn log(topic: &[u8], value: &[u8]) {
    let event = Log { 
        topic: topic.to_vec(), 
        value: value.to_vec()
    };
    let serialized_event = Log::serialize(&event);

    let log_ptr= serialized_event.as_ptr();
    let log_len = serialized_event.len() as u32;

    unsafe {
        imports::_log(log_ptr, log_len);
    }
}

/// A call to contract. The caller should already know the data type of return value from the function call.
/// It returns Option of T where T is return value from the function. 
/// If data type T is different from the actual return value type of the function, None is returned.
pub fn call<T: borsh::BorshDeserialize>(address: PublicAddress, method_name: &str, arguments: Vec<u8>, value: u64) -> Option<T> {
    let return_value = call_untyped(address, method_name, arguments, value)?;
    T::deserialize(&mut return_value.as_slice()).map_or(None, |value| Some(value))
}

/// A call to contract, with vector of bytes as return type.
/// It returns Option of Vec of bytes. Interpretation on the bytes depends on caller
pub fn call_untyped(contract_address: PublicAddress, method_name: &str, arguments: Vec<u8>, value: u64) -> Option<Vec<u8>> {
    let call_command = pchain_types::blockchain::Command::Call( CallInput{ 
        target: contract_address, 
        method: method_name.to_string(), 
        arguments: <Vec<Vec<u8>>>::deserialize(&arguments).map_or(None, |args| Some(args)), 
        amount: if value > 0 { Some(value) } else { None }
    }).serialize();

    let call_ptr: *const u8 = call_command.as_ptr();
    let call_len = call_command.len() as u32;

    let mut return_val_ptr: u32 = 0;
    let return_val_ptr_ptr = &mut return_val_ptr;

    let return_value = unsafe {
        let return_val_len = imports::call(call_ptr, call_len, return_val_ptr_ptr);
        Vec::<u8>::from_raw_parts(return_val_ptr as *mut u8, return_val_len as usize, return_val_len as usize)
    };

    if return_value.is_empty() { 
        None
    } else {
        Some(return_value)
    }
}

/// transfer balance amount to another address. 
pub fn transfer(recipient: PublicAddress, amount: u64) {
    let mut transfer_bytes = Vec::new();
    transfer_bytes.append(&mut recipient.to_vec());
    transfer_bytes.append(&mut amount.to_le_bytes().to_vec());

    let transfer_ptr = transfer_bytes.as_ptr();
    unsafe { imports::transfer(transfer_ptr) }
}