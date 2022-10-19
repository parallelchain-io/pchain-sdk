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

extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemStruct, ItemImpl, NestedMeta, ItemTrait, ImplItemMethod};


mod core_impl;
use self::core_impl::*;


/// 
/// Please note that `contract` cannot be used with `contract_init`
/// 
/// Examples are available at `smart_contract/examples`
/// 
/// # Basic example 
/// Define fields in struct as contract storage. Define methods in impl as entrypoints
/// 
/// ```no_run
/// #[contract]
/// struct MyContract {
///   data :i32
/// }
/// 
/// #[contract]
/// impl MyContract {
///   pub fn callable_function_a() {
///     ...
///   }
///   pub fn callable_function_b(input :i32) -> String {
///     ...
///   }
/// }
/// ```
/// # Example
/// Add attribute "meta" for exposing available entrypoints of the contract.
/// 
/// ```no_run
/// #[contract_methods(meta)]
/// impl MyContract{
///     ...
/// }
/// ```
#[proc_macro_attribute]
pub fn contract(_attr_args: TokenStream, input: TokenStream) -> TokenStream {

  if let Ok(mut ist) = syn::parse::<ItemStruct>(input.clone()) {
    generate_contract_struct(&mut ist)
  } else {
    generate_compilation_error("ERROR:  contract macro can only be applied to smart contract Struct to read/write into world state".to_string())
  }
}
#[proc_macro_attribute]
pub fn contract_methods(_attr_args: TokenStream, input: TokenStream) -> TokenStream {
  if let Ok(mut ipl) = syn::parse::<ItemImpl>(input.clone()) {
    let attr_args_string = _attr_args.to_string();
    let attributes = attr_args_string.split(",").collect::<Vec<&str>>();
    generate_contract_impl(&mut ipl, attributes.contains(&"meta"))
  } else {
    generate_compilation_error("ERROR: contract_methods macro can only be applied to smart contract implStruct/implTrait to generate actions(), views(), init() entrypoints.".to_string())
  }
}


/// `use_contract` provides smart contract developers a way to make cross contract calls by using
/// public functions from other smart contracts in the form of traits. 
/// 
/// Examples are available at `smart_contract/examples`
/// 
/// # To use this macro
/// ```no_run
/// // The argument to `use_contract` is the address of the external smart contract to be called.
/// // As rust enforces a unique name for each trait defined, it is important that the external 
/// // contract address is fed into `use_contract`. The trait name can therefore be anything. 
/// // However it is recommended to use a name similar to the external smart contract to be called.
/// #[use_contract("Ns9DuNe8aS5QISfCyjEoAcZq20OVr2nKQTKsYGmo/Jw=")]
/// pub trait MyContract {
///   fn print_a_value();
///   fn get_commodities_price(item: String) -> u64;
/// }
/// 
/// 
/// // .. MyContract struct definition .. //
/// 
/// #[contract]
/// impl MyContract {
///   pub fn callable_function_a() {
///     let gas: u64 = 100;
///     let value: u64: 500;
/// 
///     // The functions from `MyContract` can now be called as associated functions. However you access 
///     //`MyContract` as `snake_case` instead of `CamelCase` as shown in the example. 
///     my_contract::print_a_value(100, 500);
///     my_contract::get_commodities_price("sugar".to_string(),100,500);
///   }
/// }
/// ```
/// The available functions can be used anywhere at the crate level by the smart contract developer. As an example
/// ```no_run
/// // In external_call.rs
/// use pchain_sdk::use_contract;
/// 
/// #[use_contract("Ns9DuNe8aS5QISfCyjEoAcZq20OVr2nKQTKsYGmo/Jw=")]
/// pub trait MyContract {
///   fn print_a_value();
///   fn get_commodities_price(item: String) -> u64;
/// }
/// ```
/// 
/// ```no_run
/// // In lib.rs
/// 
/// pub mod external_call;
/// use external_call::my_contract;
///
/// #[contract]
/// impl MyContract {
///   pub fn callable_function_a() {
///     ...
///     my_contract::print_a_value(100, 500);
///     my_contract::get_commodities_price("sugar".to_string(),100,500);
///     ...
///   }
/// }
/// ```
#[proc_macro_attribute]
pub fn use_contract(attr_args: TokenStream, input: TokenStream) -> TokenStream {  

  let attr_args = syn::parse_macro_input!(attr_args as syn::AttributeArgs);
  if attr_args.len() < 1 || attr_args.len() > 2 {
    return generate_compilation_error("At least one argument is required. Expect first argument to be a contract address. Second argument (Optional) to be 'action' or 'view'.".to_string());
  }

  match syn::parse::<ItemTrait>(input) {
    Ok(it) => {
      // `attr_args[0]` is the contract address of the external contract to be called.
      let attr_contract_address = &attr_args[0];
      let contract_address = match attr_contract_address {
            NestedMeta::Lit(syn::Lit::Str(s)) => s.value(),
            NestedMeta::Lit(_) | NestedMeta::Meta(_) => {
              return generate_compilation_error("Only &str are allowed as first argument to use_contract".to_string())
            },
      };
      // `attr_args[1]` (optional) determines whether this trait contains action methods or view methods. 
      // By default (not specifying it), trait contains action methods.
      let method_type = if attr_args.len()==1 { UseContractMethodType::Action } // default action methods
      else {
        match &attr_args[1] {
          NestedMeta::Meta(meta) => {
            match meta.path().get_ident().unwrap().to_string().as_str() {
              "action" => UseContractMethodType::Action,
              "view" => UseContractMethodType::View,
              _=> return generate_compilation_error("The second argument should be either action or view.".to_string())
            }
          }
          _ => return generate_compilation_error("The second argument is not recognised.".to_string())
        }
      };

      generate_external_contract_mod(it, contract_address, method_type)
    },
    Err(_) => {
      generate_compilation_error("use_contract can only be applied to trait definitions.".to_string())
    },
  }

}

