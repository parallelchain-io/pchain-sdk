use crate::Callback;
use crate::context::ContractCallData;
use crate::imports::*;
use borsh::BorshDeserialize;
use protocol_types::{Serializable, Deserializable};


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
/// let tx: smart_contract::Transaction = Transaction::new();
/// 
/// assert!(tx.get("hello").is_none());
/// 
/// tx.Set("hello", "world");
/// assert_eq!(tx.get("hello")?, "world");
///
/// tx.Set("hello", "");
/// assert_eq!(tx.get("hello")?, "");
/// ```
pub struct Transaction {
    /// Block Hash of this block
    pub this_block_number: u64,
    /// Block Hash of the previous block.
    pub prev_block_hash: protocol_types::crypto::Sha256Hash,
    /// Unix timestamp
    pub timestamp: u32,
    /// Reserved data
    pub random_bytes: protocol_types::crypto::Sha256Hash,
    /// Address of a recipient.
    pub to_address: protocol_types::crypto::PublicAddress,
    /// Address of a sender.
    pub from_address: protocol_types::crypto::PublicAddress,
    /// Amount of tokens to be sent to the smart contract address.
    pub value: u64,
    /// Hash of a transaction's signature. The transaction hashes of other transactions
    /// included in a block are used to obtain the Merkle root hash of a block.
    pub transaction_hash: protocol_types::crypto::Sha256Hash,
    /// Transaction data as arguments to this contract call
    pub arguments: Vec<u8>,
}

impl Transaction {
    /// Default constructor.
    /// 
    /// `new` should never fail if the ParallelChain Mainnet Fullnode
    /// is configured properly.
    /// 
    /// `new` expects arguments in the form of Vec<u8>.
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
            arguments: params_from_transaction.data,
        }
    }

    /// get returns Some(value) if a non-empty string is stored with key in the world state. 
    ///
    /// If get fails, the smart contract terminates and the sets this invocation made
    /// are not committed.
    pub fn get(key: &[u8]) -> Option<Vec<u8>> {

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

            // If module execution reaches this point, we can assume that the `get` has succeeded.
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
    pub fn set(key: &[u8], value: &[u8]) {
        let key_ptr = key.as_ptr();
        let val_ptr = value.as_ptr();
        unsafe {       
            raw_set(key_ptr, key.len() as u32, val_ptr, value.len() as u32);
        } 
    }

    /// `return_value` places `value` in the receipt of an `ExternalToContract` transaction.
    /// This method is not required when `contract_init` macro is being used on the actions()
    /// entrypoint function.
    pub fn return_value(value: Vec<u8>) {    
        let value_ptr = value.as_ptr();
        let value_len = value.len() as u32;
        unsafe {           
            raw_return(value_ptr, value_len);
        }
    }

    /// get input arguments for entrypoint
    pub fn get_arguments() -> Vec<u8> {
        let mut args_ptr: u32 = 0;
        let args_ptr_ptr = &mut args_ptr;

        let arguments;
        unsafe {
            let args_len = raw_get_arguments(args_ptr_ptr);
            arguments = Vec::<u8>::from_raw_parts(args_ptr as *mut u8,args_len as usize, args_len as usize);
        }
        arguments
    }

    pub fn emit_event(topic: &[u8], value: &[u8]) {
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

    /// calling function which handled by blockchain executor
    /// It returns Option of Vec of bytes. Interpretation on the bytes depends on caller
    pub fn call_contract(contract_address : protocol_types::PublicAddress, method_name:&str, arguments :Vec<u8>, value :u64, gas :u64) -> Option<Vec<u8>> {
        let contract_address_ptr : *const u8 = contract_address.as_ptr();

        let value_ptr :*const u64 = &value;
        let gas_ptr :*const u64 = &gas;

        let mut val_ptr: u32 = 0;
        let val_ptr_ptr = &mut val_ptr;

        let is_multiple_methods_contract = method_name.len() > 0;

        let call_data = ContractCallData::to_raw_call_data(method_name, arguments.clone());
        let call_data_ptr :*const u8 = call_data.as_ptr();
        let call_data_len = call_data.len();

        let value;
        unsafe {
            let val_len = raw_call(contract_address_ptr, call_data_ptr, call_data_len as u32, value_ptr as *const u8, gas_ptr as *const u8, val_ptr_ptr);

            // This Vec<u8> takes ownership of the segment of memory, letting the Rust ownership
            // system to Drop it later.
            value = Vec::<u8>::from_raw_parts(val_ptr as *mut u8, val_len as usize, val_len as usize);
        }

        match value {
            vec if vec.is_empty() => {
                None
            },
            return_value => {
                // Raw call function should return the values set by tx.return_value
                // If mutiple methods is called, the return value type should be Callback
                if is_multiple_methods_contract {
                    Callback::from_callback(return_value)
                } else { // Otherwise, raw bytes should be returned (e.g. for those contracts using contract_init)
                    Some(return_value)
                }
            }
        }
    }

    /// view contract by accessing view entrypoint of the contract
    pub fn view_contract(contract_address : protocol_types::PublicAddress, method_name:&str, arguments :Vec<u8>) -> Option<Vec<u8>> {
        let contract_address_ptr : *const u8 = contract_address.as_ptr();
        let mut val_ptr: u32 = 0;
        let val_ptr_ptr = &mut val_ptr;

        let is_multiple_methods_contract = method_name.len() > 0;

        let call_data = ContractCallData::to_raw_call_data(method_name, arguments.clone());
        let call_data_ptr :*const u8 = call_data.as_ptr();
        let call_data_len = call_data.len();

        let value;
        unsafe {
            let val_len = raw_view(contract_address_ptr, call_data_ptr, call_data_len as u32, val_ptr_ptr);

            // This Vec<u8> takes ownership of the segment of memory, letting the Rust ownership
            // system to Drop it later.
            value = Vec::<u8>::from_raw_parts(val_ptr as *mut u8, val_len as usize, val_len as usize);
        }

        match value {
            vec if vec.is_empty() => {
                None
            },
            return_value => {
                // Raw function should return the values set by tx.return_value
                // If mutiple methods is called, the return value type should be Callback
                if is_multiple_methods_contract {
                    Callback::from_callback(return_value)
                } else { // Otherwise, raw bytes should be returned (e.g. for those contracts using contract_init)
                    Some(return_value)
                }
            }
        }
    }

    /// A call to contract. The caller should already know the data type of return value from the function call
    /// It returns Option of T where T is return value from the function. 
    /// If data type T is different from the actual return value type of the function, None is returned.
    pub fn call<T: BorshDeserialize>(address : protocol_types::PublicAddress, method_name:&str, arguments :Vec<u8>, value :u64, gas :u64) -> Option<T> {
        if let Some(ret)= Self::call_contract(address, method_name, arguments, value, gas) {
            let mut ret = ret.as_slice();
            match BorshDeserialize::deserialize(&mut ret) {
                Ok(e) => {
                    return Some(e);
                }
                _=>{ return None;}
            }
        }
        None
    }

    /// pay() calls the raw_pay() that 
    /// runs a ctoe call to transfer credit to another address. 
    /// Return the remaining balance of the receiver's account
    pub fn pay(address : protocol_types::PublicAddress, value : u64) -> u64 {
        let contract_address_ptr : *const u8 = address.as_ptr();
        let value_ptr :*const u64 = &value;
        unsafe {
            raw_pay(contract_address_ptr, value_ptr as *const u8)
        }
    }
}
