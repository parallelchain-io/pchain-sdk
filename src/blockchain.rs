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

/// pchain_sdk::blockchain is a handle containing the blockchain parameters of the smart 
/// contract invocation (e.g., the previous block hash, timestamp etc.)

/// Get the `number` field of the Block that contains the Transaction which triggered this Contract call. 
pub fn block_number() -> Vec<u8> {
    let mut args_ptr: u32 = 0;
    let args_ptr_ptr = &mut args_ptr;

    let arguments;
    unsafe {
        let args_len = imports::get_blockchain_height(args_ptr_ptr);
        arguments = Vec::<u8>::from_raw_parts(args_ptr as *mut u8,args_len as usize, args_len as usize);
    }
    arguments
}

/// Get the `prev_hash` field of the Block that contains the Transaction which triggered this Contract call.
pub fn prev_hash() -> Vec<u8> {
    let mut args_ptr: u32 = 0;
    let args_ptr_ptr = &mut args_ptr;

    let arguments;
    unsafe {
        let args_len = imports::get_blockchain_prev_hash(args_ptr_ptr);
        arguments = Vec::<u8>::from_raw_parts(args_ptr as *mut u8,args_len as usize, args_len as usize);
    }
    arguments
}

/// Get the `timestamp` field of the Block that contains the Transaction which triggered this Contract call.
pub fn timestamp() -> u32 {
    let mut args_ptr: u32 = 0;
    let args_ptr_ptr = &mut args_ptr;

    let arguments;
    unsafe {
        let args_len = imports::get_blockchain_timestamp(args_ptr_ptr);
        arguments = Vec::<u8>::from_raw_parts(args_ptr as *mut u8,args_len as usize, args_len as usize);
    }
    u32::from_le_bytes(arguments.try_into().unwrap())
}

/// Get the `random_bytes` field of the Block that contains the Transaction which triggered this Contract call.
pub fn random_bytes() -> Vec<u8> {
    let mut args_ptr: u32 = 0;
    let args_ptr_ptr = &mut args_ptr;

    let arguments;
    unsafe {
        let args_len = imports::get_blockchain_random_bytes(args_ptr_ptr);
        arguments = Vec::<u8>::from_raw_parts(args_ptr as *mut u8,args_len as usize, args_len as usize);
    }
    arguments
}