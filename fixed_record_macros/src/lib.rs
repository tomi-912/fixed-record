#![deny(unsafe_op_in_unsafe_fn)]
use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod core;
mod helpers;

#[proc_macro_attribute]
pub fn fixed_record_main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    match core::expand_fixed_record_main(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
