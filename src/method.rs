/*
    Copyright © 2023, ParallelChain Lab 
    Licensed under the Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
*/

//! Defines the abstract level of inputs and outputs of a contract method.
//! Contracts written use the `#[contract]` macro to unpack those Transaction Command `Call` 
//! that contain a serialized `arguments`. This structure selects the Method 
//! that a call should enter, as well as provides the arguments for the function call. 
//! 
//! The structure of output of a contract method wraps the return value in Internal Calls.
//!
//! Developers typically do not need to care about the contents of this module. The `#[contract]` macro handles
//! the creation and consumption of ContractMethodInput in expanded code. 

use borsh::{BorshSerialize, BorshDeserialize};

use crate::transaction;

/// Input of a contract method in a call, which consists of method name and its borsh-serialized arguments.
pub struct ContractMethodInput {
    pub method_name: String,
    pub arguments: Vec<u8>,
}

impl ContractMethodInput {
    /// Convert the fields in the transaction Call command
    pub fn from_transaction() -> Self {
        Self{
            method_name: transaction::method(),
            arguments: transaction::arguments()
        }
    }
    /// Get method_name as &str from Call command.
    pub fn method_name(&self) -> &str {
        self.method_name.as_str()
    }

    /// Converts `arguments` in Call command  to `Vec<Vec<u8>>` so that it can be parsed to specific data type for the entrypoint function.
    pub fn get_multiple_arguments(&self) -> Vec<Vec<u8>> {
        let mut args = self.arguments.as_slice();
        BorshDeserialize::deserialize(&mut args).unwrap()
    }

    /// Parser function to deserialize indexed argument into defined data type
    pub fn parse_multiple_arguments<T: BorshDeserialize>(args: &[Vec<u8>], idx: usize) -> T {
        let bs = args[idx].clone();
        BorshDeserialize::deserialize(&mut bs.as_ref()).unwrap()
    }
}



/// Builder to contruct arguments in Call command so that it can be passed to entrypoint
/// 
/// 
/// # Basic example 
/// ```no_run
/// let args_builder = pchain_sdk::ContractMethodInputBuilder::new();
/// args_builder
/// .add("i am string".to_string())
/// .add(0_i32)
/// .add(vec![0u8; 8]);
/// 
/// // construct Vec<u8> data to pass to call arguments 
/// let args :Vec<u8> = args_builder.to_call_arguments();
/// ...
/// ```
pub struct ContractMethodInputBuilder {
    pub args :Vec<Vec<u8>>
}
impl ContractMethodInputBuilder {
    pub fn new() -> Self {
        Self { args: vec![] }
    }
    pub fn add<T: BorshSerialize>(&mut self,  arg :T) -> &mut Self{
        self.args.push(arg.try_to_vec().unwrap());
        self
    }
    pub fn to_call_arguments(&self) -> Vec<u8> {
        // use pchain_types for serialization for consistency with runtime
        <Vec<Vec<u8>> as pchain_types::serialization::Serializable>::serialize(&self.args)
    }
}


/// Encapsulates the return value as serialized bytes from contract method. 
/// None if the contract method does not specify return value.
#[derive(BorshSerialize, BorshDeserialize, Default)]
pub struct ContractMethodOutput(Option<Vec<u8>>);

impl ContractMethodOutput {
    pub fn set<T: BorshSerialize>(result :&T) -> Self {
        Self(Some(T::try_to_vec(result).unwrap()))
    }

    pub fn get(self) -> Option<Vec<u8>> {
        self.0
    }
}