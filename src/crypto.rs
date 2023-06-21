/*
    Copyright © 2023, ParallelChain Lab 
    Licensed under the Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
*/

//! Defines runtime-supported cryptographic functions. What cryptgraphic functions have in common 
//! is that the operations they implement are: 1. Relatively expensive, and 2. Relatively common in Contract applications. 
//! In order to reduce gas costs, these functions are implemented in native (not-WASM) code that lives outside of the 
//! WASM runtime, and exposed to calls through the handles defined in this module.

use crate::imports;

/// Computes the SHA256 digest (32 bytes) of arbitrary input.
pub fn sha256(input: Vec<u8>) -> Vec<u8>{
    let input_ptr = input.as_ptr();

    let mut val_ptr: u32 = 0;
    let val_ptr_ptr = &mut val_ptr;

    unsafe {
        imports::sha256(input_ptr, input.len() as u32, val_ptr_ptr);
        Vec::<u8>::from_raw_parts(val_ptr as *mut u8, 32, 32)
    }
}

/// Computes the Keccak256 digest (32 bytes) of arbitrary input.
pub fn keccak256(input: Vec<u8>) -> Vec<u8>{
    let input_ptr = input.as_ptr();

    let mut val_ptr: u32 = 0;
    let val_ptr_ptr = &mut val_ptr;

    unsafe {
        imports::keccak256(input_ptr, input.len() as u32, val_ptr_ptr);
        Vec::<u8>::from_raw_parts(val_ptr as *mut u8,  32, 32)
    }
}

/// Computes the RIPEMD160 digest (20 bytes) of arbitrary input.
pub fn ripemd(input: Vec<u8>) -> Vec<u8>{
    let input_ptr = input.as_ptr();

    let mut val_ptr: u32 = 0;
    let val_ptr_ptr = &mut val_ptr;

    unsafe {
        imports::ripemd(input_ptr, input.len() as u32, val_ptr_ptr);
        Vec::<u8>::from_raw_parts(val_ptr as *mut u8, 20, 20)
    }
}

/// Returns whether an Ed25519 signature was produced by a specified by a specified address over some specified message.
/// Contract call fails if the input `address` or `signature` is not valid.
pub fn verify_ed25519_signature(input: Vec<u8>, signature: Vec<u8>, address: Vec<u8>) -> bool {
    let input_ptr = input.as_ptr();
    let signature_ptr = signature.as_ptr();
    let address_ptr = address.as_ptr();

    let value;
    unsafe {
        value = imports::verify_ed25519_signature(input_ptr, input.len() as u32, signature_ptr, address_ptr);
    }

    value != 0
}