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

use borsh::{BorshSerialize, BorshDeserialize};
use pchain_types::{CallData as ProtocolCallData, Serializable, Deserializable};
use crate::transaction;

static CALLDATA_VERSION :u32 = 0;

/// wrapper struct got pchain_types::CallData
pub struct CallData {
    call_data: ProtocolCallData,
}

impl CallData {
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
    pub fn from_raw_call_data() -> Option<CallData> {
        let bs = transaction::data();
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
                CallData{
                    call_data: ProtocolCallData{
                        method_name : "".to_string(),
                        arguments: vec![]
                    }
            });
        }
        
        let bs = &bs[4..];
        let deserialize_ret = <ProtocolCallData as Deserializable<ProtocolCallData>>::deserialize(bs.as_ref());
        if !deserialize_ret.is_ok() { return None; }
        let ctx: ProtocolCallData = deserialize_ret.unwrap();
        Some(CallData {call_data: ctx})
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
    
        let ctx = ProtocolCallData {
            method_name: method_name.to_string(),
            arguments: args
        };
        let mut call_data_bs = <ProtocolCallData as Serializable<ProtocolCallData>>::serialize(&ctx);
        bs.append(&mut call_data_bs);
        bs
    }
}



/// Builder to contruct CallData::arguments so that it can be passed to entrypoint
/// 
/// 
/// # Basic example 
/// ```no_run
/// let args_builder = pchain_sdk::CallDataBuilder::new();
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
        Vec::<Vec<u8>>::try_to_vec(&self.args).unwrap()
    }
}