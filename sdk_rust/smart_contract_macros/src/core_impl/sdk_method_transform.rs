use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::ItemStruct;
use snakecase::unicode::to_snakecase;


/// `generate_sdk_typed_methods` do the following items:
///  1. Parses the `argument_struct_type_name`.
///  2. Creates an SDK typed trait that allows direct interaction 
///     with the world state using custom data types such as structs.
/// 
/// The rationale for this is the SDK `get` and `set` methods only support 
/// byte strings. `generate_sdk_typed_methods` helps serialize custom data 
/// types into byte strings. 
pub(crate) fn generate_sdk_typed_methods(argument_struct_definition_node: &ItemStruct) -> TokenStream {
    
    /////////////////////////////////////////////
    // 1. Parses the `argument_struct_type_name`.
    /////////////////////////////////////////////
    let argument_struct_type_name = &argument_struct_definition_node.ident;

    ///////////////////////////////////////////////////////////////////////////////
    //  2. Creates an SDK typed trait that allows direct interaction with the world
    //.    state using custom data types such as structs.
    ///////////////////////////////////////////////////////////////////////////////
    // for example : MyStruct becomes `get_my_struct` and `set_my_struct`
    let typed_get = format_ident!("get_{}", to_snakecase(argument_struct_type_name.to_string()));
    let typed_set = format_ident!("set_{}", to_snakecase(argument_struct_type_name.to_string()));

    let sdk_typed_trait = format_ident!("sdk_typed_{}", argument_struct_type_name.to_string());

    quote!{

        pub trait #sdk_typed_trait {
            fn #typed_get(&self, key: &[u8]) -> Option<#argument_struct_type_name>;
            fn #typed_set(&self, key: &[u8], value: &#argument_struct_type_name); 
        }

        #argument_struct_definition_node

        impl #sdk_typed_trait for smart_contract::Transaction {

            fn #typed_get(&self, key: &[u8]) -> Option<#argument_struct_type_name> {

                // takes the byte string and deserializes it using borsh
                match Transaction::get(key) {
                    Some(raw_result) => {
                        let p :Option<#argument_struct_type_name> = match BorshDeserialize::deserialize(&mut raw_result.as_ref()) {
                            Ok(d) => Some(d),
                            Err(_) => None,
                        };
                        p
                    },
                    None => None
                }
            }  

            // serializes the given struct into a Vec<u8>
            fn #typed_set(&self, key: &[u8], value: &#argument_struct_type_name) {

                let mut buffer: Vec<u8> = Vec::new();
                value.serialize(&mut buffer).unwrap();

                Transaction::set(key, buffer.as_ref());
            }

        }
    
    }.into()
}
