/*
    Copyright © 2023, ParallelChain Lab 
    Licensed under the Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
*/

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{ItemStruct, ItemImpl, punctuated::Punctuated, FnArg, token::Comma, ImplItemMethod, Ident};

use super::generate_compilation_error;

/// `generate_contract_struct` performs the following items:
/// 1. imports crates from sdk
/// 2. generate implementation of Storage for contract
/// 3. generate implementation of Accesser for contract
pub(crate) fn generate_contract_struct(ist: &mut ItemStruct) -> TokenStream {
    let contract_struct = ist.clone();

    let code_impl_storage :proc_macro2::TokenStream = generate_storage_impl(ist).into();

    let code_impl_accesser :proc_macro2::TokenStream = generate_accesser_impl(ist).into();

    // All Code after struct
    TokenStream::from(
        quote!{
            use pchain_sdk::Storable;
            
            #contract_struct

            #code_impl_storage

            #code_impl_accesser
        }
    )
}

/// `generate_storage_impl` generates implementation of Storable for contract (load_storage and save_storage). 
pub(crate) fn generate_storage_impl(ist: &mut ItemStruct) -> TokenStream {
    let struct_name = &ist.ident;
    let fields = if let syn::Fields::Named(syn::FieldsNamed {ref named, ..})
    = &ist.fields {
        named
    } else {
        return generate_compilation_error("Cannot find named fields in the struct".to_string())
    };

    // get the values from world state
    let code_get_each_fields = fields.iter().enumerate().map(|(i, f)| {
        let f_name = f.ident.clone().unwrap();
        quote!{
            // Self is trait pchain_sdk::Storage
            #f_name: pchain_sdk::Storable::__load_storage(&field.add(#i as u8))
        }
    });

    // set the values to world state
    let code_set_each_fields = fields.iter().enumerate().map(|(i, f)| {
        let f_name = f.ident.clone().unwrap();
        quote!{
            // Self is trait Storage
            self.#f_name.__save_storage(&field.add(#i as u8));
        }
    });

    TokenStream::from(
        quote!{
            impl pchain_sdk::Storable for #struct_name {
                fn __load_storage(field :&pchain_sdk::StoragePath) -> Self {
                    #struct_name {
                        #(#code_get_each_fields,)*
                    }
                }

                fn __save_storage(&mut self, field :&pchain_sdk::StoragePath) {
                    #(#code_set_each_fields)*
                }
            }
        }
    )
}

/// `generate_accesser_impl` creates trait Accesser and generates implementation of getters and setters for contract.
/// 
/// Example:
///```no_run
/// 
/// trait MyContractAccesser {
///     fn get_data()->i32;
///     fn set_data(_:i32);
/// }
/// impl MyContractAccesser for MyContract {
///     fn get_data()->i32 {
///         ...
///     }
///     fn set_data(value: i32) {
///         ...
///     }
/// }
/// ```
pub(crate) fn generate_accesser_impl(ist: &mut ItemStruct) -> TokenStream {
    let struct_name = &ist.ident;
    let fields = if let syn::Fields::Named(syn::FieldsNamed {ref named, ..})
    = &ist.fields {
        named
    } else {
        return generate_compilation_error("Cannot find named fields in the struct".to_string())
    };

    // trait name for getter and setting
    let accesser_trait = format_ident!("{}Accesser", struct_name.to_string());

    let code_trait_methods_each_fields = fields.iter().map(|f| {
        let f_name = f.ident.clone().unwrap();
        let f_ty = f.ty.clone();
        let getter_method_name = format_ident!("get_{}", f_name.to_string());
        let setter_method_name = format_ident!("set_{}", f_name.to_string());
        quote!{
            fn #getter_method_name() -> #f_ty;
            fn #setter_method_name(_: #f_ty);
        }
    });


    let code_impl_methods_each_fields = fields.iter().enumerate().map(|(i, f)| {
        let f_name = f.ident.clone().unwrap();
        let f_ty = f.ty.clone();
        let getter_method_name = format_ident!("get_{}", f_name.to_string());
        let setter_method_name = format_ident!("set_{}", f_name.to_string());

        quote!{
            fn #getter_method_name() -> #f_ty {
                pchain_sdk::Storable::__load_storage(&pchain_sdk::StoragePath::new().add(#i as u8))
            }

            fn #setter_method_name(mut value: #f_ty) {
                value.__save_storage(&pchain_sdk::StoragePath::new().add(#i as u8));
            }
        }        
    });

    TokenStream::from(
        quote!{
            trait #accesser_trait {
                fn get() -> #struct_name;
                fn set(&mut self);
                #(#code_trait_methods_each_fields)*
            }

            impl #accesser_trait for #struct_name {
                fn get() -> #struct_name {
                    Self::__load_storage(&pchain_sdk::StoragePath::new())
                }
                fn set(&mut self){
                    self.__save_storage(&pchain_sdk::StoragePath::new())
                }
                #(#code_impl_methods_each_fields)*
            }
        }
    )
}

