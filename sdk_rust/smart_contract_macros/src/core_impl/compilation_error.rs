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