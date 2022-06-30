use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{ItemStruct, ItemImpl, punctuated::Punctuated, FnArg, token::Comma, ImplItemMethod, Ident};

use super::generate_compilation_error;

/// `generate_contract_struct` performs the following items:
/// 1. imports crates from sdk
/// 2. generate implementation of Storage for contract
/// 3. generate implementation of Accesser for contract
pub(crate) fn generate_contract_struct(ist: &mut ItemStruct) -> TokenStream {
    let original_code = ist.clone();

    let code_impl_storage :proc_macro2::TokenStream = generate_storage_impl(ist).into();

    let code_impl_accesser :proc_macro2::TokenStream = generate_accesser_impl(ist).into();

    // All Code after struct
    TokenStream::from(
        quote!{
            use smart_contract::{contract_init, Transaction, Storage, StorageField, ContractCallData, Callback};

            #original_code

            #code_impl_storage

            #code_impl_accesser
        }
    )
}

/// `generate_storage_impl` generates implementation of Storage for contract (load_storage and save_storage). 
/// Example:
///```no_run
/// impl smart_contract::Storage for MyContract {
///     fn __load_storage(field: &StorageField) -> Self {
///         MyContract {
///             data: Self::__load_storage_field(&field.add(0))
///         }
///     }
///     fn __save_storage(&self, field :&StorageField) {
///         Self::__save_storage_field(&self.field, &field.add(0));
///     }
/// }
/// ```
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
            // Self is trait smart_contract::Storage
            #f_name: Self::__load_storage_field(&field.add(#i as u8))
        }
    });

    // set the values to world state
    let code_set_each_fields = fields.iter().enumerate().map(|(i, f)| {
        let f_name = f.ident.clone().unwrap();
        quote!{
            // Self is trait smart_contract::Storage
            Self::__save_storage_field(&self.#f_name, &field.add(#i as u8));
        }
    });

    TokenStream::from(
        quote!{
            impl smart_contract::Storage for #struct_name {
                fn __load_storage(field :&StorageField) -> Self {
                    #struct_name {
                        #(#code_get_each_fields,)*
                    }
                }

                fn __save_storage(&self, field :&StorageField) {
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
                Self::__load_storage_field(&StorageField::new().add(#i as u8))
            }

            fn #setter_method_name(value: #f_ty) {
                Self::__save_storage_field(&value, &StorageField::new().add(#i as u8));
            }
        }        
    });

    TokenStream::from(
        quote!{
            trait #accesser_trait {
                fn get() -> #struct_name;
                fn set(&self);
                #(#code_trait_methods_each_fields)*
            }

            impl #accesser_trait for #struct_name {
                fn get() -> #struct_name {
                    Self::__load_storage(&StorageField::new())
                }
                fn set(&self){
                    self.__save_storage(&StorageField::new())
                }
                #(#code_impl_methods_each_fields)*
            }
        }
    )
}