/// `generate_contract_impl` generate code skeleton for Contract Methods
pub(crate) fn generate_contract_impl(ipl: &ItemImpl) -> TokenStream {
    let original_code = ipl.clone();
    let impl_name = match &*ipl.self_ty {
        syn::Type::Path(tp) => tp.path.segments.first().unwrap().ident.clone(),
        _ => {
            return generate_compilation_error("Cannot find named fields in the struct".to_string())
        }
    };

    // Create Contract Method Skeleton
    let contract_skeleton = generate_contract_methods(&impl_name, ipl);

    // All Code after impl
    TokenStream::from(
        quote!{
            #original_code

            #contract_skeleton
        }
    )
}

/// generate code segmenet from function arguments. e.g.
/// 
/// ===> transform from fn func (a: i32, b: String)
/// 
/// pass_args:
/// \[_d0, _d1\]
/// 
/// return:
/// ```no_run
/// let _d0: i32 = ContractMethodInput::parse_multiple_arguments(&multi_args, 0usize);
/// let _d1: i32 = ContractMethodInput::parse_multiple_arguments(&multi_args, 1usize);
/// ```
/// 
fn generate_let_arguments(pass_args :&mut Vec<proc_macro2::TokenStream>, fn_args :&Punctuated<FnArg, Comma>) -> proc_macro2::TokenStream {
    let mut var_idx :usize= 0;
    let code_parse_args = fn_args.iter().filter_map(|fa| {
        match &fa {
            syn::FnArg::Typed(e) => {
                let var_name = format_ident!("_d{}", format!("{}",var_idx));
                let e_ty = &e.ty;
                let q = quote!{
                    let #var_name : #e_ty = pchain_sdk::ContractMethodInput::parse_multiple_arguments(&multi_args, #var_idx);
                };
                var_idx+=1;
                pass_args.push(quote!{
                    #var_name
                });
                Some(q)
            }
            _=>{None}
        }
    });
    
    quote!{
        #(#code_parse_args)*
    }
}

