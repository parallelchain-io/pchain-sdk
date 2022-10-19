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

pub fn random() -> u64 {
    unsafe {
        imports::random()
    }
}
pub fn sha256(input :Vec<u8>) -> Vec<u8>{
    let input = input.clone();
    let input_ptr = input.as_ptr();

    let mut val_ptr: u32 = 0;
    let val_ptr_ptr = &mut val_ptr;

    let value;
    unsafe {
        let val_len = crate::imports::sha256(input_ptr, input.len() as u32, val_ptr_ptr);
        value = Vec::<u8>::from_raw_parts(val_ptr as *mut u8, val_len as usize, val_len as usize);
    }

    value
}

pub fn keccak256(input :Vec<u8>) -> Vec<u8>{
    let input = input.clone();
    let input_ptr = input.as_ptr();

    let mut val_ptr: u32 = 0;
    let val_ptr_ptr = &mut val_ptr;

    let value;
    unsafe {
        let val_len = crate::imports::keccak256(input_ptr, input.len() as u32, val_ptr_ptr);
        value = Vec::<u8>::from_raw_parts(val_ptr as *mut u8, val_len as usize, val_len as usize);
    }

    value
}

pub fn keccak512(input :Vec<u8>) -> Vec<u8>{
    let input = input.clone();
    let input_ptr = input.as_ptr();

    let mut val_ptr: u32 = 0;
    let val_ptr_ptr = &mut val_ptr;

    let value;
    unsafe {
        let val_len = crate::imports::keccak512(input_ptr, input.len() as u32, val_ptr_ptr);
        value = Vec::<u8>::from_raw_parts(val_ptr as *mut u8, val_len as usize, val_len as usize);
    }

    value
}

pub fn ripemd160(input :Vec<u8>) -> Vec<u8>{
    let input = input.clone();
    let input_ptr = input.as_ptr();

    let mut val_ptr: u32 = 0;
    let val_ptr_ptr = &mut val_ptr;

    let value;
    unsafe {
        let val_len = crate::imports::ripemd160(input_ptr, input.len() as u32, val_ptr_ptr);
        value = Vec::<u8>::from_raw_parts(val_ptr as *mut u8, val_len as usize, val_len as usize);
    }

    value
}

pub fn blake2b(input :Vec<u8>, output_len: u32) -> Vec<u8>{
    let input = input.clone();
    let input_ptr = input.as_ptr();

    let mut val_ptr: u32 = 0;
    let val_ptr_ptr = &mut val_ptr;

    let value;
    unsafe {
        let val_len = crate::imports::blake2b(input_ptr, input.len() as u32, output_len, val_ptr_ptr);
        value = Vec::<u8>::from_raw_parts(val_ptr as *mut u8, val_len as usize, val_len as usize);
    }

    value
}

pub fn verify_signature(input :Vec<u8>, signature: Vec<u8>, address :Vec<u8>) -> bool {
    let input = input.clone();
    let input_ptr = input.as_ptr();

    let signature = signature.clone();
    let signature_ptr = signature.as_ptr();

    let address = address.clone();
    let address_ptr = address.as_ptr();

    let value;
    unsafe {
        value = crate::imports::verify_signature(input_ptr, input.len() as u32, signature_ptr, address_ptr);
    }

    value != 0
}