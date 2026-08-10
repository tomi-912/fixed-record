use syn::{Expr, GenericArgument, Lit, PathArguments, Type};

/// 型が Fixed<N> であることを確認し、N の数値を取得する
pub fn extract_fixed_len(ty: &Type) -> syn::Result<usize> {
    let Type::Path(tp) = ty else {
        return Err(syn::Error::new_spanned(
            ty,
            "fixed_record_main fields must be Fixed<N>",
        ));
    };

    // 最後のセグメント（Fixed<N>）を取得
    let Some(segment) = tp.path.segments.last() else {
        return Err(syn::Error::new_spanned(
            ty,
            "fixed_record_main fields must be Fixed<N>",
        ));
    };

    if segment.ident != "Fixed" {
        return Err(syn::Error::new_spanned(
            ty,
            "fixed_record_main fields must be Fixed<N>",
        ));
    }

    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return Err(syn::Error::new_spanned(
            ty,
            "Fixed field must specify a byte length, for example Fixed<8>",
        ));
    };

    // 最初のジェネリクス引数 <N> を取得
    let Some(GenericArgument::Const(Expr::Lit(expr_lit))) = args.args.first() else {
        return Err(syn::Error::new_spanned(
            ty,
            "Fixed<N> length must be an integer literal",
        ));
    };

    let Lit::Int(lit_int) = &expr_lit.lit else {
        return Err(syn::Error::new_spanned(
            ty,
            "Fixed<N> length must be an integer literal",
        ));
    };

    lit_int
        .base10_parse::<usize>()
        .map_err(|err| syn::Error::new_spanned(lit_int, format!("invalid Fixed<N> length: {err}")))
}
