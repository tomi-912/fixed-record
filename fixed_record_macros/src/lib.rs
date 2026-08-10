#![deny(unsafe_op_in_unsafe_fn)]
use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

mod core;
mod helpers;

#[proc_macro_attribute]
pub fn fixed_record_main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let field_enum = core::gen_field_enum(&input);
    let impl_block = core::impl_fixed_record_core(&input);

    let output = quote! {
        #[repr(C)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #input

        #field_enum

        #impl_block
    };

    output.into()
}
