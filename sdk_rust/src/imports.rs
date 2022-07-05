/*
 Copyright (c) 2022 ParallelChain Lab

 This program is free software: you can redistribute it and/or modify
 it under the terms of the GNU General Public License as published by
 the Free Software Foundation, either version 3 of the License, or
 (at your option) any later version.

 This program is distributed in the hope that it will be useful,
 but WITHOUT ANY WARRANTY; without even the implied warranty of
 MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 GNU General Public License for more details.

 You should have received a copy of the GNU General Public License
 along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

extern "C" {
    // If set was unsuccessful, the WASM instance will be terminated
    // and changes rolled back.
    pub(crate) fn raw_set(key_ptr: *const u8, key_len: u32, val_ptr: *const u8, val_len: u32);

    // because WASM doesn't yet support multiple return values, we
    // pass back a pointer to the beginning of the gotten value by
    // writing on val_ptr.
    //
    // If get was unsuccessful, the WASM instance will be terminated
    // and changes rolled back.
    pub(crate) fn raw_get(key_ptr: *const u8, key_len: u32, val_ptr_ptr: *const u32) -> u32;
    
    pub(crate) fn raw_get_params_from_transaction(params_from_transaction_ptr_ptr: *const u32) -> u32;

    pub(crate) fn raw_get_params_from_blockchain(params_from_blockchain_ptr_ptr: *const u32) -> u32;

    pub(crate) fn raw_emit(event_ptr: *const u8, event_len: u32);

    pub(crate) fn raw_return(value_ptr: *const u8, value_len: u32);

    pub(crate) fn raw_get_arguments(val_ptr_ptr: *const u32) -> u32;

    pub(crate) fn raw_call(address_ptr: *const u8, call_data_ptr: *const u8, call_data_len :u32, value_ptr :*const u8, gas_ptr :*const u8, return_ptr: *const u32) -> u32;

    pub(crate) fn raw_view(address_ptr: *const u8, call_data_ptr: *const u8, call_data_len :u32, return_ptr: *const u32) -> u32;

    pub(crate) fn raw_pay(address_ptr: *const u8, value_ptr : *const u8) -> u64;

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

