/*
    Copyright © 2023, ParallelChain Lab 
    Licensed under the Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
*/
 
//! Defines functions for getting information about the Transaction that triggered this call, e.g. the
//! calling account (Transaction's `signer`) and `transaction_hash`, etc.

use crate::imports;

/// Get from address of invoking transaction
pub fn calling_account() -> [u8;32] {
    let mut args_ptr: u32 = 0;
    let args_ptr_ptr = &mut args_ptr;
    
    let arguments =
    unsafe {
        imports::calling_account(args_ptr_ptr);
        Vec::<u8>::from_raw_parts(args_ptr as *mut u8, 32, 32)
    };
    TryInto::<[u8;32]>::try_into(arguments).unwrap()
}

/// Get to address of invoking transaction
pub fn current_account() -> [u8;32] {
    let mut args_ptr: u32 = 0;
    let args_ptr_ptr = &mut args_ptr;

    let arguments =
    unsafe {
        imports::current_account(args_ptr_ptr);
        Vec::<u8>::from_raw_parts(args_ptr as *mut u8, 32, 32)
    };
    TryInto::<[u8;32]>::try_into(arguments).unwrap()
}

/// Get value of invoking transaction
pub fn amount() -> u64 {
    unsafe { imports::amount() }
}

/// Returns whether it is an internal call
pub fn is_internal_call() -> bool {
    unsafe { imports::is_internal_call() != 0 }
}

/// Get transaction hash of invoking transaction
pub fn transaction_hash() -> [u8;32] {
    let mut args_ptr: u32 = 0;
    let args_ptr_ptr = &mut args_ptr;

    let arguments =
    unsafe {
        imports::transaction_hash(args_ptr_ptr);
        Vec::<u8>::from_raw_parts(args_ptr as *mut u8,32, 32)
    };
    TryInto::<[u8;32]>::try_into(arguments).unwrap()
}

/// Get method name of the invoking Contract Method
pub fn method() -> String {
    let mut args_ptr: u32 = 0;
    let args_ptr_ptr = &mut args_ptr;

    let arguments = 
    unsafe {
        let args_len = imports::method(args_ptr_ptr);
        Vec::<u8>::from_raw_parts(args_ptr as *mut u8,args_len as usize, args_len as usize)
    };
    String::from_utf8(arguments).unwrap()
}

/// Get method name of the invoking Contract Method
pub fn arguments() -> Vec<u8> {
    let mut args_ptr: u32 = 0;
    let args_ptr_ptr = &mut args_ptr;

    unsafe {
        let args_len = imports::arguments(args_ptr_ptr);
        Vec::<u8>::from_raw_parts(args_ptr as *mut u8,args_len as usize, args_len as usize)
    }
}