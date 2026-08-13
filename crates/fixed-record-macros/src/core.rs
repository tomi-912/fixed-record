use crate::helpers::extract_fixed_len;
use heck::AsPascalCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields};

#[derive(Clone, Copy)]
pub struct MacroOptions {
    pub clear_byte: u8,
}

/// Internal metadata for a field name, byte size, offset, and enum variant.
/// フィールド名、サイズ、オフセット、バリアント名をまとめた内部用構造体です。
pub(super) struct FieldMeta<'a> {
    pub(super) name: &'a syn::Ident,
    pub(super) size: usize,
    pub(super) offset: usize,
    pub(super) variant: syn::Ident,
    pub(super) doc_attrs: Vec<syn::Attribute>,
}

/// Collects all field metadata from the input struct.
/// フィールド情報を一括解析する補助関数です。
fn collect_field_meta(input: &DeriveInput) -> syn::Result<Vec<FieldMeta<'_>>> {
    let fields_raw = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(f) => &f.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    &data.fields,
                    "fixed_record supports only structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "fixed_record can only be used on structs",
            ));
        }
    };

    let mut current_offset = 0usize;
    fields_raw
        .iter()
        .map(|f| {
            let Some(name) = f.ident.as_ref() else {
                return Err(syn::Error::new_spanned(
                    f,
                    "fixed_record supports only named fields",
                ));
            };
            let size = extract_fixed_len(&f.ty)?;
            let offset = current_offset;
            current_offset += size;
            let doc_attrs = f
                .attrs
                .iter()
                .filter(|attr| attr.path().is_ident("doc"))
                .cloned()
                .collect();

            Ok(FieldMeta {
                name,
                size,
                offset,
                variant: format_ident!("{}", AsPascalCase(name.to_string()).to_string()),
                doc_attrs,
            })
        })
        .collect()
}

/// Generates the field identifier enum, such as `{StructName}Field`.
/// フィールド識別用列挙型 `{StructName}Field` を生成します。
pub fn gen_field_enum(input: &DeriveInput) -> syn::Result<TokenStream> {
    let struct_name = &input.ident;
    let struct_vis = &input.vis;
    let field_enum_name = format_ident!("{}Field", struct_name);
    let metas = collect_field_meta(input)?;

    let variants = metas.iter().map(|m| {
        let v = &m.variant;
        let docs = &m.doc_attrs;
        quote! {
            #( #docs )*
            #v
        }
    });
    Ok(quote! {
        #[doc = "Field identifier enum for the generated record."]
        #[doc = "生成レコード用のフィールド識別列挙型です。"]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #struct_vis enum #field_enum_name {
            #( #variants ),*
        }
    })
}

/// Validates the attribute macro input and generates the record, field enum, and implementations.
/// attribute macro の入力を検証し、構造体・フィールド enum・実装一式の token を生成します。
pub fn expand_fixed_record(input: &DeriveInput, options: MacroOptions) -> syn::Result<TokenStream> {
    let struct_name = &input.ident;
    let field_enum_name = format_ident!("{}Field", struct_name);
    let field_count = collect_field_meta(input)?.len();
    let field_enum = gen_field_enum(input)?;
    let impl_block = impl_fixed_record_core(input, options)?;
    let sequence_field_impls = (0..=field_count).map(|len| {
        quote! {
            impl ::fixed_record::traits::SequenceFields<#struct_name> for [#field_enum_name; #len] {
                #[doc = "Returns the fields used by `Reader` sequence checks as a `Vec`."]
                #[doc = "`Reader` のシーケンスチェック対象フィールドを `Vec` にして返します。"]
                fn to_sequence_fields(self) -> Vec<#field_enum_name> {
                    self.to_vec()
                }
            }

            impl<'a> ::fixed_record::traits::SequenceFields<#struct_name> for &'a [#field_enum_name; #len] {
                #[doc = "Returns the fields used by `Reader` sequence checks as a `Vec`."]
                #[doc = "`Reader` のシーケンスチェック対象フィールドを `Vec` にして返します。"]
                fn to_sequence_fields(self) -> Vec<#field_enum_name> {
                    self.to_vec()
                }
            }
        }
    });
    Ok(quote! {
        #[repr(C)]
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            ::fixed_record::zerocopy::FromBytes,
            ::fixed_record::zerocopy::IntoBytes,
            ::fixed_record::zerocopy::Immutable,
            ::fixed_record::zerocopy::KnownLayout
        )]
        #[zerocopy(crate = "fixed_record::zerocopy")]
        #input

        #field_enum

        #( #sequence_field_impls )*

        #impl_block
    })
}

