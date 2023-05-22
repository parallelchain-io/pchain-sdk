/*
    Copyright © 2023, ParallelChain Lab 
    Licensed under the Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
*/

//! Defines the signatures of the externally-defined functions that Contract WASM modules expect 
//! to be linked to the WASM runtime during module instantiation. The definitions (function bodies) of these functions
//! should follow a version of Contract Binary Interface.

extern "C" {
    // Account State Accessors
    pub(crate) fn set(key_ptr: *const u8, key_len: u32, value_ptr: *const u8, value_len: u32);
    pub(crate) fn get(key_ptr: *const u8, key_len: u32, value_ptr_ptr: *const u32) -> i64;
    pub(crate) fn get_network_storage(key_ptr: *const u8, key_len: u32, value_ptr_ptr: *const u32) -> i64;
    pub(crate) fn balance() -> u64;

    // Block Field Getters
    pub(crate) fn block_height() -> u64;
    pub(crate) fn block_timestamp() -> u32;
    pub(crate) fn prev_block_hash(hash_ptr_ptr: *const u32);

    // Call Context Getters
    pub(crate) fn calling_account(address_ptr_ptr: *const u32);
    pub(crate) fn current_account(address_ptr_ptr: *const u32);
    pub(crate) fn method(method_ptr_ptr: *const u32) -> u32;
    pub(crate) fn arguments(arguments_ptr_ptr: *const u32) -> u32;
    pub(crate) fn amount() -> u64;
    pub(crate) fn is_internal_call() -> i32;
    pub(crate) fn transaction_hash(hash_ptr_ptr: *const u32);

    // Internal Call Triggers
    pub(crate) fn call(call_input_ptr: *const u8, call_input_len: u32, rval_ptr_ptr: *const u32) -> u32;
    pub(crate) fn return_value(return_val_ptr: *const u8, return_val_len: u32);
    pub(crate) fn transfer(transfer_input_ptr: *const u8);

    // Network Command Triggers
    pub(crate) fn defer_create_deposit(create_deposit_input_ptr: *const u8, create_deposit_input_len: u32);
    pub(crate) fn defer_set_deposit_settings(set_deposit_settings_input_ptr: *const u8, set_deposit_settings_input_len: u32);
    pub(crate) fn defer_topup_deposit(top_up_deposit_input_ptr: *const u8, top_up_deposit_input_len: u32);
    pub(crate) fn defer_withdraw_deposit(withdraw_deposit_input_ptr: *const u8, withdraw_deposit_input_len: u32);
    pub(crate) fn defer_stake_deposit(stake_deposit_input_ptr: *const u8, stake_deposit_input_len: u32);
    pub(crate) fn defer_unstake_deposit(unstake_deposit_input_ptr: *const u8, unstake_deposit_input_len: u32);

    // Logging
    pub(crate) fn _log(log_ptr: *const u8, log_len: u32);

    // Cryptographic operations
    pub(crate) fn sha256(msg_ptr: *const u8, msg_len: u32, digest_ptr_ptr: *const u32);
    pub(crate) fn keccak256(msg_ptr: *const u8, msg_len: u32, digest_ptr_ptr: *const u32);
    pub(crate) fn ripemd(msg_ptr: *const u8, msg_len: u32, digest_ptr_ptr: *const u32);
    pub(crate) fn verify_ed25519_signature(msg_ptr: *const u8, msg_len: u32, signature_ptr: *const u8, address_ptr: *const u8) -> i32;

}