/// `generate_contract_impl` generate code skeleton for entrypoints
/// 
/// - Init entrypoint
/// - Actions entrypoint
/// - Views entrypoint
pub(crate) fn generate_contract_impl(ipl: &ItemImpl, is_export_metadata: bool) -> TokenStream {
    let original_code = ipl.clone();
    let impl_name = match &*ipl.self_ty {
        syn::Type::Path(tp) => tp.path.segments.first().unwrap().ident.clone(),
        _ => {
            return generate_compilation_error("Cannot find named fields in the struct".to_string())
        }
    };
    ////////////////////////////////////////////////////////
    // 1. Create Contract Entrypoint Skeleton
    ////////////////////////////////////////////////////////
    let contract_skeleton = generate_actions_entrypoint(&impl_name, ipl);
    

    ////////////////////////////////////////////////////////
    // 2. Create Init Entrypoint Skeleton
    ////////////////////////////////////////////////////////
    let constructor_skeleton = match generate_init_entrypoint(&impl_name, ipl){
        Ok(s) => s,
        Err(e) => return e
    };
    
    ////////////////////////////////////////////////////////
    // 3. Create View Entrypoint Skeleton
    ////////////////////////////////////////////////////////
    let views_skeleton = generate_views_entrypoint(&impl_name, ipl);

    ////////////////////////////////////////////////////////
    // 4. Create Meta Entrypoint Skeleton
    ////////////////////////////////////////////////////////
    let meta_skeleton = if is_export_metadata {
        let external_call_info = generate_contract_metadata(&impl_name, ipl);
        quote!{
            #[no_mangle]
            static __contract_metadata__ :&str = #external_call_info;
        }
    } else {
        quote!{}
    };

    // All Code after impl
    TokenStream::from(
        quote!{
            #original_code

            #contract_skeleton

            #constructor_skeleton

            #views_skeleton

            #meta_skeleton
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
/// let _d0: i32 = ContractCallData::parse_multiple_arguments(&multi_args, 0usize);
/// let _d1: i32 = ContractCallData::parse_multiple_arguments(&multi_args, 1usize);
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
                    let #var_name : #e_ty = ContractCallData::parse_multiple_arguments(&multi_args, #var_idx);
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

/// generate skeleton code for init entrypoint
fn generate_init_entrypoint(impl_name :&Ident, ipl: &ItemImpl) -> Result<Option<proc_macro2::TokenStream>, TokenStream> {
    // let code_contractor_call;
    let mut constructor_iter = ipl.items.iter().filter(|f| { match &f { syn::ImplItem::Method(e) => { e.is_init_entrypint() }, _=> false } });
    

    let constructor_function_name = quote!{ init };

    // Skeleton - Constructor entrypoint
    match constructor_iter.next() {
        Some(syn::ImplItem::Method(e)) => {
            if constructor_iter.count() > 0 {
                return Err(generate_compilation_error("Cannot have more than one init entrypoint method".to_string()))
            }

            if !matches!(&e.sig.output, syn::ReturnType::Default) {
                return Err(generate_compilation_error("Init entrypoint does not return value".to_string()))
            }

            let f_name = &e.sig.ident;

            // parse multiple entrypoint arguments
            let has_typed_args = e.sig.inputs.iter().any(|f| match &f {
                syn::FnArg::Typed(_) => true,
                _=> false
            });
            
            let code_init_multiple_args = if has_typed_args {
                quote!{ let multi_args = ctx.get_multiple_arguments(); }
            } else { quote!{} };

            // create method body based input arguments
            let mut pass_args :Vec<proc_macro2::TokenStream> = vec![];
            let code_parse_args = generate_let_arguments(&mut pass_args, &e.sig.inputs);

            Ok(Some(quote!{
                #[no_mangle]
                pub extern "C" fn #constructor_function_name(){
                    if let Some(mut ctx) = ContractCallData::from_raw_call_data() {
                        #code_init_multiple_args
                        #code_parse_args
                        #impl_name::#f_name(#(#pass_args,)*);
                    } else {
                        // panic if the caller invokes this contract by using input data with different calldata vesion
                        // entrypoint was not executed as expected so it should not return as success to avoid confusing caller.
                        panic!("Unable to parse input data when invoking this contract.")
                    }
                }
            }))
        }
        _=>{Ok(None)}
    }
}


/// `generate_actions_entrypoint` performs the following items:
/// 1. generate entrypoint function actions() with macro #[contract_init]
/// 2. generate skeleton of code inside actions().
/// Example:
/// ```no_run
/// #[no_mangle]
/// pub extern "C" fn actions() {
///     if let Some(mut ctx) = ContractCallData::from_raw_call_data() {
///         let callback :Callback = match ctx.method_name.as_str() {
///             "call_a_function" => {
///                 ... // execute entrypoint
///             },
///             _=>{ unimplemented!() }
///         };
///         callback.return_value();
///     }
/// }
/// ```
/// Each arm inside the entrypoint selection could be associate, mutable or immutable.
/// 
/// associate method does not interact with Contract Storage.
/// ```no_run
/// "associate_method_call" => {
///     ...
///     MyContract::associate_method_call()
///     ...
/// }
/// ```
/// 
/// immutable method can read data from Contract Storage but not modifying the data.
/// ```no_run
/// "immutable_method_call" => {
///     let mut contract = MyContract::__load_storage(&StorageField::new());
///     ...
///     MyContract::immutable_method_call()
///     ...
/// }
/// ```
/// 
/// mutable method can read/write data from/to Contract Storage.
/// ```no_run
/// "mutable_method_call" => {
///     let mut contract = MyContract::__load_storage(&StorageField::new());
///     ...
///     MyContract::mutable_method_call()
///     ...
///     contract.__save_storage(&StorageField::new());
///     ...
/// }
/// ```
fn generate_actions_entrypoint(impl_name :&Ident, ipl: &ItemImpl) -> Option<proc_macro2::TokenStream> {
    // create code segment for function selection
    let code_function_selection = ipl.items.iter().filter_map(|f| {
        match &f {
            syn::ImplItem::Method(e) => {
                let fn_name = &e.sig.ident;

                if !e.is_action_entrypoint() {
                    return None;
                }

                // define load storage
                let code_load_storage = if e.is_mutable_method() {
                    quote!{let mut contract = #impl_name::__load_storage(&StorageField::new());}
                } else if e.is_immutable_method() {
                    quote!{let contract = #impl_name::__load_storage(&StorageField::new());}
                } else {
                    quote!{}
                };

                // create method body based input arguments
                let has_typed_args = e.sig.inputs.iter().any(|f| match &f {
                    syn::FnArg::Typed(_) => true,
                    _=> false
                });
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
                if e.is_associate_method() {
                    quote!{#impl_name::#fn_name(#(#pass_args,)*);}
                } else {
                    quote!{contract.#fn_name(#(#pass_args,)*);}
                };

                // define save storage
                let code_save_storage = if e.is_mutable_method() {
                    quote!{contract.__save_storage(&StorageField::new());}
                } else {
                    quote!{}
                };

                // define return method
                let code_return_cb = 
                if has_return_value {
                    quote!{Callback::from(&ret_cb)}
                } else {
                    quote!{Callback::default()}
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

    // Skeleton - actions entrypoint
    Some(quote!{
        #[no_mangle]
        pub extern "C" fn actions() {
            // Parse contract input
            if let Some(mut ctx) = ContractCallData::from_raw_call_data() {
                // Enter function selector
                let callback :Callback = match ctx.get_method_name() {
                    #(#code_function_selection)*
                    _=>{ unimplemented!() }
                };
                // Return
                callback.return_value();
            } else {
                // panic if the caller invokes this contract by using input data with different calldata vesion
                // entrypoint was not executed as expected so it should not return as success to avoid confusing caller.
                panic!("Unable to parse input data when invoking this contract.")
            }
        }
    })
}

fn generate_views_entrypoint(impl_name :&Ident, ipl: &ItemImpl) -> Option<proc_macro2::TokenStream> {
    // check whether it consists of view methods. Otherwise return None so that there is no warnings for unreachable code
    if !ipl.items.iter().any(|f|{
        match &f {
            syn::ImplItem::Method(e) => e.is_view_entrypoint(),
            _ => false
        }
    }) { return None }
    
    // create code segment for function selection
    let code_views_selection = ipl.items.iter().filter_map(|f| {
        match &f {
            syn::ImplItem::Method(e) => {
                let fn_name = &e.sig.ident;
                if !e.is_view_entrypoint() {
                    return None;
                }

                // create method body based input arguments
                let has_typed_args = e.sig.inputs.iter().any(|f| match &f {
                    syn::FnArg::Typed(_) => true,
                    _=> false
                });
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
                let code_call_function = quote!{#impl_name::#fn_name(#(#pass_args,)*);};
                
                // define return method
                let code_return_cb = 
                if has_return_value {
                    quote!{Callback::from(&ret_cb)}
                } else {
                    quote!{Callback::default()}
                };

                Some(quote!{
                    stringify!(#fn_name) => {
                        #code_init_multiple_args
                        #code_parse_args
                        #code_return_handle
                        #code_call_function
                        #code_return_cb
                    }
                })
            }
            _=>{None}
        }
    });

    // Skeleton - views entrypoint
    Some(
    quote!{
        #[no_mangle]
        pub extern "C" fn views() {
            if let Some(mut ctx) = ContractCallData::from_raw_call_data() {
                // Enter function selector
                let callback :Callback = match ctx.get_method_name() {
                    #(#code_views_selection)*
                    _=>{ unimplemented!() }
                };
                // Return
                callback.return_value();
            } else {
                // panic if the caller invokes this contract by using input data with different calldata vesion
                // entrypoint was not executed as expected so it should not return as success to avoid confusing caller.
                panic!("Unable to parse input data when invoking this contract.")
            }
        }
    })
}

/// generate the string representation of contract meta
fn generate_contract_metadata(impl_name :&Ident, ipl: &ItemImpl) -> String {
    let fns : String = ipl.items.iter().flat_map(|f| {
        match &f {
            syn::ImplItem::Method(e) => {
                if !e.is_action_entrypoint() { return None }
                let args : Vec<String>= e.sig.inputs.iter().flat_map(|arg| {
                    match &arg {
                        syn::FnArg::Typed(pty) => {
                            let pat = &pty.pat;
                            let ty = &pty.ty;
                            let pat_string = quote!(#pat);
                            let ty_string = quote!(#ty).to_string().chars().filter(|c| !c.is_whitespace()).collect::<String>();
                            Some(format!("{} :{}", pat_string, ty_string))
                        },
                        _=>{ None }
                    }
                }).collect();
                
                let args = args.join(", ");
                let rets = match &e.sig.output{
                    syn::ReturnType::Type(_, output_type) => {
                        let ret_tokenstream = quote!(#output_type);
                        format!(" -> {}", ret_tokenstream.to_string().chars().filter(|c| !c.is_whitespace()).collect::<String>())
                    }
                    _=> { "".to_string()}
                };
                Some(format!("fn {}({}){};", &e.sig.ident ,args, rets))
            }
            _=>{None}
        }
    }).collect();
    format!("pub trait {} {{{}}}\0", impl_name, fns)
}

/// Trait for adding helper functions to method for checking information of a contract
trait EntrypointAnalysis {
    fn is_mutable_method(&self) -> bool;
    fn is_immutable_method(&self) -> bool;
    fn is_associate_method(&self) -> bool;
    fn is_init_entrypint(&self) -> bool;
    fn is_view_entrypoint(&self) -> bool;
    fn is_action_entrypoint(&self) -> bool;
}

/// Impl for EntrypointAnalysis explicitly to see if the methods match with design of a contract 
impl EntrypointAnalysis for ImplItemMethod {

    fn is_mutable_method(&self) -> bool {
        // mutable method with &mut self as receiver
        let fn_args = &self.sig.inputs;
        if fn_args.len() < 1 { return false; }
        match &fn_args[0] {
            syn::FnArg::Receiver(e) =>{
                e.mutability.is_some()
            }
            _=>{ false }
        }
    }
    fn is_immutable_method(&self) -> bool {
        // immutable method with &self as receiver
        let fn_args = &self.sig.inputs;
        if fn_args.len() < 1 { return false; }
        match &fn_args[0] {
            syn::FnArg::Receiver(e) =>{
                e.mutability.is_none()
            }
            _=>{ false }
        }
    }
    fn is_associate_method(&self) -> bool {
        // method without receiver
        let fn_args = &self.sig.inputs;
        !fn_args.iter().any(|fa| {
            matches!(&fa, syn::FnArg::Receiver(_)) 
        })
    }

    fn is_init_entrypint(&self) -> bool {
        let fn_args = &self.sig.inputs;
        // init does not take receiver arguments
        if fn_args.iter().any(|fa| { matches!(&fa, syn::FnArg::Receiver(_))}) {
            return false;
        }

        self.attrs.iter().any(|attr|{
            match attr.parse_meta() {
                Ok(meta)=>{
                    match meta.path().get_ident() {
                        Some(ident)=> ident.to_string() == "init".to_string(),
                        _=>{false}
                    }}
                _=>{false}
            }
        })
    }
    
    fn is_view_entrypoint(&self) -> bool {
        // view entrypoint does not count as normal associate method
        self.attrs.iter().any(|attr|{
            match attr.parse_meta() {
                Ok(meta)=>{
                    match meta.path().get_ident() {
                        Some(ident)=> ident.to_string() == "view".to_string(),
                        _=>{false}
                    }}
                _=>{false}
            }
        })
    }

    fn is_action_entrypoint(&self) -> bool {
        self.attrs.iter().any(|attr|{
            match attr.parse_meta() {
                Ok(meta)=>{
                    match meta.path().get_ident() {
                        Some(ident)=> ident.to_string() == "action".to_string(),
                        _=>{false}
                    }}
                _=>{false}
            }
        })
    }

}