#![deny(unsafe_op_in_unsafe_fn)]
use proc_macro::TokenStream;
use syn::{
    DeriveInput, Expr, Lit, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

mod core;
mod helpers;
mod list;

struct MacroArgs {
    clear_byte: u8,
}

impl Default for MacroArgs {
    fn default() -> Self {
        Self { clear_byte: b' ' }
    }
}

impl Parse for MacroArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut args = Self::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let expr: Expr = input.parse()?;

            if ident == "clear_byte" {
                args.clear_byte = parse_clear_byte(&expr)?;
            } else {
                return Err(syn::Error::new_spanned(
                    ident,
                    "unsupported fixed_record option",
                ));
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(args)
    }
}

fn parse_clear_byte(expr: &Expr) -> syn::Result<u8> {
    if let Expr::Path(expr_path) = expr {
        if expr_path.path.is_ident("ZERO") {
            return Ok(0x00);
        }
        if expr_path.path.is_ident("SPACE") {
            return Ok(b' ');
        }
        return Err(syn::Error::new_spanned(
            expr,
            "clear_byte must be ZERO, SPACE, a byte literal, or an integer from 0 to 255",
        ));
    }

    let Expr::Lit(expr_lit) = expr else {
        return Err(syn::Error::new_spanned(
            expr,
            "clear_byte must be ZERO, SPACE, a byte literal, or an integer from 0 to 255",
        ));
    };

    match &expr_lit.lit {
        Lit::Byte(lit_byte) => Ok(lit_byte.value()),
        Lit::Int(lit_int) => lit_int.base10_parse::<u8>().map_err(|err| {
            syn::Error::new_spanned(lit_int, format!("invalid clear_byte value: {err}"))
        }),
        _ => Err(syn::Error::new_spanned(
            expr,
            "clear_byte must be ZERO, SPACE, a byte literal, or an integer from 0 to 255",
        )),
    }
}

/// Generates helper types and implementations for fixed-width records.
/// 固定長レコード用の補助型と実装を生成する attribute macro です。
#[proc_macro_attribute]
pub fn fixed_record(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as MacroArgs);
    let input = parse_macro_input!(item as DeriveInput);
    let options = core::MacroOptions {
        clear_byte: args.clear_byte,
    };

    match core::expand_fixed_record(&input, options) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
