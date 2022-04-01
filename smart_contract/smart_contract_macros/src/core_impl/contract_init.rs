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
    Abi, Attribute,
    Block, Expr, ExprBlock,
    FnArg, Ident, 
    ItemFn,Local, Pat,
    ReturnType, Stmt, Type,
    token, parse_quote, PathArguments,  
};
use syn::fold::{self, Fold};
use proc_macro::TokenStream;
use quote::quote;
use super::generate_compilation_error;

//////////////////////////////////////////////////////////////////////////////
// TransformationRetrurnExpr traverses every node to identify the return value 
// of a node through recursion.
//////////////////////////////////////////////////////////////////////////////
struct TransformReturnExpr {
    // 1. The fold recursion trait in syn package only supports the transformation 
    //    of a node of one form to another. 
    //
    // 2. Therefore the method signature of this trait takes in the
    //    node as its only argument.
    //
    // 3. `TransformReturnExpr` contains a field called `sdk_variable_name` which 
    //    allows a return expression to call the SDK's return_value() method.
    sdk_variable_name: Ident,
}

impl Fold for TransformReturnExpr {
    // `fold_expr` is a convenience function provided by the syn crate
    // that traverses through every node of a specified type (e.g: Expr)
    // in the syntax tree.
    fn fold_expr(&mut self, node: Expr) -> Expr {
        // Traverses the syntax tree to see if a node is of
        // a `ExprReturn` type. Any node that contains the explicit
        // keyword `return` will be transformed into the SDK's 
        // `return_value()` method
        if let Expr::Return(e) = node {
            // Assigns the value inside of the "returned" call to a temporary variable
            // name called `ret_val` for serialization.
            let ret_val = *e.expr.unwrap();
            
            // the SDK variable name to call the `return_value()` method.
            let sdk_variable_name = &self.sdk_variable_name;
            
            // Transforms the parsed nodes above into another node that:
            // 1. Serializes ret_val and writes to a buffer as a Vec<u8>.
            // 2. Calls the SDK's `return_value()` method with the buffer as the argument.
            let transformed_return_expr: ExprBlock = parse_quote!(
                {
                    let ret_variable = #ret_val;
                    let mut return_value_buffer: Vec<u8> = Vec::new();
                    ret_variable.serialize(&mut return_value_buffer).unwrap();
                    #sdk_variable_name.return_value(return_value_buffer);
                }
            );
    
            Expr::Block(transformed_return_expr)
                  
        } else {
            // Delegate to the default impl to traverse any nested functions.
            fold::fold_expr(self, node)
        }
    
     
        
    }
}


