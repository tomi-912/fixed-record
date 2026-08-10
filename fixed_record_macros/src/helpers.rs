use syn::{Expr, GenericArgument, Lit, PathArguments, Type};

/// 型が Fixed<N> であることを確認し、N の数値を取得する
pub fn extract_fixed_len(ty: &Type) -> usize {
    if let Type::Path(tp) = ty {
        // 最後のセグメント（Fixed<N>）を取得
        let segment = tp.path.segments.last().expect("Type path is empty");

        if segment.ident == "Fixed" {
            if let PathArguments::AngleBracketed(args) = &segment.arguments {
                // 最初のジェネリクス引数 <N> を取得
                if let Some(GenericArgument::Const(Expr::Lit(expr_lit))) = args.args.first() {
                    if let Lit::Int(lit_int) = &expr_lit.lit {
                        return lit_int
                            .base10_parse::<usize>()
                            .expect("Failed to parse N in Fixed<N>");
                    }
                }
            }
        }
    }
    panic!("FixedRecord fields must be of type 'Fixed<N>'");
}
