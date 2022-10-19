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

/// A struct to store return value from an entrypoint
#[derive(BorshSerialize, BorshDeserialize)]
pub struct CallResult(Option<Vec<u8>>);

impl Default for CallResult {
    fn default() -> Self {
        Self (None)
    }
}

impl CallResult {
    pub fn set<T: BorshSerialize>(result :&T) -> Self {
        Self(Some(T::try_to_vec(result).unwrap()))
    }

    pub fn get(&self) -> Option<Vec<u8>> {
        self.0.clone()
    }

    pub fn to_vec(&self) -> Vec<u8> {
        CallResult::try_to_vec(self).unwrap()
    }
}