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

use crate::imports;

/// pchain_sdk::transaction is a handle containing the parameters of transaction related smart contract invocation
/// (e.g., the 'args' string provided by the contract_caller, etc.)

/// Get from address of invoking transaction
pub fn from_address() -> [u8;32] {
    let mut args_ptr: u32 = 0;
    let args_ptr_ptr = &mut args_ptr;
    
    let arguments;
    unsafe {
        let args_len = imports::get_transaction_from_address(args_ptr_ptr);
        arguments = Vec::<u8>::from_raw_parts(args_ptr as *mut u8,args_len as usize, args_len as usize);
    }
    TryInto::<[u8;32]>::try_into(arguments).unwrap()
}

/// Get to address of invoking transaction
pub fn to_address() -> [u8;32] {
    let mut args_ptr: u32 = 0;
    let args_ptr_ptr = &mut args_ptr;

    let arguments;
    unsafe {
        let args_len = imports::get_transaction_to_address(args_ptr_ptr);
        arguments = Vec::<u8>::from_raw_parts(args_ptr as *mut u8,args_len as usize, args_len as usize);
    }
    TryInto::<[u8;32]>::try_into(arguments).unwrap()
}

/// Get value of invoking transaction
pub fn value() -> u64 {
    let mut args_ptr: u32 = 0;
    let args_ptr_ptr = &mut args_ptr;

    let arguments;
    unsafe {
        let args_len = imports::get_transaction_value(args_ptr_ptr);
        arguments = Vec::<u8>::from_raw_parts(args_ptr as *mut u8,args_len as usize, args_len as usize);
    }
    u64::from_le_bytes(arguments.try_into().unwrap())
}

/// Get nonce of invoking transaction
pub fn sequence_number() -> u64 {
    let mut args_ptr: u32 = 0;
    let args_ptr_ptr = &mut args_ptr;

    let arguments;
    unsafe {
        let args_len = imports::get_transaction_nonce(args_ptr_ptr);
        arguments = Vec::<u8>::from_raw_parts(args_ptr as *mut u8,args_len as usize, args_len as usize);
    }
    u64::from_le_bytes(arguments.try_into().unwrap())
}

/// Get transaction hash of invoking transaction
pub fn hash() -> [u8;32] {
    let mut args_ptr: u32 = 0;
    let args_ptr_ptr = &mut args_ptr;

    let arguments;
    unsafe {
        let args_len = imports::get_transaction_hash(args_ptr_ptr);
        arguments = Vec::<u8>::from_raw_parts(args_ptr as *mut u8,args_len as usize, args_len as usize);
    }
    TryInto::<[u8;32]>::try_into(arguments).unwrap()
}

/// get input data/arguments for entrypoint
pub fn data() -> Vec<u8> {
    let mut args_ptr: u32 = 0;
    let args_ptr_ptr = &mut args_ptr;

    let arguments;
    unsafe {
        let args_len = imports::get_transaction_data(args_ptr_ptr);
        arguments = Vec::<u8>::from_raw_parts(args_ptr as *mut u8,args_len as usize, args_len as usize);
    }
    arguments
}