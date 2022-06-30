extern crate proc_macro;
use proc_macro::TokenStream;
use syn::{ItemFn, ItemStruct, ItemImpl, NestedMeta, ItemTrait, ImplItemMethod};


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
/// Examples are available at `smart_contract/examples`
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
/// pub fn actions() {
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
/// Examples are available at `smart_contract/examples`
/// 
/// # Basic example 
/// ```no_run
/// // `contract_init` must be used if you intend to write
/// // the smart contract in idiomatic rust code.
/// //
/// // The return value of the entrypoint function `Result<u32>` will be emitted
/// // as protocol_types::Transaction::Receipts.
/// #[contract_init]
/// pub fn actions(tx: Transaction<MyArgument>) -> Result<u32> {
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
      be applied to smart contract actions() entrypoints.";
      generate_compilation_error(result_message.to_string())
  }

}

/// `contract` macro transforms idiomatic rust smart contracts into contracts
/// that are readable and deployable by nodes in ParallelChain Mainnet.
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
/// #[contract(meta)]
/// impl MyContract{
///     ...
/// }
/// ```
#[proc_macro_attribute]
pub fn contract(_attr_args: TokenStream, input: TokenStream) -> TokenStream {

  if let Ok(mut ist) = syn::parse::<ItemStruct>(input.clone()) {
    generate_contract_struct(&mut ist)
  } else if let Ok(mut ipl) = syn::parse::<ItemImpl>(input.clone()) {
    let attr_args_string = _attr_args.to_string();
    let attributes = attr_args_string.split(",").collect::<Vec<&str>>();
    generate_contract_impl(&mut ipl, attributes.contains(&"meta"))
  } else {
    generate_compilation_error("ERROR: entrypoint_bindgen can only be applied to smart contract actions() entrypoints.".to_string())
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
/// use smart_contract::use_contract;
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
  match syn::parse::<ItemTrait>(input) {
    Ok(it) => {
      // `attr_args` is the contract address of the external contract to be called.
      match syn::parse::<NestedMeta>(attr_args) {
        Ok(a) => {
          match a {
            NestedMeta::Lit(syn::Lit::Str(s)) => {
              // transform the trait into a set of functions that calls the native cross_contract associated function
              // provided by the SDK.
              generate_external_contract_mod(it, s.value())
            },
            // if Meta is used
            NestedMeta::Meta(_) => {
              generate_compilation_error("Only &str are allowed as the sole argument to use_contract".to_string())
            },
            // multiple args
            _ =>  {
              generate_compilation_error("One argument is allowed in use_contract.".to_string())
            },
          }
        },
        // if more than one argument is used in this macro
        Err(e) => {
          generate_compilation_error(e.to_string())
        },
      }
    },
    Err(_) => {
      generate_compilation_error("use_contract can only be applied to trait definitions.".to_string())
    },
  }

}

/// The macro trait `ContractField` can be derived to generate impl so that nested struct can be supported in contract struct.
/// 
/// ### Example
/// ```no_run
/// #[derive(ContractField)]
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
#[proc_macro_derive(ContractField)]
pub fn derive_contract_field(input: TokenStream) -> TokenStream {  
  if let Ok(mut ist) = syn::parse::<ItemStruct>(input.clone()) {
    generate_storage_impl(&mut ist)
  } else {
    generate_compilation_error("derive(ContractField) can only be applied to struct definitions.".to_string())
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

