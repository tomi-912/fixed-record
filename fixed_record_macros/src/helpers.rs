use syn::{Expr, GenericArgument, Lit, PathArguments, Type, UnOp};

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
    let Some(GenericArgument::Const(expr)) = args.args.first() else {
        return Err(syn::Error::new_spanned(
            ty,
            "Fixed<N> length must be an integer literal",
        ));
    };

    if matches!(expr, Expr::Unary(expr_unary) if matches!(expr_unary.op, UnOp::Neg(_))) {
        return Err(syn::Error::new_spanned(
            expr,
            "Fixed<N> length must not be negative",
        ));
    }

    let Expr::Lit(expr_lit) = expr else {
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

    if lit_int.to_string().starts_with('-') {
        return Err(syn::Error::new_spanned(
            lit_int,
            "Fixed<N> length must not be negative",
        ));
    }

    let len = lit_int.base10_parse::<usize>().map_err(|err| {
        syn::Error::new_spanned(lit_int, format!("invalid Fixed<N> length: {err}"))
    })?;

    if len == 0 {
        return Err(syn::Error::new_spanned(
            lit_int,
            "Fixed<N> length must be greater than 0",
        ));
    }

    Ok(len)
}
