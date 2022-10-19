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

use proc_macro2::Span;
use proc_macro::TokenStream;

/// `generate_compilation_error` is called if the macros provided 
/// by the SDK are misused on blocks of code. 
/// 
/// For example, the macro `sdk_method_bindgen` works with data structures.
/// Therefore, using this macro on a function will trigger this error.
pub(crate) fn generate_compilation_error(result_message: String) -> TokenStream {
    TokenStream::from(
        syn::Error::new(
            Span::call_site(),
            result_message,
        ).to_compile_error(),
    )
}