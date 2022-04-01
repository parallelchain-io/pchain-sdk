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

use crate::imports::*;
use protocol_types::sc_params::Deserializable as DeserializableSCParams;
use protocol_types::transaction::Deserializable as DeserializableTransaction;
use borsh::de::BorshDeserialize;
use protocol_types::transaction::Serializable;

/// smart_contract::Transaction is a handle containing the parameters of the smart contract invocation
/// (e.g., the 'args' string provided by the contract_caller, the previous block hash, etc.)
///
/// It also has methods attached ('set' & 'get') that allow smart contracts to maintain
/// persistent, blockchained state.
///
/// From this point on, smart_contract::Transaction can be interchangeably known as
/// "ParallelChain Mainnet Smart Contract Development Kit" or the "SDK" or "SC-SDK".
/// 
/// # Basic example 
/// ```no_run
/// // where A: primitive type or custom data type that implements the BorshSerializable
/// // trait.
/// let tx: smart_contract::Transaction<A> = Transaction::<A>::new();
/// 
/// assert!(tx.get("hello").is_none());
/// 
/// tx.Set("hello", "world");
/// assert_eq!(tx.get("hello")?, "world");
///
/// tx.Set("hello", "");
/// assert_eq!(tx.get("hello")?, "");
/// ```

pub struct Transaction<A> {
    pub this_block_number: u64,
    pub prev_block_hash: protocol_types::crypto::Sha256Hash,
    pub timestamp: u32,
    pub random_bytes: protocol_types::crypto::Sha256Hash,
    pub to_address: protocol_types::crypto::PublicAddress,
    pub from_address: protocol_types::crypto::PublicAddress,
    pub value: u64,
    pub transaction_hash: protocol_types::crypto::Sha256Hash,
    pub arguments: A,
}

impl<A: BorshDeserialize> Transaction<A> {
    /// Default constructor.
    /// 
    /// `new` should never fail if the ParallelChain Mainnet Fullnode
    /// is configured properly.
    /// 
    /// `new` expects arguments (A) in the form of either primitive types
    /// or custom data types to be fed into the smart contract.
    pub fn new() -> Self {

        let params_from_transaction = Self::parse_params_from_transaction();
        let params_from_blockchain = Self::parse_params_from_blockchain();
                
        Self {
            this_block_number: params_from_blockchain.this_block_number,
            prev_block_hash: params_from_blockchain.prev_block_hash,
            timestamp: params_from_blockchain.timestamp,
            random_bytes: params_from_blockchain.random_bytes,
            to_address: params_from_transaction.to_address,
            from_address: params_from_transaction.from_address,
            value: params_from_transaction.value,
            transaction_hash: params_from_transaction.transaction_hash,
            arguments: BorshDeserialize::deserialize(&mut params_from_transaction.data.as_ref()).unwrap(),
        }
    }

    /// get returns Some(value) if a non-empty string is stored with key in the world state. 
    ///
    /// If get fails, the smart contract terminates and the sets this invocation made
    /// are not committed.
    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {

        let key_ptr = key.as_ptr();

        // `get` needs to get two things from `raw_get`:
        //   * A WASM linear memory offset pointing to where the host process
        //     wrote the UTF-8 encoded result of the DB get: `val_ptr`.
        //   * The byte-wise length of the gotten value: `val_len`.
        // 
        // `val_len` is returned directly, but the problem is that WASM does not
        // yet support multiple return values, so we can't return `val_ptr` alongside it.
        // Instead, we allocate a space in the heap for the host to write `val_ptr` in,
        // and then tell the host to write `val` there by passing it `val_ptr_ptr` through
        // `raw_get`.
        //
        // When val_ptr leaves this scope, it is deallocated (we have no further use
        // for it).
        let mut val_ptr: u32 = 0;
        let val_ptr_ptr = &mut val_ptr;

        let value;
        unsafe {
            let val_len = raw_get(key_ptr, key.len() as u32, val_ptr_ptr);

            // If module execution reaches this point, we can assume that
            // the `get` has succeeded.
            //
            // This Vec<u8> takes ownership of the segment of memory, letting the Rust ownership
            // system to Drop it later.
            value = Vec::<u8>::from_raw_parts(val_ptr as *mut u8, val_len as usize, val_len as usize);
        }

        match value {
            vec if vec.is_empty() => {
                None
            },
            set_value => {
                Some(set_value)
            }
        }
    } 

    /// set binds key to value in the world state.
    pub fn set(&self, key: &[u8], value: &[u8]) {
        let key_ptr = key.as_ptr();
        let val_ptr = value.as_ptr();
        unsafe {       
            raw_set(key_ptr, key.len() as u32, val_ptr, value.len() as u32);
        } 
    }

    /// `return_value` returns any values in the smart contract
    /// back to fullnode::executor in the form of receipts.
    /// 
    /// This method is not required to be used when the
    /// `contract_init` macro is being used on the
    /// contract() entrypoint function.
    pub fn return_value(&self, value: Vec<u8>) {    
        let value_ptr = value.as_ptr();
        let value_len = value.len() as u32;
        unsafe {           
            raw_return(value_ptr, value_len);
        }
    }

    pub fn emit_event(&self, topic: &[u8], value: &[u8]) {
        let event = protocol_types::transaction::Event{ 
            topic: topic.to_vec(), 
            value: value.to_vec()
        };
        let serialized_event = protocol_types::transaction::Event::serialize(&event);

        let event_ptr= serialized_event.as_ptr();
        let event_len = serialized_event.len() as u32;

        unsafe {
            raw_emit(event_ptr, event_len);
        }
    }

    fn parse_params_from_transaction() -> protocol_types::sc_params::ParamsFromTransaction {
        let params_from_transaction_ptr: u32 = 0;
        let params_from_transaction_ptr_ptr: *const u32 = &params_from_transaction_ptr;

        let bytes;

        unsafe {
            let params_len = raw_get_params_from_transaction(params_from_transaction_ptr_ptr);
            bytes = Vec::from_raw_parts(params_from_transaction_ptr as *mut u8, params_len as usize, params_len as usize);
        }

        // SAFETY: this will not fail if fullnode serializes transaction correctly.
        let transaction = protocol_types::transaction::Transaction::deserialize(&bytes).unwrap();
        let params_from_transaction = protocol_types::sc_params::ParamsFromTransaction{
                                                            to_address: transaction.to_address,
                                                            from_address: transaction.from_address,
                                                            data: transaction.data,
                                                            value: transaction.value,
                                                            transaction_hash: transaction.hash,
                                                        };

        params_from_transaction
    }

    fn parse_params_from_blockchain() -> protocol_types::sc_params::ParamsFromBlockchain {
        let params_from_blockchain_ptr: u32 = 0;
        let params_from_blockchain_ptr_ptr: *const u32 = &params_from_blockchain_ptr;

        let bytes;

        unsafe {
            let params_len = raw_get_params_from_blockchain(params_from_blockchain_ptr_ptr);
            bytes = Vec::from_raw_parts(params_from_blockchain_ptr as *mut u8, params_len as usize, params_len as usize);
        }

        // SAFETY: this will not fail if fullnode serializes params_from_blockchain correctly.
        protocol_types::sc_params::ParamsFromBlockchain::deserialize(&bytes).unwrap()
    }

}
