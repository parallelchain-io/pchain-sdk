/*
    Copyright © 2023, ParallelChain Lab 
    Licensed under the Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
*/

//! Defines functions to defer a network command that to be executed after success of this contract call.
//! This command is considered as part of the contract call. 
//! i.e. There is no modification on the field commands in the Transaction, and no additional Command Receipt for it.

use pchain_types::{blockchain::Command, serialization::Serializable, cryptography::PublicAddress, runtime::{CreateDepositInput, SetDepositSettingsInput, TopUpDepositInput, WithdrawDepositInput, StakeDepositInput, UnstakeDepositInput}};

use crate::imports;

/// Instantiation of Deposit in state.
/// This execution is deferred to be executed after success of this contract call.
pub fn defer_create_deposit(
    operator: PublicAddress,
    balance: u64,
    auto_stake_rewards: bool,
) {
    let command = Command::CreateDeposit(CreateDepositInput{ operator, balance, auto_stake_rewards }).serialize();
    let command_ptr = command.as_ptr();
    let command_len = command.len() as u32;
    unsafe { imports:: defer_create_deposit(command_ptr, command_len) }
}

/// Update settings of an existing Deposit.
/// This execution is deferred to be executed after success of this contract call.
pub fn defer_set_deposit_settings(
    operator: PublicAddress,
    auto_stake_rewards: bool,
) {
    let command = Command::SetDepositSettings( SetDepositSettingsInput{ operator, auto_stake_rewards }).serialize();
    let command_ptr = command.as_ptr();
    let command_len = command.len() as u32;
    unsafe { imports:: defer_set_deposit_settings(command_ptr, command_len) }
}

/// Increase balance of an existing Deposit.
/// This execution is deferred to be executed after success of this contract call.
pub fn defer_topup_deposit(
    operator: PublicAddress,
    amount: u64,
) {
    let command = Command::TopUpDeposit( TopUpDepositInput{ operator, amount }).serialize();
    let command_ptr = command.as_ptr();
    let command_len = command.len() as u32;
    unsafe { imports:: defer_topup_deposit(command_ptr, command_len) }
}

/// Withdraw balance from an existing Deposit.
/// This execution is deferred to be executed after success of this contract call.
pub fn defer_withdraw_deposit(
    operator: PublicAddress,
    max_amount: u64,
) {
    let command = Command::WithdrawDeposit( WithdrawDepositInput{ operator, max_amount }).serialize();
    let command_ptr = command.as_ptr();
    let command_len = command.len() as u32;
    unsafe { imports:: defer_withdraw_deposit(command_ptr, command_len) }
}

/// Increase stakes to an existing Pool
/// This execution is deferred to be executed after success of this contract call.
pub fn defer_stake_deposit(
    operator: PublicAddress,
    max_amount: u64,
) {
    let command = Command::StakeDeposit( StakeDepositInput{ operator, max_amount }).serialize();
    let command_ptr = command.as_ptr();
    let command_len = command.len() as u32;
    unsafe { imports:: defer_stake_deposit(command_ptr, command_len) }
}

/// Remove stakes from an existing Pool.
/// This execution is deferred to be executed after success of this contract call.
pub fn defer_unstake_deposit(
    operator: PublicAddress,
    max_amount: u64,
) {
    let command = Command::UnstakeDeposit( UnstakeDepositInput{ operator, max_amount }).serialize();
    let command_ptr = command.as_ptr();
    let command_len = command.len() as u32;
    unsafe { imports:: defer_unstake_deposit(command_ptr, command_len) }
}