/// The macro `contract_field` can generate impl so that nested struct can be supported in contract struct.
/// 
/// ### Example
/// ```no_run
/// #[contract_field]
/// struct MyField {
///     data: u64
/// }
/// 
/// #[contract]
/// struct MyContract {
///     my_field :MyField
/// }
/// ```
/// In the above example, the key used for storing in world-state will be "MyContract/my_field/data" 
/// while the value stored in world-state will be borse-serialized u64 data.
/// 
#[proc_macro_attribute]
pub fn contract_field(_attr_args: TokenStream, input: TokenStream) -> TokenStream {
  if let Ok(mut ist) = syn::parse::<ItemStruct>(input.clone()) {
    let contract_field_struct = ist.clone();
    let struct_impls:proc_macro2::TokenStream = generate_storage_impl(&mut ist).into();
    
    TokenStream::from(
      quote!{
        #contract_field_struct

        #struct_impls
      }
    )
  } else {
    generate_compilation_error("#[contract_field] can only be applied to struct definitions.".to_string())
  }
}

/// `view` macro applies to impl methods for read-only contract call. The below operations will take no effect after execution
/// 
/// - set data to storage
/// - emit events
/// - cross-contract call
/// - internal transaction
///  
/// ### Example
/// ```no_run
/// #[view]
/// pub fn view_method(d1: i32) -> String { ..
/// ```
#[proc_macro_attribute]
pub fn view(_attr_args: TokenStream, input: TokenStream) -> TokenStream {
  match syn::parse::<ImplItemMethod>(input.clone()) {
    Ok(_) => {input},
    _=> generate_compilation_error("view can only be applied to impl methods.".to_string())
  }
}

/// `action` macro applies to impl methods for mutable contract call.
/// 
/// ### Example
/// ```no_run
/// #[action]
/// pub fn action_method(d1: i32) -> String{ ..
/// ```
#[proc_macro_attribute]
pub fn action(_attr_args: TokenStream, input: TokenStream) -> TokenStream {
  // it does nothing. The macro contract will handle this attribure.
  input
}

/// `init` macro applies to impl methods for init entrypoint
/// 
/// ### Example
/// ```no_run
/// #[init]
/// pub fn init_method(d1: i32) -> String { ..
/// ```
#[proc_macro_attribute]
pub fn init(_attr_args: TokenStream, input: TokenStream) -> TokenStream {
  // it does nothing. The macro contract will handle this attribure.
  input
}