/// `generate_contract_methods` performs the following items:
/// 1. generate contract method function entrypoint() with macro #[contract_init]
/// 2. generate skeleton of code inside entrypoint().
fn generate_contract_methods(impl_name :&Ident, ipl: &ItemImpl) -> Option<proc_macro2::TokenStream> {
    // create code segment for function selection
    let code_function_selection = ipl.items.iter().filter_map(|f| {
        match &f {
            syn::ImplItem::Method(e) => {
                let fn_name = &e.sig.ident;

                if !e.is_contract_method() {
                    return None;
                }

                // define load storage
                let code_load_storage = if e.is_mutable() {
                    quote!{let mut contract = #impl_name::__load_storage(&pchain_sdk::StoragePath::new());}
                } else if e.is_immutable() {
                    quote!{let contract = #impl_name::__load_storage(&pchain_sdk::StoragePath::new());}
                } else {
                    quote!{}
                };

                // create method body based input arguments
                let has_typed_args = e.sig.inputs.iter().any(|f| matches!(f, syn::FnArg::Typed(_)));
                let code_init_multiple_args = if has_typed_args {
                    quote!{ let multi_args = ctx.get_multiple_arguments(); }
                } else { quote!{} };
                let mut pass_args :Vec<proc_macro2::TokenStream> = vec![];
                let code_parse_args = generate_let_arguments(&mut pass_args, &e.sig.inputs);

                // define calling body
                let has_return_value = !matches!(&e.sig.output, syn::ReturnType::Default);
                let code_return_handle = if has_return_value {
                    quote!{let ret_cb = }
                } else {
                    quote!{}
                };
                let code_call_function = 
                if e.is_associate() {
                    quote!{#impl_name::#fn_name(#(#pass_args,)*);}
                } else {
                    quote!{contract.#fn_name(#(#pass_args,)*);}
                };

                // define save storage
                let code_save_storage = if e.is_mutable() {
                    quote!{contract.__save_storage(&pchain_sdk::StoragePath::new());}
                } else {
                    quote!{}
                };

                // define return method
                let code_return_cb = 
                if has_return_value {
                    quote!{pchain_sdk::ContractMethodOutput::set(&ret_cb)}
                } else {
                    quote!{pchain_sdk::ContractMethodOutput::default()}
                };

                Some(quote!{
                    stringify!(#fn_name) => {
                        #code_load_storage
                        #code_init_multiple_args
                        #code_parse_args
                        #code_return_handle
                        #code_call_function
                        #code_save_storage
                        #code_return_cb
                    }
                })               
            }
            _=> {None}
        }
    });

    // Skeleton - contract entrypoint
    Some(quote!{
        #[no_mangle]
        pub extern "C" fn entrypoint() {
            // Parse contract input
            let mut ctx = pchain_sdk::ContractMethodInput::from_transaction();
            // Enter function selector
            let callresult: pchain_sdk::ContractMethodOutput = match ctx.method_name.as_str() {
                #(#code_function_selection)*
                _=>{ unimplemented!() }
            };
            // Return
            if let Some(return_value) = callresult.get() {
                pchain_sdk::return_value(return_value);
            } 
        }
    })
}

/// Trait for adding helper functions to method for checking information of a contract
trait ContractMethodAnalysis {
    fn is_mutable(&self) -> bool;
    fn is_immutable(&self) -> bool;
    fn is_associate(&self) -> bool;
    fn is_contract_method(&self) -> bool;
}

/// Impl for EntrypointAnalysis explicitly to see if the methods match with design of a contract 
impl ContractMethodAnalysis for ImplItemMethod {

    fn is_mutable(&self) -> bool {
        // mutable method with &mut self as receiver
        let fn_args = &self.sig.inputs;
        if fn_args.is_empty() { return false; }
        match &fn_args[0] {
            syn::FnArg::Receiver(e) =>{
                e.mutability.is_some()
            }
            _=>{ false }
        }
    }
    fn is_immutable(&self) -> bool {
        // immutable method with &self as receiver
        let fn_args = &self.sig.inputs;
        if fn_args.is_empty() { return false; }
        match &fn_args[0] {
            syn::FnArg::Receiver(e) =>{
                e.mutability.is_none()
            }
            _=>{ false }
        }
    }
    fn is_associate(&self) -> bool {
        // method without receiver
        let fn_args = &self.sig.inputs;
        !fn_args.iter().any(|fa| {
            matches!(&fa, syn::FnArg::Receiver(_)) 
        })
    }

    fn is_contract_method(&self) -> bool {
        self.attrs.iter().any(|attr|{
            attr.parse_meta().map_or(false, |meta| {
                meta.path().get_ident().map_or(false, |ident| {
                    *ident == *"call"
                })
            })
        })
    }

}