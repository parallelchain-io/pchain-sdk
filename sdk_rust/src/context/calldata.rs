use crate::{Transaction, convert_bytes};

use borsh::{BorshSerialize, BorshDeserialize};
use protocol_types::{CallData, Serializable, Deserializable};

static CALLDATA_VERSION :u32 = 0;

/// wrapper struct got protocol_types::CallData
pub struct ContractCallData {
    call_data :CallData
}

impl  ContractCallData {

    /// get method_name as &str from call_data
    pub fn get_method_name(&self) -> &str {
        self.call_data.method_name.as_str()
    }

    /// The field `arguments` in CallData defines inputs arguments to the function with name which is defined in the field `entrypoint`
    /// The function here converts `arguments` to Vec<Vec<u8>> so that it can be parsed to specific data type for the entrypoint function.
    /// 
    /// ### Example
    /// ```no_run
    /// fn method_1(data :i32, name :String) { ...
    /// ```
    /// If `arguments` represents the above function, the output is a vector = {"data", "name"} 
    /// where "data" and "name" are Borsh-Serialized bytes
    pub fn get_multiple_arguments(&self) -> Vec<Vec<u8>> {
        let mut args = self.call_data.arguments.as_slice();
        let args_bs :Vec<Vec<u8>> = 
        if let Ok(args_bytes) = BorshDeserialize::deserialize(&mut args){
            args_bytes
        } else { vec![] };
        args_bs
    }

    /// Parser function to deserialize indexed argument into defined data type
    pub fn parse_multiple_arguments<T: BorshDeserialize>(args: &Vec<Vec<u8>>, idx: usize) -> T {
        let bs = args[idx].clone();
        if let Ok(x) = BorshDeserialize::deserialize(&mut bs.as_ref()) {
            x
        } else {
            panic!()
        }
    }

    /// Convert the field `data` in the transaction into CallData.
    /// Valid CallData should follow below requirements
    /// - first 4 bytes are version bytes which matches CALLDATA_VERSION
    /// - if it is less than 4 bytes, it assumes version = 0 and CallData is returned with "empty" data
    /// - return None if version does not match with CALLDATA_VERSION
    /// - the rest of the bytes are borsh-serialized from the structure CallData
    pub fn from_raw_call_data() -> Option<ContractCallData> {
        let bs = Transaction::get_arguments();
        let version = if bs.len() < 4 { 
            0
        } else { 
            let mut version_bs = [0u8; 4];
            version_bs.clone_from_slice(&bs[0..4]);
            u32::from_le_bytes(version_bs)
        };

        if version != CALLDATA_VERSION {
           return None; // unable to parse the input data
        }

        // Version correct but no function name/arguments. 
        // Assume the caller indicates invoking the contract without entrypoint selection.
        if bs.len() <= 4 {
            return Some(
                ContractCallData{
                    call_data: CallData{
                        method_name : "".to_string(),
                        arguments: vec![]
                    }
            });
        }
        
        let bs = &bs[4..];
        let deserialize_ret = CallData::deserialize(bs.as_ref());
        if !deserialize_ret.is_ok() { return None; }
        let ctx :CallData = deserialize_ret.unwrap();
        Some(ContractCallData {call_data: ctx})
    }

    /// contrust CallData structure for making contract calls.
    /// It returns raw bytes that passed into the export functions
    pub fn to_raw_call_data(method_name :&str, args :Vec<u8>) -> Vec<u8> {
        if method_name.is_empty() {
            return args; // contract without multiple entrypoints
        }
        let mut bs :Vec<u8>= vec![];
        let mut version_bs = (CALLDATA_VERSION as i32).to_le_bytes().to_vec();
        bs.append(&mut version_bs);
    
        let ctx = CallData{
            method_name: method_name.to_string(),
            arguments: args
        };
        let mut call_data_bs = CallData::serialize(&ctx);
        bs.append(&mut call_data_bs);
        bs
    }
}



/// Builder to contruct CallData::arguments so that it can be passed to entrypoint
/// 
/// 
/// # Basic example 
/// ```no_run
/// let args_builder = smart_contract::CallDataBuilder::new();
/// args_builder
/// .add("i am string".to_string())
/// .add(0_i32)
/// .add(vec![0u8; 8]);
/// 
/// // construct Vec<u8> data to pass to CallData::arguments 
/// let args :Vec<u8> = args_builder.to_call_arguments();
/// ...
/// ```
pub struct CallDataBuilder {
    pub args :Vec<Vec<u8>>
}
impl CallDataBuilder {
    pub fn new() -> Self {
        Self { args: vec![] }
    }
    pub fn add<T: BorshSerialize>(&mut self,  arg :T) -> &mut Self{
        let mut args_bs:Vec<u8> = vec![]; 
        arg.serialize(&mut args_bs).unwrap();
        self.args.push(args_bs.clone());
        self
    }
    pub fn to_call_arguments(&self) -> Vec<u8> {
        convert_bytes(&self.args)
    }
}