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

        impl #sdk_typed_trait for smart_contract::Transaction<#argument_struct_type_name> {

            fn #typed_get(&self, key: &[u8]) -> Option<#argument_struct_type_name> {

                // takes the byte string and deserializes it using borsh
                match self.get(key) {
                    Some(raw_result) => {
                        match BorshDeserialize::deserialize(&mut raw_result.as_ref()) {
                            Ok(d) => Some(d),
                            Err(_) => None,
                        }
                    },
                    None => None
                }
            }  

            // serializes the given struct into a Vec<u8>
            fn #typed_set(&self, key: &[u8], value: &#argument_struct_type_name) {

                let mut buffer: Vec<u8> = Vec::new();
                value.serialize(&mut buffer).unwrap();

                self.set(key, buffer.as_ref());
            }

        }
    
    }.into()


}