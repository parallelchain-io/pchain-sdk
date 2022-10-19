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

extern "C" {
    // If set was unsuccessful, the WASM instance will be terminated
    // and changes rolled back.
    pub(crate) fn set(key_ptr: *const u8, key_len: u32, val_ptr: *const u8, val_len: u32);

    // because WASM doesn't yet support multiple return values, we
    // pass back a pointer to the beginning of the gotten value by
    // writing on val_ptr.
    //
    // If get was unsuccessful, the WASM instance will be terminated
    // and changes rolled back.
    pub(crate) fn get(key_ptr: *const u8, key_len: u32, val_ptr_ptr: *const u32) -> i64;
    
    // Getters for Transaction-related data.
    pub(crate) fn get_transaction_from_address(from_address_ptr_ptr: *const u32) -> u32;
    pub(crate) fn get_transaction_to_address(to_address_ptr_ptr: *const u32) -> u32;
    pub(crate) fn get_transaction_value(value_ptr_ptr: *const u32) -> u32;
    pub(crate) fn get_transaction_nonce(nonce_ptr_ptr: *const u32) -> u32;
    pub(crate) fn get_transaction_hash(hash_ptr_ptr: *const u32) -> u32;
    pub(crate) fn get_transaction_data(data_ptr_ptr: *const u32) -> u32;

    // Getters for Blockchain-related data.
    pub(crate) fn get_blockchain_height(height_ptr_ptr: *const u32) -> u32;
    pub(crate) fn get_blockchain_prev_hash(prev_hash_ptr_ptr: *const u32) -> u32;
    pub(crate) fn get_blockchain_timestamp(timestamp_ptr_ptr: *const u32) -> u32;
    pub(crate) fn get_blockchain_random_bytes(random_bytes_ptr_ptr: *const u32) -> u32;

    pub(crate) fn emit(event_ptr: *const u8, event_len: u32);

    pub(crate) fn return_value(value_ptr: *const u8, value_len: u32);

    pub(crate) fn call_action(address_ptr: *const u8, call_data_ptr: *const u8, call_data_len :u32, value_ptr :*const u8, return_ptr: *const u32) -> u32;

    pub(crate) fn call_view(address_ptr: *const u8, call_data_ptr: *const u8, call_data_len :u32, return_ptr: *const u32) -> u32;

    pub(crate) fn pay(address_ptr: *const u8, value_ptr : *const u8) -> u64;

    ////////////////////////////////////
    // Precompiles
    ////////////////////////////////////

    pub(crate) fn random() -> u64;

    pub(crate) fn sha256(key_ptr: *const u8, key_len: u32, val_ptr_ptr: *const u32) -> u32;

    pub(crate) fn keccak256(key_ptr: *const u8, key_len: u32, val_ptr_ptr: *const u32) -> u32;

    pub(crate) fn keccak512(key_ptr: *const u8, key_len: u32, val_ptr_ptr: *const u32) -> u32;

    pub(crate) fn ripemd160(key_ptr: *const u8, key_len: u32, val_ptr_ptr: *const u32) -> u32;

    pub(crate) fn blake2b(key_ptr: *const u8, key_len: u32, return_len: u32, val_ptr_ptr: *const u32) -> u32;

    pub(crate) fn verify_signature(input_ptr: *const u8, input_len: u32, signature_ptr: *const u8, address_ptr: *const u8) -> i32;

}

