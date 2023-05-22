/*
    Copyright © 2023, ParallelChain Lab 
    Licensed under the Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
*/

//! Defines functions for getting the information about the Blockchain that is available in the context of this
//! Transaction. This includes information about the 'current' Block, e.g., its height and timestamp, but also
//! information about preceding Blocks. 

use crate::imports;

/// Get the `number` field of the Block that contains the Transaction which triggered this Contract call. 
pub fn block_number() -> u64 {
    unsafe { imports::block_height() }
}

/// Get the `prev_hash` field of the Block that contains the Transaction which triggered this Contract call.
pub fn prev_block_hash() -> Vec<u8> {
    let mut args_ptr: u32 = 0;
    let args_ptr_ptr = &mut args_ptr;

    unsafe {
        imports::prev_block_hash(args_ptr_ptr);
        Vec::<u8>::from_raw_parts(args_ptr as *mut u8, 32, 32)
    }
}

/// Get the `timestamp` field of the Block that contains the Transaction which triggered this Contract call.
pub fn timestamp() -> u32 {
    unsafe { imports::block_timestamp() }
}

/// Get the balance of current account
pub fn balance() -> u64 {
    unsafe { imports::balance() }
}