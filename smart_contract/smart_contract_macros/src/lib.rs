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


extern crate proc_macro;
use proc_macro::TokenStream;
use syn::{ItemFn, ItemStruct};


mod core_impl;
use self::core_impl::*;


/// `sdk_method_bindgen` provides "convenience methods" to interact with the world 
/// state using custom data types (structs). 
/// 
/// These convenience methods are explicitly known as "typed methods" from the SDK.
/// 
/// This macro expects the `trait: BorshSerialize/BorshDeserialize` trait to be 
/// implemented on the struct. See <https://docs.rs/borsh/0.9.3/borsh/index.html> 
/// on how to implement the traits above.
/// 
/// Examples are available at `smart_contract_sdks/examples`
/// 
/// # Basic example 
/// ```no_run
/// // add in the BorshSerialize/BorshDeserialize macros to 
/// // automatically implement the traits required for the
/// // `sdk_method_bindgen` macro.
/// #[derive(BorshSerialize, BorshDeserialize)]
/// #[sdk_method_bindgen]
/// struct MyArgument {
///   first_argument: String,
///   second_argument: u32,
/// }
/// // the code illustrates the "typed methods" available to the 
/// // smart contract developer after this macro is used 
/// #[contract_init]
/// pub fn contract() {
/// 
///   let tx = smart_contract::Transaction::<MyArgument>::new();
/// 
///   //initialize MyArgument
///   let my_argument = MyArgument { /.. / }; 
/// 
///   // This is the "typed_set" method to bind a key to a value that is of a 
///   // custom data type. See the `set` method of the SDK for more information. 
///   // Note: the naming convention of this "typed_method" is in snake case.
///   // It takes in a byte strting as the key and the custom data type itself as 
///   // its value.
///   tx.set_my_argument(key, value);
///   
///   // This is the "typed_get" method get a custom data type from the
///   // world state. Some(value) is returned if a non-empty value
///   // in the world state. See the `get` method of the SDK for more 
///   // information. Note: the naming convention of this "typed_method" 
///   // is in snake case. It takes in a byte strting as the key and the 
///   // custom data type itself as its value.
///   tx.get_my_argument(key);
/// 
/// }
/// ```
#[proc_macro_attribute]
pub fn sdk_method_bindgen(_attr_args: TokenStream, input: TokenStream) -> TokenStream {   

  if let Ok(istruct) = syn::parse::<ItemStruct>(input.clone()) {
    generate_sdk_typed_methods(&istruct)
  }  else {
    let result_message= "ERROR: sdk_method_bindgen can only be applied on struct types
    that is a generic parameter (A) to the sdk (smart_contract::Transaction<A>)";
    generate_compilation_error(result_message.to_string())
  }

}


/// `contract_init` macro transforms idiomatic rust smart contracts into contracts
/// that are readable and deployable by ParallelChain Mainnet Fullnode.
/// 
/// This macro expects the `trait: BorshSerialize/BorshDeserialize` trait to be 
/// implemented on the struct. See <https://docs.rs/borsh/0.9.3/borsh/index.html> 
/// on how to implement the traits above.
/// 
/// Examples are available at `smart_contract_sdks/examples`
/// 
/// # Basic example 
/// ```no_run
/// // `contract_init` must be used if you intend to write
/// // the smart contract in idiomatic rust code.
/// //
/// // The return value of the entrypoint function `Result<u32>` will be emitted
/// // as protocol_types::Transaction::Receipts.
/// #[contract_init]
/// pub fn contract(tx: Transaction<MyArgument>) -> Result<u32> {
/// 
///   //initialize MyArgument
///   let my_argument = MyArgument { 
///     first_argument: String::from("Hello ParallelChain"),
///     second_argument: 555,
///   }; 
/// 
///   Ok(my_argument.second_argument)
/// }
/// ```
#[proc_macro_attribute]
pub fn contract_init(_attr_args: TokenStream, input: TokenStream) -> TokenStream {   

  if let Ok(mut ifn) = syn::parse::<ItemFn>(input.clone()) {
    transform_contract_entrypoint(&mut ifn)    
  } else {
      let result_message= "ERROR: entrypoint_bindgen can only 
      be applied to smart contract contract() entrypoints.";
      generate_compilation_error(result_message.to_string())
  }

}