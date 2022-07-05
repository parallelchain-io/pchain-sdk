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

use syn::{
    Block, FnArg, ItemTrait,
    parse_quote, punctuated::Punctuated,
    TraitItem, TraitItemMethod, Signature,
    spanned::Spanned, token::Comma, ItemMod,Item, ItemFn, Visibility, ReturnType, Type, TypePath, PatType,    
};
use proc_macro::TokenStream;
use quote::{quote, format_ident};
use super::generate_compilation_error;

use snakecase::unicode::to_snakecase;

/// `general_external_contract_mod` performs the following items:
///  1. Parses the trait items of an external smart contract trait definition written by a 
///     smart contract developer.
///  2. Appends the arguments `value` and `gas` to each function signature in the trait item.
///  3. Parses the decoded contract address from the argument to `use_contract` attribute macro.
///  4. Generates a function definition from the parsed items in steps 1 - 3. The function body 
///     will contain the cross contract call provided by the SDK.
///  5. Collects the function definitions in step 4 and embed inside a `mod`. The `mod` name will
///     be the `trait` name in snake case.
/// 
/// The rationale for this is illustrated below:
///  1. Facilitates the usage of calling other contracts.
pub(crate) fn generate_external_contract_mod(trait_definition: ItemTrait, contract_address: String) -> TokenStream {
    
    /////////////////////////////////////////////////////////////////////////////////////////
    // 1. Parses the trait items of an external smart contract trait definition written by a 
    //    smart contract developer.
    /////////////////////////////////////////////////////////////////////////////////////////
    // `trait_definition` and `contract_address` are parsed at the `use_contract` attribute macro definition.
    match transform_to_external_contract_mod(trait_definition, &contract_address) {
        Ok(external_contract_mod) => quote!{ #external_contract_mod }.into(),
        Err(e) => generate_compilation_error(e.to_string()),
    }

}

// `transform_to_external_contract_mod` takes the parsed properties in the previous sentence and trasnforms
// the trait item methods into a list of function definitions embedded in a mod block.
fn transform_to_external_contract_mod(original_trait_definition: ItemTrait, contract_address: &String) -> syn::Result<ItemMod> {
        
    // stores the trait item methods that are converted into function definitions
    let mut item_functions: Vec<Item> = Vec::new();
    
    for item in original_trait_definition.items {
        match item {
            TraitItem::Method(mut method) => {
                item_functions.push(
                    // converts trait item methods into a list of function definitions
                    transform_to_function_definition(
                        &mut method,
                        &original_trait_definition.vis,
                         contract_address)?
                );
            },
            _ => {
                // if the TraitItem is not a method, throw an error and halt compilation of the smart contract
                return Err(syn::Error::new(
                    item.span(),
                    "Traits that are used to describe external contract should only include methods.",
                ))
            }
        }
    }

    ///////////////////////////////////////////////////////////////////////////////////////////////
    // 5. Collects the function definitions in step 4 and embed inside a `mod`. The `mod` name will
    //    be the `trait` name in snake case.
    ///////////////////////////////////////////////////////////////////////////////////////////////
    // Returns a new `ItemMod` syntax tree. all of the trait item methods are stored in this mod block.
    return Ok(ItemMod {
        attrs: original_trait_definition.attrs,
        vis: original_trait_definition.vis,
        mod_token: syn::token::Mod::default(),
        ident: format_ident!("{}", to_snakecase(original_trait_definition.ident.to_string())),
        content: Some((syn::token::Brace::default(), item_functions)),
        semi: None, 
    })

} 

// `transform_to_function_defintion` is where the bulk of the `use_macro` logic takes place. It does the following low level operations:
// a. Takes the trait item methods and appends the `value` and `gas` arguments to its signature.
// b. Adds a block to the function defintion in step 1. This function block contains the SDK provided cross contract associated function.
// c. Inherits the visibility properties and returns the new transformed node as a `Item`
fn transform_to_function_definition(original_trait_item_method: &mut TraitItemMethod, trait_visibility: &Visibility, contract_address: &String) -> syn::Result<Item> {
    // no default implementation of a trait is allowed. The SDK cross contract associated function will handle the default implementation.
    if original_trait_item_method.default.is_some() {
        return Err(syn::Error::new(
            original_trait_item_method.span(),
            "Traits that are used to describe external contract should not include
             default implementations because this is not a valid use case of traits
             to describe external contracts.",
        ))
    } else {
        // generate a new node that contains the arguments for `value` and `gas`
        let mut new_trait_item_method_arguments: Punctuated<FnArg, Comma> = Punctuated::new();

        // turn methods defined by trait into an associated function definition
        for function_argument in original_trait_item_method.sig.inputs.iter() {
            // parse the existing arguments of the trait item method. Remove any receivers (self, &self) in this
            // function signature as these methods will turn into associated functions.
            if let FnArg::Typed(t) = function_argument {
                let argument: FnArg = parse_quote!{#t};
                new_trait_item_method_arguments.push(argument);

                if let syn::Pat::Ident(_) = t.pat.as_ref() { } else {
                    return Err(syn::Error::new(
                        original_trait_item_method.span(),
                        "Traits that are used to describe external contract should only include function with argument name and type. For example, wildcard variable is not allowed.",
                    ));
                }
            } else {
                return Err(syn::Error::new(
                    original_trait_item_method.span(),
                    "Traits that are used to describe external contract should not include function with receiver as argument.",
                ));
            }
        }

        // generate statements to construct input arguments to call_contract
        let (let_args_builder, call_args) = 
        if original_trait_item_method.sig.inputs.len() > 0 {
            (
                quote!{ let mut args_builder = smart_contract::CallDataBuilder::new();}, 
                quote!{ args_builder.to_call_arguments() }
            )
        } else { // else the method does not take any input arguments
            (quote!{}, quote!{ vec![]})
        };
        let args_builder_add= original_trait_item_method.sig.inputs.iter().filter_map(|f|{
            match &f {
                FnArg::Typed(PatType {pat, .. }) => {
                    if let syn::Pat::Ident(e) = pat.as_ref() {
                        let argument_name = &e.ident;
                        Some(quote!{ args_builder.add(#argument_name); })
                    } else { None }
                },
                _=> None
            }
        });

        // generates nodes for each individual argument in the new function definition
        let value_arg: FnArg = parse_quote!{value: u64};
        let gas_arg: FnArg = parse_quote!{gas: u64};

        let mut call_function = quote!{ call_contract };

        // generate a node for the return type of the new associated function.
        let mut return_type: TypePath = parse_quote!{ Option<Vec<u8>> };
        if let syn::ReturnType::Type(_, box_type) = &original_trait_item_method.sig.output {
            match box_type.as_ref() {
                syn::Type::Path(e) => {
                    if let Some(ps) = e.path.segments.first() {
                        // use the call function with known return data type
                        call_function = quote!{ call };

                        // known return data type
                        let psident = &ps.ident;
                        return_type = parse_quote!{ Option<#psident> };
                    }
                }
                _=>{ return Err(syn::Error::new(
                    original_trait_item_method.span(),
                    "Traits that are used to describe external contract should include only function with Typed value or without return value",
                ));}
            }
        }

        ///////////////////////////////////////////////////////////////////////////////////////////
        // 2. Appends the arguments `value` and `gas` to each function signature in the trait item.
        ///////////////////////////////////////////////////////////////////////////////////////////
        new_trait_item_method_arguments.push(value_arg);
        new_trait_item_method_arguments.push(gas_arg);

        // gets the trait item method name to be passed as part of the SDK cross contract call
        let trait_item_method_name = format!("{}", original_trait_item_method.sig.ident.to_string());

        //////////////////////////////////////////////////////////////////////////////////////////////
        // 3. Parses the decoded contract address from the argument to `use_contract` attribute macro.
        //////////////////////////////////////////////////////////////////////////////////////////////
        //////////////////////////////////////////////////////////////////////////////////////////////
        //  4. Generates a function definition from the parsed items in steps 1 - 3. The function body 
        //     will contain the cross contract call provided by the SDK.
        //////////////////////////////////////////////////////////////////////////////////////////////
        // generate the body of the new associated function
        let callback_from_contract: Block = parse_quote! {
            {
                #let_args_builder
                #(#args_builder_add)*
                smart_contract::Transaction::#call_function(
                    smart_contract::decode_contract_address(#contract_address.to_string()),
                    #trait_item_method_name,
                    #call_args,
                    value,
                    gas
                )
            }
        };

        // returns the new associated function defintion
        return Ok(
            Item::Fn(
                ItemFn {
                    attrs: original_trait_item_method.clone().attrs,
                    vis: trait_visibility.to_owned(),
                    sig:    Signature { 
                                inputs: new_trait_item_method_arguments,
                                output: ReturnType::Type(syn::token::RArrow::default(), Box::new(Type::Path(return_type))),
                                ..original_trait_item_method.to_owned().sig
                            },
                    block: Box::new(callback_from_contract),

                }
            )
        )

    }
}
