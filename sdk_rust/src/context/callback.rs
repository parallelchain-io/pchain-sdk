use crate::Transaction;
use crate::{convert_bytes};

use borsh::{BorshSerialize, BorshDeserialize};

/// A struct to store return value from an entrypoint
#[derive(BorshSerialize, BorshDeserialize)]
pub struct Callback {
    return_val :Vec<u8>
}

impl Default for Callback {
    fn default() -> Self {
        Self {
            return_val: "Success".as_bytes().to_vec(),
        }
    }
}

impl Callback {

    /// Convert Borsh-serializable structure to bytes
    pub fn from<T: BorshSerialize>(result :&T) -> Self{
        Self {
            return_val: convert_bytes(result)
        }
    }

    /// Convert Borsh-serialized bytes to Callback and then return the field `return_val` as Option of bytes
    pub fn from_callback(bytes : Vec<u8>) -> Option<Vec<u8>> {
        if let Ok(Callback { return_val, .. }) = BorshDeserialize::deserialize(&mut bytes.as_slice()) {
            Some(return_val)
        } else {
            None
        }
    }

    /// Serialize the field `return_val` into bytes and save to transaction as return value
    pub fn return_value(&self) {
        let mut bs : Vec<u8> = vec![];
        self.serialize(&mut bs).unwrap();
        Transaction::return_value(bs.to_vec());
    }
}