/// `transform_contract_entrypoint` performs the following items:
///  1. Gets all the nodes required to transform the `contract()` entrypoint.
///. 2. Moves the SDK definition from an argument in `contract()` entrypoint
///     to a variable assignment within the entrypoint.
///  3. Traverses through the syntax tree of the `contract()` entrypoint's body of 
///.    statements to get explicit calls to `return` and the last return statement 
///     if any. Transforms the mentioned "returns" into the SDK's `return_value()` method.
///  4. Generates the new `contract()` entrypoint in the smart contract source code.
/// 
/// The rationale for this is illustrated below:
///  1. Encourage smart contract developers to write smart contracts in idiomatic rust.
///  2. The smart contracts are built in WebAssembly and compiled in wasmer. Wasmer is an
///     isolated and deterministic containerized environment. The values are returned as 
///     `protocol_types::Transaction::Receipts` that are callable by the SDK's `return_value()` 
///     method.
pub(crate) fn transform_contract_entrypoint(entrypoint_node: &mut ItemFn) -> TokenStream {

    ///////////////////////////////////////////////////////////////////////////
    // 1. Gets all the nodes required to transform the `contract()` entrypoint.
    ///////////////////////////////////////////////////////////////////////////
    // check if there is more than one argument to the entrypoint function
    if *&entrypoint_node.sig.inputs.len() as i32 != 1 {
        return generate_compilation_error(
            "ERROR: when using entrypoint_bindgen macro,
            please only set the smart contract sdk 
            (smart_contract::Transaction<A> as the only argument
            to the contract() entrypoint function.)".to_string());
    }
    
    // `entrypoint_argument` is the SDK itself.
    let entrypoint_argument = 
        // SAFETY: The `contract()` entrypoint only takes in the
        // SDK (smart_contract::Transaction<A>) as its argument.
        match &entrypoint_node.sig.inputs.first().unwrap() {
            FnArg::Typed(pt) => {
                pt
            }
            _ => unreachable!(),
    };

    // Parses the variable name of the SDK
    let sdk_variable_name = 
        if let Pat::Ident(pi)= &*entrypoint_argument.pat {
            &pi.ident
        } else {
            unreachable!()
    };

    // generic_parameter (A) is any type supplied to  the SDK (smart_contract::Transaction).
    // It accepts any primitive or non-primitive types. 
    let generic_parameter = 
        if let Type::Path(tp) = &*entrypoint_argument.ty {
                tp.path.segments.first().unwrap().arguments.to_owned()
            } else {
                unreachable!()
    };
    
    /////////////////////////////////////////////////////////////////////////////
    //  2. Moves the SDK definition from an argument in `contract()` entrypoint
    //     to a variable assignment within the entrypoint.
    ////////////////////////////////////////////////////////////////////////////
    let sdk_contract_statement: Stmt = parse_quote! {
        let #entrypoint_argument = smart_contract::Transaction::#generic_parameter::new();
    };

    // vector to contain all of the transformed statements in the `contract()` entrypoint
    let mut transformed_contract_statements: Vec<Stmt> = Vec::new();
    
    // appends the SDK as the first statement in the vector.
    transformed_contract_statements.push(sdk_contract_statement);
    
    /////////////////////////////////////////////////////////////////////////////////////////
    // 3. Traverses through the syntax tree of the `contract()` entrypoint's body of 
    //    statements to get explicit calls to `return` and the last return statement 
    //    if any. Transforms the mentioned "returns" into the SDK's `return_value()` method.
    /////////////////////////////////////////////////////////////////////////////////////////
    // Append the rest of the existing contract_statements
    if entrypoint_node.block.stmts.len() != 0 {

        // Gets the return type from `contract()` entrypoint. Any child of a `ReturnType` node
        // will be assigned to a variable and have its typed inferred to said return type.
        let return_type: Option<Type> = if let syn::ReturnType::Type(_, t) = &entrypoint_node.sig.output {
            Some(*t.to_owned())
        } else {
            None
        };


        // Transforms the last_statement of `contract()` into a call to SDK's `return_value()` 
        // if there are any explicit calls to return. 
        let transformed_last_statement = 
            transform_last_expression_to_sdk_return(
                sdk_variable_name,
                &entrypoint_node.block.stmts.pop().unwrap(),
                return_type.as_ref()
        );
  
        // checks for the number of statements in the `contract()` entrypoint.
        // If this is zero, the only statements available in the entrypoint are
        // the SDK initialization and the last statement.
        if entrypoint_node.block.stmts.len() > 0 {
            // Transforms any statements in `contract()` that explicitly calls
            // a return keyword.
            for statement in &entrypoint_node.block.stmts {
                let transformed_contract_statement = 
                    transform_qualified_contract_statement_to_sdk_return(
                        &sdk_variable_name,
                        statement
                    );
                transformed_contract_statements.push(
                    transformed_contract_statement.to_owned());
            }
        } 

        // append the transformed last statement to the block of statements
        // in the `contract()` entrypoint.
        for return_statement in transformed_last_statement {
            transformed_contract_statements.push(return_statement);
        }
        
    }

    ///////////////////////////////////////////////////////////////////////////////////
    //  4. Generates the new `contract()` entrypoint in the smart contract source code.
    ///////////////////////////////////////////////////////////////////////////////////
    let transformed_item_fn = transform_item_fn_node(
        entrypoint_node.to_owned(),
        transformed_contract_statements,
    );

    quote!{
        #transformed_item_fn
    }.into()

}


// `transform_last_expression_to_sdk_return` function does the following:
// a. Extracts the `last_statement` from the block of statements inside `contract()` entrypoint.
// b. Prepares a return variable for serialization. 
// c. Serializes the return variable in "b" and feeds it as an argument to the SDK's `return_value()` method.
fn transform_last_expression_to_sdk_return(
    sdk_variable_name: &Ident,
    last_statement: &Stmt,
    return_type: Option<&Type>,
) -> Vec<Stmt> {
    ////////////////////////////////////////////////////////////////////////////////////////////////
    // a. Extracts the `last_statement` from the block of statements inside `contract()` entrypoint.
    ////////////////////////////////////////////////////////////////////////////////////////////////
    let last_expression =
    extract_expression_from_last_statement(last_statement);

    match return_type {
        Some(rt) => {
            ///////////////////////////////////////////////////
            // b. prepares a return variable for serialization.
            ///////////////////////////////////////////////////
            let sdk_variable_return_name: Ident = parse_quote!(
                ret_val
            );


            if let Type::Path(tp) = 
                rt {
                    // check if return type is a Result
                    if &tp.path.segments.first().unwrap().ident.to_string() == "Result" {
                        
                        // get the return type inside Result
                        let parsed_return_type = if let PathArguments::AngleBracketed(ab) = 
                            &tp.path.segments.first().unwrap().arguments {
                                ab.args.first().unwrap()
                            } else {
                                unreachable!()
                        };
                        
                        // If the last statement is inside a `Result` enum, the value inside `Result` will be unwrapped.
                        // This value will be assigned to `sdk_variable_return_name`. `sdk_variable_return_name` will be
                        // inferred with the return type of `contract()` entrypoint. Otherwise, ignore the unwrap process
                        // and use the return type directly. 
                        let transformed_last_statement: Vec<Stmt> = parse_quote!(
                            let #sdk_variable_return_name: #return_type = #last_expression;
                            let unwrapped_sdk_variable_return_name: #parsed_return_type = #sdk_variable_return_name.unwrap();
                            
                            let mut return_value_buffer : Vec<u8> = Vec::new();
                            /////////////////////////////////////////////////////////////////////////////////////////////////////////////
                            // c. Serializes the return variable in "b" and feeds it as an argument to the SDK's `return_value()` method.
                            /////////////////////////////////////////////////////////////////////////////////////////////////////////////
                            unwrapped_sdk_variable_return_name.serialize(&mut return_value_buffer)
                                                              .unwrap();
                            #sdk_variable_name.return_value(return_value_buffer);
                        );

                        transformed_last_statement

                    } else {
                        // If the last statement is inside a `Result` enum, the value inside `Result` will be unwrapped.
                        // This value will be assigned to `sdk_variable_return_name`. `sdk_variable_return_name` will be
                        // inferred with the return type of `contract()` entrypoint. Otherwise, ignore the unwrap process
                        // and use the return type directly. 
                        let transformed_last_statement: Vec<Stmt> = parse_quote!(
                            let #sdk_variable_return_name: #return_type = #last_expression;
                
                            let mut return_value_buffer : Vec<u8> = Vec::new();
                            /////////////////////////////////////////////////////////////////////////////////////////////////////////////
                            // c. Serializes the return variable in "b" and feeds it as an argument to the SDK's `return_value()` method.
                            /////////////////////////////////////////////////////////////////////////////////////////////////////////////
                            #sdk_variable_return_name.serialize(&mut return_value_buffer)
                                                     .unwrap();
                            #sdk_variable_name.return_value(return_value_buffer.as_ref());
                        );
                        
                        transformed_last_statement

                    }
                } else {
                    unreachable!()
            }

        },
        // If there is no return value in the `contract()` function, add the last statement 
        // to the block of statements in `contract()` entrypoint.
        None => {
            let untransformed_last_statement = last_statement.to_owned();
            let untransformed_last_statement: Vec<Stmt> = parse_quote!(
                    #untransformed_last_statement
            );
            untransformed_last_statement    
        }
    }
    
}

// `extract_expression_from_last_statement` parses an expression with a
// return value from last_statement.
fn extract_expression_from_last_statement(last_statement: &Stmt) -> &Expr {
    match last_statement {
        // If the statement contains an expression.
        Stmt::Expr(e) => {
            // If there is an explicit return keyword in the last statement
            if let Expr::Return(re) = e {
                let unwrapped_re = re.expr.as_ref().unwrap();
                &*unwrapped_re
            } else {
                e
            }
        },
        // If the statement contains an expression and a semicolon
        Stmt::Semi(e, _) => {
            // If there is an explicit return keyword in the last statement
            if let Expr::Return(re) = e {
                let unwrapped_re = re.expr.as_ref().unwrap();
                &*unwrapped_re
            } else {
                e
            }
        },
        // traverses through the child nodes for any expression within the 
        // last statement
        _ => extract_expression_from_last_statement(last_statement)
    }
}

// `transform_qualified_contract_statement_to_sdk_return` is a wrapper function for
// TransformReturnExpr.fold_expr that transforms any explicit calls to return 
// in `contract()` to the SDK's `return_value()` method. See `TransformReturnExpr`
// for a more detailed explanation of what it does.
fn transform_qualified_contract_statement_to_sdk_return(
    sdk_variable_name: &Ident,
    statement: &Stmt
) -> Stmt {

    // initializes the Transformation struct to do the tree traversal.
    let mut transformation_struct = TransformReturnExpr {
        sdk_variable_name: sdk_variable_name.to_owned(),
    };

    // If the node contains an explicit return statement.
    match statement {
        // if the node is an expression without a semicolon
        Stmt::Expr(e) => {
            Stmt::Expr(transformation_struct.fold_expr(e.to_owned()))   
        },
        // if the node is an expression with a semicolon
        Stmt::Semi(e, semi) => {    
            Stmt::Semi(
                transformation_struct.fold_expr(e.to_owned()), 
                *semi
            )
        },
        // if the node is a local assignment (for eg: let a = 10;)
        Stmt::Local(l) => {
            match &l.init {
                Some(li) => {
                    let expression = *li.1.to_owned();
                    Stmt::Local(Local {
                        attrs: l.clone().attrs,
                        let_token: l.clone().let_token,
                        pat: l.clone().pat,
                        init: Some(
                            // the eq token from the statement
                            (l.clone().init.unwrap().0,
                             Box::new(transformation_struct.fold_expr(expression)))
                        ),
                        semi_token: l.semi_token
                    })
                },
                None => statement.to_owned(),
            }
        },
        _ => statement.to_owned()
    }

}


fn transform_item_fn_node(
    parsed_entrypoint_node: ItemFn, 
    transformed_contract_statements: Vec<Stmt>,
) -> ItemFn {

    // get the transformed statements in `contract()` entrypoint
    let transformed_block: Box<Block> = Box::new(
        Block{
            brace_token: token::Brace::default(),
            stmts: transformed_contract_statements,
        }
    );

    // clear the arguments to the entrypoint function
    let mut transformed_sig = parsed_entrypoint_node.sig;
    transformed_sig.inputs.clear();

    // add the #[no_mangle] attribute for the transformed smart contract entrypoint
    let no_mangle_attr: Attribute = parse_quote!(
        #[no_mangle]
    );

    // add the extern "C" abi for executor to communicate with wasmer
    let abi_extern: Abi = parse_quote!(
        extern "C"
    );

    // remove the return type in the node by changing the output field to default.
    ItemFn {
        attrs: vec![no_mangle_attr],
        vis: parsed_entrypoint_node.vis,
        sig: syn::Signature {
            constness: transformed_sig.constness,
            asyncness: transformed_sig.asyncness,
            unsafety: transformed_sig.unsafety,
            abi: Some(abi_extern),
            fn_token: transformed_sig.fn_token,
            ident: transformed_sig.ident,
            generics: transformed_sig.generics,
            paren_token: transformed_sig.paren_token,
            inputs: transformed_sig.inputs,
            variadic: transformed_sig.variadic,
            output: ReturnType::Default,
        },
        block: transformed_block,
    }
}