/// Generates methods, trait implementations, and list types for fixed-width record usage.
/// 固定長レコードとして使うためのメソッド、trait 実装、リスト型を生成します。
pub fn impl_fixed_record_core(
    input: &syn::DeriveInput,
    options: MacroOptions,
) -> syn::Result<proc_macro2::TokenStream> {
    let struct_name = &input.ident;
    let field_enum_name = format_ident!("{}Field", struct_name);
    let metas = collect_field_meta(input)?;
    let clear_byte = options.clear_byte;
    let total_len: usize = metas.iter().map(|m| m.size).sum();
    let field_names: Vec<_> = metas.iter().map(|m| m.name).collect();
    let metas_variants: Vec<_> = metas.iter().map(|m| &m.variant).collect();
    let to_bytes_blocks = metas.iter().map(|m| {
        let name = m.name;
        let offset = m.offset;
        let size = m.size;
        quote! {
            out[#offset..#offset + #size].copy_from_slice(self.#name.as_bytes());
        }
    });
    let parse_field_inits = metas.iter().map(|m| {
        let name = m.name;
        let offset = m.offset;
        let size = m.size;
        quote! {
            #name: ::fixed_record::Fixed::<#size>::from_slice(&src[#offset..#offset + #size])?
        }
    });
    let get_field_bytes_arms = metas.iter().map(|m| {
        let name = m.name;
        let variant = &m.variant;
        quote! {
            #field_enum_name::#variant => self.#name.as_bytes()
        }
    });

    // Generate field-specific getters and builder-style setters.
    // フィールドごとの getter と builder 形式の setter を生成します。
    let field_methods = metas.iter().map(|m| {
        let name = m.name;
        let name_str = quote::format_ident!("{}_str", name);
        let with_name = quote::format_ident!("with_{}", name);
        let with_name_int = quote::format_ident!("with_{}_int", name);
        let with_name_signed = quote::format_ident!("with_{}_int_signed", name);
        let try_with_name_int = quote::format_ident!("try_with_{}_int", name);
        let try_with_name_signed = quote::format_ident!("try_with_{}_int_signed", name);
        let with_name_int_truncated = quote::format_ident!("with_{}_int_truncated", name);
        let with_name_signed_truncated = quote::format_ident!("with_{}_int_signed_truncated", name);
        let size = m.size;
        let docs = &m.doc_attrs;

        quote! {
            #( #docs )*
            #[doc = "Returns the field as bytes."]
            #[doc = "フィールドをバイト列として返します。"]
            pub fn #name(&self) -> &[u8] {
                self.#name.as_bytes()
            }

            #( #docs )*
            #[doc = "Returns the field as a UTF-8 string."]
            #[doc = "フィールドを UTF-8 文字列として参照します。"]
            pub fn #name_str(&self) -> Result<&str, ::fixed_record::error::Error> {
                self.#name.as_str()
            }

            #( #docs )*
            #[doc = "Overwrites the field from the beginning with a string."]
            #[doc = "フィールドに文字列を先頭から上書きします。"]
            #[doc = "The field is not cleared before writing, so shorter strings leave trailing bytes intact."]
            #[doc = "書き込み前のクリアは行わないため、短い文字列では後続バイトが残ります。"]
            pub fn #with_name(mut self, s: &str) -> Self {
                self.#name.write_bytes(s.as_bytes());
                self
            }

            #( #docs )*
            #[doc = "Sets the field to a zero-padded numeric string."]
            #[doc = "フィールドに数値をゼロ埋め文字列としてセットします。"]
            #[doc = "Values wider than the field are truncated and reported to stderr."]
            #[doc = "値がフィールド幅を超える場合は切り捨て、stderr に警告を出します。"]
            pub fn #with_name_int(self, val: i64) -> Self {
                self.#with_name_int_truncated(val)
            }

            #( #docs )*
            #[doc = "Sets the field to a zero-padded numeric string."]
            #[doc = "フィールドに数値をゼロ埋め文字列としてセットします。"]
            #[doc = "Returns `Error::FieldOverflow` when the value is wider than the field."]
            #[doc = "値がフィールド幅を超える場合は `Error::FieldOverflow` を返します。"]
            pub fn #try_with_name_int(self, val: i64) -> Result<Self, ::fixed_record::error::Error> {
                let s = format!("{:0>width$}", val, width = #size);
                let actual = s.as_bytes().len();
                if actual > #size {
                    return Err(::fixed_record::error::Error::FieldOverflow {
                        field: stringify!(#name),
                        size: #size,
                        actual,
                    });
                }
                Ok(self.#with_name(&s))
            }

            #( #docs )*
            #[doc = "Sets the field to a zero-padded numeric string and keeps only the leading bytes when it overflows."]
            #[doc = "フィールドに数値をゼロ埋め文字列としてセットし、幅を超えた場合は先頭側だけ残します。"]
            #[doc = "Truncation is reported to stderr."]
            #[doc = "切り捨てが発生した場合は stderr に警告を出します。"]
            pub fn #with_name_int_truncated(self, val: i64) -> Self {
                let s = format!("{:0>width$}", val, width = #size);
                let actual = s.as_bytes().len();
                if actual > #size {
                    eprintln!(
                        "fixed_record: truncating field `{}` from {} bytes to {} bytes",
                        stringify!(#name),
                        actual,
                        #size
                    );
                }
                self.#with_name(&s)
            }

            #( #docs )*
            #[doc = "Sets the field to a zero-padded signed numeric string prefixed with `+` or `-`."]
            #[doc = "フィールドに符号付き数値を + または - を先頭にしてゼロ埋めでセットします。"]
            #[doc = "Values wider than the field are truncated and reported to stderr."]
            #[doc = "値がフィールド幅を超える場合は切り捨て、stderr に警告を出します。"]
            pub fn #with_name_signed(self, val: i64) -> Self {
                self.#with_name_signed_truncated(val)
            }

            #( #docs )*
            #[doc = "Sets the field to a zero-padded signed numeric string prefixed with `+` or `-`."]
            #[doc = "フィールドに符号付き数値を + または - を先頭にしてゼロ埋めでセットします。"]
            #[doc = "Returns `Error::FieldOverflow` when the value is wider than the field."]
            #[doc = "値がフィールド幅を超える場合は `Error::FieldOverflow` を返します。"]
            pub fn #try_with_name_signed(self, val: i64) -> Result<Self, ::fixed_record::error::Error> {
                let sign = if val < 0 { '-' } else { '+' };
                let abs = val.unsigned_abs();

                // Remaining width is total size minus one byte for the sign.
                // 残りの幅は、全体サイズから符号分の1バイトを引いた値です。
                let rest = #size - 1;

                let s = format!("{}{:0>width$}", sign, abs, width = rest);
                let actual = s.as_bytes().len();
                if actual > #size {
                    return Err(::fixed_record::error::Error::FieldOverflow {
                        field: stringify!(#name),
                        size: #size,
                        actual,
                    });
                }
                Ok(self.#with_name(&s))
            }

            #( #docs )*
            #[doc = "Sets the field to a signed numeric string and keeps only the leading bytes when it overflows."]
            #[doc = "フィールドに符号付き数値をセットし、幅を超えた場合は先頭側だけ残します。"]
            #[doc = "Truncation is reported to stderr."]
            #[doc = "切り捨てが発生した場合は stderr に警告を出します。"]
            pub fn #with_name_signed_truncated(self, val: i64) -> Self {
                let sign = if val < 0 { '-' } else { '+' };
                let abs = val.unsigned_abs();

                // Remaining width is total size minus one byte for the sign.
                // 残りの幅は、全体サイズから符号分の1バイトを引いた値です。
                let rest = #size - 1;

                let s = format!("{}{:0>width$}", sign, abs, width = rest);
                let actual = s.as_bytes().len();
                if actual > #size {
                    eprintln!(
                        "fixed_record: truncating field `{}` from {} bytes to {} bytes",
                        stringify!(#name),
                        actual,
                        #size
                    );
                }
                self.#with_name(&s)
            }
        }
    });
    // Generate constants and match arms.
    // 各定数と match arm を生成します。
    let size_consts = metas.iter().map(|m| {
        let c_name = format_ident!("FIELD_SIZE_{}", m.name.to_string().to_uppercase());
        let size = m.size;
        quote!(pub const #c_name: usize = #size;)
    });
    let size_arms = metas.iter().map(|m| {
        let v = &m.variant;
        let s = m.size;
        quote!(#field_enum_name::#v => #s)
    });
    let offset_arms = metas.iter().map(|m| {
        let v = &m.variant;
        let o = m.offset;
        quote!(#field_enum_name::#v => #o)
    });
    let name_arms = metas.iter().map(|m| {
        let v = &m.variant;
        let n = m.name.to_string();
        quote!(#field_enum_name::#v => #n)
    });
    let all_variants = metas.iter().map(|m| &m.variant);
    let list_impl = crate::list::gen_list_impl(input, &metas);

    Ok(quote! {
        impl #struct_name {
            #[doc = "Total byte length of the record."]
            #[doc = "レコード全体の合計バイト長です。"]
            pub const TOTAL_LEN: usize = #total_len;

            #[doc = "Byte value used by `set_field_*` methods to clear a field before writing."]
            #[doc = "`set_field_*` が書き込み前にフィールドをクリアするときのバイト値です。"]
            pub const CLEAR_BYTE: u8 = #clear_byte;

            #(
                #[doc = "Byte length constant for the field."]
                #[doc = "フィールドのバイト長定数です。"]
                #size_consts
            )*

            #( #field_methods )*

            #[doc = "Returns the byte length of the specified field."]
            #[doc = "指定したフィールドのバイト長を返します。"]
            pub const fn size_of(field: #field_enum_name) -> usize {
                match field { #( #size_arms ),* }
            }

            #[doc = "Returns the byte offset from the start of the record to the specified field."]
            #[doc = "レコードの先頭から指定したフィールドまでのバイトオフセットを返します。"]
            pub const fn offset_of(field: #field_enum_name) -> usize {
                match field { #( #offset_arms ),* }
            }

            #[doc = "Returns the declaration name of the specified field."]
            #[doc = "指定したフィールドの定義名を文字列として返します。"]
            pub const fn name_of(field: #field_enum_name) -> &'static str {
                match field { #( #name_arms ),* }
            }

            #[doc = "Returns all fields defined on this struct."]
            #[doc = "この構造体に定義されているすべてのフィールドのリストを返します。"]
            pub const fn all_fields() -> &'static [#field_enum_name] {
                &[ #( #field_enum_name::#all_variants ),* ]
            }
            #[doc = "Creates a new record with every field filled with `0x00`."]
            #[doc = "全フィールドを `0x00` で埋めた新しいインスタンスを生成します。"]
            pub const fn zeroed() -> Self {
                Self {
                    #( #field_names: ::fixed_record::types::Fixed::zeroed() ),*
                }
            }

            #[doc = "Creates a new record with every field filled with spaces (`0x20`)."]
            #[doc = "全フィールドをスペース (`0x20`) で埋めた新しいインスタンスを生成します。"]
            pub const fn spaced() -> Self {
                Self {
                    #( #field_names: ::fixed_record::types::Fixed::spaced() ),*
                }
            }

            #[doc = "Creates a new record with every field filled with `CLEAR_BYTE`."]
            #[doc = "全フィールドを `CLEAR_BYTE` で埋めた新しいインスタンスを生成します。"]
            pub const fn cleared() -> Self {
                Self {
                    #( #field_names: ::fixed_record::types::Fixed::filled(Self::CLEAR_BYTE) ),*
                }
            }

            #[doc = "Copies the record into a fixed-width byte array."]
            #[doc = "インスタンスを固定長バイト配列としてコピーして返します。"]
            pub fn to_bytes(&self) -> [u8; Self::TOTAL_LEN] {
                let mut out = [0u8; Self::TOTAL_LEN];
                #( #to_bytes_blocks )*
                out
            }

            #[doc = "Returns the total byte length of this record."]
            #[doc = "インスタンスの合計バイト長を返します。"]
            pub const fn byte_len(&self) -> usize {
                Self::TOTAL_LEN
            }

            #[doc = "Returns the total byte length of the specified fields."]
            #[doc = "指定されたフィールドリストの合計バイト長を返します。"]
            pub fn byte_len_fields(fields: &[#field_enum_name]) -> usize {
                fields.iter().map(|field| Self::size_of(*field)).sum()
            }

            #[doc = "Creates a new builder value initialized with `CLEAR_BYTE`."]
            #[doc = "`CLEAR_BYTE` で初期化した新しいビルダーインスタンスを生成します。"]
            pub fn builder() -> Self {
                Self::cleared()
            }

            #[doc = "Finishes building and returns the record value."]
            #[doc = "ビルドを完了し、インスタンスを返します。"]
            pub fn build(self) -> Self {
                self
            }

            #[doc = "Reads bytes and creates a new owned record value."]
            #[doc = "バイト列を読み取って、構造体の新しいインスタンス（所有権あり）を作成します。"]
            pub fn parse(src: &[u8]) -> Result<Self, ::fixed_record::error::Error> {
                if src.len() < Self::TOTAL_LEN {
                    return Err(::fixed_record::error::Error::TooShort);
                }
                Ok(Self {
                    #( #parse_field_inits ),*
                })
            }

            #[doc = "Converts a string into a record by reading it as bytes."]
            #[doc = "文字列をバイト列として読み取り、構造体へ変換します。"]
            pub fn parse_str(src: &str) -> Result<Self, ::fixed_record::error::Error> {
                Self::parse(src.as_bytes())
            }

            #[doc = "Reads a string as a zero-copy shared reference when it is exactly one record wide."]
            #[doc = "文字列を、ちょうど1レコード分の幅である場合にコピーせず共有参照として読み取ります。"]
            pub fn ref_from_str(src: &str) -> Result<&Self, ::fixed_record::error::Error> {
                if src.len() < Self::TOTAL_LEN {
                    return Err(::fixed_record::error::Error::TooShort);
                }
                if src.len() > Self::TOTAL_LEN {
                    return Err(::fixed_record::error::Error::ParseError);
                }

                <Self as ::fixed_record::zerocopy::FromBytes>::ref_from_bytes(src.as_bytes())
                    .map_err(|_| ::fixed_record::error::Error::AlignmentError)
            }

            #[doc = "Reads the first record-width bytes as a zero-copy shared reference."]
            #[doc = "先頭の1レコード分のバイト列を、コピーせず共有参照として読み取ります。"]
            #[doc = "Unlike `zerocopy::FromBytes::ref_from_bytes`, extra trailing bytes are accepted."]
            #[doc = "`zerocopy::FromBytes::ref_from_bytes` と異なり、後続の余りバイトを許容します。"]
            pub fn ref_from_bytes_prefix(src: &[u8]) -> Result<&Self, ::fixed_record::error::Error> {
                if src.len() < Self::TOTAL_LEN {
                    return Err(::fixed_record::error::Error::TooShort);
                }

                <Self as ::fixed_record::zerocopy::FromBytes>::ref_from_bytes(
                    &src[..Self::TOTAL_LEN],
                )
                .map_err(|_| ::fixed_record::error::Error::AlignmentError)
            }

            #[doc = "Reads the first record-width bytes of a string as a zero-copy shared reference."]
            #[doc = "文字列の先頭1レコード分のバイト列を、コピーせず共有参照として読み取ります。"]
            pub fn ref_from_str_prefix(src: &str) -> Result<&Self, ::fixed_record::error::Error> {
                Self::ref_from_bytes_prefix(src.as_bytes())
            }

            // Dynamic field operations.
            // フィールド操作（動的アクセス）です。

            #[doc = "Returns the raw bytes of the specified field."]
            #[doc = "指定フィールドの生バイト列を返します。"]
            pub fn get_field_bytes(&self, field: #field_enum_name) -> &[u8] {
                match field {
                    #( #get_field_bytes_arms ),*
                }
            }

            #[doc = "Returns the specified field as a string after UTF-8 validation."]
            #[doc = "指定フィールドを文字列として取得します（UTF-8チェック）。"]
            pub fn get_field_str(&self, field: #field_enum_name) -> Result<&str, ::fixed_record::error::Error> {
                std::str::from_utf8(self.get_field_bytes(field))
                    .map_err(|_| ::fixed_record::error::Error::Utf8Error)
            }

            #[doc = "Returns a string slice with leading and trailing spaces or NUL bytes removed."]
            #[doc = "フィールドから前後の空白やヌル文字を取り除いた文字列スライスを取得します。"]
            pub fn get_field_trimmed(&self, field: #field_enum_name) -> Result<&str, ::fixed_record::error::Error> {
                Ok(self.get_field_str(field)?.trim_matches(|c: char| c == ' ' || c == '\0'))
            }

            #[doc = "Returns the specified field as a trimmed `String`."]
            #[doc = "指定フィールドをトリミング済みの `String` として取得します。"]
            pub fn get_field_string_trimmed(&self, field: #field_enum_name) -> Result<String, ::fixed_record::error::Error> {
                self.get_field_trimmed(field).map(|s| s.to_string())
            }

            #[doc = "Trims the field and parses it into any `FromStr` type."]
            #[doc = "フィールドをトリミングした後、任意の `FromStr` 型にパースして取得します。"]
            pub fn get_field_as<T: std::str::FromStr>(&self, field: #field_enum_name) -> Result<T, ::fixed_record::error::Error> {
                self.get_field_trimmed(field)?
                    .parse::<T>()
                    .map_err(|_| ::fixed_record::error::Error::ParseError)
            }

            #[doc = "Returns the whole record as a UTF-8 string slice."]
            #[doc = "レコード全体を UTF-8 文字列スライスとして返します。"]
            pub fn as_str(&self) -> Result<&str, ::fixed_record::error::Error> {
                std::str::from_utf8(<Self as ::fixed_record::zerocopy::IntoBytes>::as_bytes(self))
                    .map_err(|_| ::fixed_record::error::Error::Utf8Error)
            }

            #[doc = "Fills the specified field with `0x00` bytes."]
            #[doc = "指定したフィールドを `0x00` で埋めます。"]
            pub fn fill_field_zero(&mut self, field: #field_enum_name) {
                match field {
                    #(
                        #field_enum_name::#metas_variants => {
                            self.#field_names.fill_zero();
                        }
                    ),*
                }
            }

            #[doc = "Fills the specified field with spaces (`0x20`)."]
            #[doc = "指定したフィールドを半角スペース (`0x20`) で埋めます。"]
            pub fn fill_field_space(&mut self, field: #field_enum_name) {
                match field {
                    #(
                        #field_enum_name::#metas_variants => {
                            self.#field_names.fill_space();
                        }
                    ),*
                }
            }

            #[doc = "Fills the specified field with `CLEAR_BYTE`."]
            #[doc = "指定したフィールドを `CLEAR_BYTE` で埋めます。"]
            pub fn fill_field_clear(&mut self, field: #field_enum_name) {
                match field {
                    #(
                        #field_enum_name::#metas_variants => {
                            self.#field_names.fill(Self::CLEAR_BYTE);
                        }
                    ),*
                }
            }

            #[doc = "Clears the specified field with `CLEAR_BYTE` and then writes bytes into it."]
            #[doc = "特定フィールドを `CLEAR_BYTE` でクリアしてからバイト列を書き込みます。"]
            pub fn set_field_bytes(&mut self, field: #field_enum_name, data: &[u8]) {
                match field {
                    #(
                        #field_enum_name::#metas_variants => {
                            self.#field_names.fill(Self::CLEAR_BYTE);
                            self.#field_names.write_bytes(data);
                        }
                    ),*
                }
            }

            #[doc = "Overwrites the specified field from the beginning with bytes."]
            #[doc = "特定フィールドにバイト列を先頭から上書きします。"]
            #[doc = "The field is not cleared before writing, so shorter inputs leave trailing bytes intact."]
            #[doc = "書き込み前のクリアは行わないため、短い入力では後続バイトが残ります。"]
            pub fn set_field_bytes_no_clear(&mut self, field: #field_enum_name, data: &[u8]) {
                match field {
                    #(
                        #field_enum_name::#metas_variants => {
                            self.#field_names.write_bytes(data);
                        }
                    ),*
                }
            }

            #[doc = "Clears the specified field with `CLEAR_BYTE` and then writes a string into it."]
            #[doc = "特定フィールドを `CLEAR_BYTE` でクリアしてから文字列を書き込みます。"]
            pub fn set_field_str(&mut self, field: #field_enum_name, s: &str) {
                match field {
                    #(
                        #field_enum_name::#metas_variants => {
                            self.#field_names.fill(Self::CLEAR_BYTE);
                            self.#field_names.write_bytes(s.as_bytes());
                        }
                    ),*
                }
            }

            #[doc = "Overwrites the specified field from the beginning with a string."]
            #[doc = "特定フィールドに文字列を先頭から上書きします。"]
            #[doc = "The field is not cleared before writing, so shorter strings leave trailing bytes intact."]
            #[doc = "書き込み前のクリアは行わないため、短い文字列では後続バイトが残ります。"]
            pub fn set_field_str_no_clear(&mut self, field: #field_enum_name, s: &str) {
                self.set_field_bytes_no_clear(field, s.as_bytes());
            }

            #[doc = "Fills all fields with `0x00` bytes."]
            #[doc = "すべてのフィールドを `0x00` で一括上書きします。"]
            pub fn fill_zero(&mut self) {
                #( self.#field_names.fill_zero(); )*
            }

            #[doc = "Fills all fields with spaces (`0x20`)."]
            #[doc = "すべてのフィールドを半角スペース (`0x20`) で一括上書きします。"]
            pub fn fill_space(&mut self) {
                #( self.#field_names.fill_space(); )*
            }

            // Bulk application helpers.
            // 一括適用・流し込みの補助メソッドです。

            #[doc = "Passes `self` to a closure for method-chain friendly mutation."]
            #[doc = "自身をクロージャに渡して加工する、メソッドチェーン向けの汎用メソッドです。"]
            pub fn apply<F>(mut self, f: F) -> Self
            where F: FnOnce(&mut Self) {
                f(&mut self);
                self
            }

            #[doc = "Applies bytes field by field from the first field."]
            #[doc = "先頭フィールドから順に、渡されたバイト列を各フィールドの長さ分ずつ流し込みます。"]
            pub fn apply_bytes(self, data: &[u8]) -> Self {
                self.apply_bytes_from(Self::all_fields()[0], data)
            }

            #[doc = "Applies a string field by field from the first field."]
            #[doc = "先頭フィールドから順に、渡された文字列を流し込みます。"]
            pub fn apply_str(self, s: &str) -> Self {
                self.apply_bytes(s.as_bytes())
            }

            #[doc = "Applies bytes sequentially from the specified starting field."]
            #[doc = "開始フィールドを指定して、そこから順次データを流し込みます。"]
            pub fn apply_bytes_from(self, start_field: #field_enum_name, data: &[u8]) -> Self {
                let mut this = self;
                let mut current_pos = 0;

                for &field in Self::all_fields()
                    .iter()
                    .skip_while(|&&f| f != start_field)
                {
                    if current_pos >= data.len() { break; }
                    let field_size = Self::size_of(field);
                    let end_pos = (current_pos + field_size).min(data.len());
                    this.set_field_bytes(field, &data[current_pos..end_pos]);
                    current_pos += field_size;
                }

                this
            }

            #[doc = "Applies string data sequentially from the specified starting field."]
            #[doc = "開始フィールドを指定して、そこから順次文字列データを流し込みます。"]
            pub fn apply_str_from(self, start_field: #field_enum_name, s: &str) -> Self {
                self.apply_bytes_from(start_field, s.as_bytes())
            }

            #[doc = "Compares all fields in declaration order."]
            #[doc = "全フィールドを定義順に比較します。"]
            pub fn compare_all_fields(&self, other: &Self) -> std::cmp::Ordering {
                use std::cmp::Ordering;
                #(
                    let ordering = self.#field_names.cmp(&other.#field_names);
                    if ordering != Ordering::Equal {
                        return ordering;
                    }
                )*
                Ordering::Equal
            }

            #[doc = "Compares records using the specified field priority order."]
            #[doc = "指定されたフィールドの優先順に従って比較します。"]
            pub fn compare_by_fields(&self, other: &Self, fields: &[#field_enum_name]) -> std::cmp::Ordering {
                use std::cmp::Ordering;
                for field in fields {
                    let ordering = self.get_field_bytes(*field).cmp(other.get_field_bytes(*field));
                    if ordering != Ordering::Equal {
                        return ordering;
                    }
                }
                Ordering::Equal
            }

            #[doc = "Dumps all fields of the record to standard output."]
            #[doc = "レコードの全フィールドを標準出力へダンプします。"]
            pub fn dump(&self) {
                println!("{}", self.to_dump_string());
            }

            #[doc = "Returns all fields of the record as a dump string."]
            #[doc = "レコードの全フィールドをダンプ文字列として返します。"]
            pub fn to_dump_string(&self) -> String {
                let mut s = format!("--- {} Dump ---\n", stringify!(#struct_name));
                #(
                    let value = self.#field_names.as_str().unwrap_or("[Invalid UTF-8]");
                    s.push_str(&format!("{:<15}: [{}]\n", stringify!(#field_names), value));
                )*
                s.push_str("------------------\n");
                s
            }
        }

        impl #field_enum_name {
            #[doc = "Returns the field name as a string."]
            #[doc = "フィールド名を文字列として返します。"]
            pub const fn as_str(&self) -> &'static str {
                match self {
                    #( Self::#metas_variants => #struct_name::name_of(*self), )*
                }
            }

            #[doc = "Returns the declared byte size of this field."]
            #[doc = "このフィールドの定義サイズを返します。"]
            pub const fn size(&self) -> usize {
                match self {
                    #( Self::#metas_variants => #struct_name::size_of(*self), )*
                }
            }
        }

        impl ::fixed_record::FixedRecord for #struct_name {
            type Field = #field_enum_name;

            const TOTAL_LEN: usize = #struct_name::TOTAL_LEN;

            #[doc = "Creates a record from fixed-width bytes."]
            #[doc = "固定長バイト列からレコードを作成します。"]
            fn parse(src: &[u8]) -> Result<Self, ::fixed_record::Error> {
                #struct_name::parse(src)
            }

            #[doc = "Copies the record as fixed-width bytes."]
            #[doc = "レコードを固定長バイト列としてコピーして返します。"]
            fn to_bytes(&self) -> Vec<u8> {
                #struct_name::to_bytes(self).to_vec()
            }

            #[doc = "Returns the declaration name of the specified field."]
            #[doc = "指定フィールドの定義名を返します。"]
            fn field_name(field: Self::Field) -> &'static str {
                #struct_name::name_of(field)
            }

            #[doc = "Returns the bytes stored in the specified field."]
            #[doc = "指定フィールドのバイト列を返します。"]
            fn field_bytes(&self, field: Self::Field) -> &[u8] {
                self.get_field_bytes(field)
            }
        }

        impl Default for #struct_name {
            #[doc = "Calls `cleared()` to initialize the record with `CLEAR_BYTE`."]
            #[doc = "`cleared()` を呼び出して `CLEAR_BYTE` で初期化します。"]
            fn default() -> Self {
                Self::cleared()
            }
        }

        #list_impl
    })
}
