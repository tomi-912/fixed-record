use crate::helpers::extract_fixed_len;
use heck::AsPascalCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields};

#[derive(Clone, Copy)]
pub struct MacroOptions {
    pub clear_byte: u8,
}

/// フィールド名、サイズ、オフセット、バリアント名をまとめた内部用構造体
struct FieldMeta<'a> {
    name: &'a syn::Ident,
    size: usize,
    offset: usize,
    variant: syn::Ident,
    doc_attrs: Vec<syn::Attribute>,
}

/// フィールド情報を一括解析する補助関数
fn collect_field_meta(input: &DeriveInput) -> syn::Result<Vec<FieldMeta<'_>>> {
    let fields_raw = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(f) => &f.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    &data.fields,
                    "fixed_record_main supports only structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "fixed_record_main can only be used on structs",
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
                    "fixed_record_main supports only named fields",
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

/// フィールド識別用列挙型 (#struct_nameField) を生成する
pub fn gen_field_enum(input: &DeriveInput) -> syn::Result<TokenStream> {
    let struct_name = &input.ident;
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
        #[doc = "フィールド識別用の列挙型です。"]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum #field_enum_name {
            #( #variants ),*
        }
    })
}

/// attribute macro の入力を検証し、構造体・フィールド enum・実装一式の token を生成します。
pub fn expand_fixed_record_main(
    input: &DeriveInput,
    options: MacroOptions,
) -> syn::Result<TokenStream> {
    let field_enum = gen_field_enum(input)?;
    let impl_block = impl_fixed_record_core(input, options)?;
    let repr_attr = if cfg!(feature = "unchecked") {
        quote!(#[repr(C)])
    } else {
        quote!()
    };

    Ok(quote! {
        #repr_attr
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #input

        #field_enum

        #impl_block
    })
}

/// 固定長レコードとして使うためのメソッド、trait 実装、リスト型を生成します。
pub fn impl_fixed_record_core(
    input: &syn::DeriveInput,
    options: MacroOptions,
) -> syn::Result<proc_macro2::TokenStream> {
    let struct_name = &input.ident;
    let field_enum_name = format_ident!("{}Field", struct_name);
    let entry_name = format_ident!("{}Entry", struct_name);
    let list_name = format_ident!("{}List", struct_name);
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
            #name: ::fixed_record_main::Fixed::<#size>::from_slice(&src[#offset..#offset + #size])?
        }
    });
    let get_field_bytes_arms = metas.iter().map(|m| {
        let name = m.name;
        let variant = &m.variant;
        quote! {
            #field_enum_name::#variant => self.#name.as_bytes()
        }
    });
    let unchecked_methods = if cfg!(feature = "unchecked") {
        quote! {
            #[doc = "構造体のメモリ配置を固定長バイト配列参照として直接読み替えます。"]
            #[doc = ""]
            #[doc = "# Safety"]
            #[doc = "この関数は構造体のメモリレイアウトが固定長レコードのバイト配置と完全に一致する場合にのみ安全です。"]
            #[doc = "padding や alignment の影響を受ける可能性があるため、通常は `to_bytes` を使ってください。"]
            pub unsafe fn as_bytes_unchecked(&self) -> &[u8; Self::TOTAL_LEN] {
                unsafe { &*(self as *const Self as *const [u8; Self::TOTAL_LEN]) }
            }

            #[doc = "バイト列を構造体のメモリへ直接コピーして、構造体の新しいインスタンスを作成します。"]
            #[doc = ""]
            #[doc = "# Safety"]
            #[doc = "この関数は構造体のメモリレイアウトが固定長レコードのバイト配置と完全に一致する場合にのみ安全です。"]
            #[doc = "padding や alignment の影響を受ける可能性があるため、通常は `parse` を使ってください。"]
            pub unsafe fn parse_unchecked(src: &[u8]) -> Result<Self, ::fixed_record_main::error::Error> {
                if src.len() < Self::TOTAL_LEN {
                    return Err(::fixed_record_main::error::Error::TooShort);
                }
                let mut inst = Self::zeroed();
                unsafe {
                    std::ptr::copy_nonoverlapping(src.as_ptr(), &mut inst as *mut Self as *mut u8, Self::TOTAL_LEN);
                }
                Ok(inst)
            }

            #[doc = "入力されたバイト列をコピーせず、構造体の参照として直接読み取ります。"]
            #[doc = ""]
            #[doc = "# Safety"]
            #[doc = "この関数は入力バイト列のアドレス、長さ、ライフタイム、構造体のメモリレイアウトがすべて `Self` として有効な場合にのみ安全です。"]
            #[doc = "通常は所有値を返す `parse` を使ってください。"]
            pub unsafe fn from_bytes_unchecked(src: &[u8]) -> Result<&Self, ::fixed_record_main::error::Error> {
                if src.len() < Self::TOTAL_LEN {
                    return Err(::fixed_record_main::error::Error::TooShort);
                }
                if src.as_ptr() as usize % std::mem::align_of::<Self>() != 0 {
                    return Err(::fixed_record_main::error::Error::AlignmentError);
                }
                unsafe { Ok(&*(src.as_ptr() as *const Self)) }
            }

            #[doc = "入力された文字列をコピーせず、構造体の参照として直接読み取ります。"]
            #[doc = ""]
            #[doc = "# Safety"]
            #[doc = "この関数は入力文字列のアドレス、長さ、ライフタイム、構造体のメモリレイアウトがすべて `Self` として有効な場合にのみ安全です。"]
            #[doc = "通常は所有値を返す `parse_str` を使ってください。"]
            pub unsafe fn from_str_unchecked(src: &str) -> Result<&Self, ::fixed_record_main::error::Error> {
                unsafe { Self::from_bytes_unchecked(src.as_bytes()) }
            }
        }
    } else {
        quote!()
    };
    let index_insert_blocks = metas.iter().map(|m| {
        let name = m.name;
        let variant = &m.variant;
        let size = m.size;
        quote! {
            {
                let value = record.#name;
                let tree = self.indices
                    .entry(#field_enum_name::#variant)
                    .or_insert_with(|| {
                        Box::new(
                            std::collections::BTreeMap::<
                                ::fixed_record_main::Fixed<#size>,
                                std::collections::BTreeSet<usize>
                            >::new()
                        )
                    });

                if let Some(map) = tree.downcast_mut::<
                    std::collections::BTreeMap<
                        ::fixed_record_main::Fixed<#size>,
                        std::collections::BTreeSet<usize>
                    >
                >() {
                    map.entry(value).or_default().insert(id);
                }
            }
        }
    });
    let index_vacuum_blocks = metas.iter().map(|m| {
        let name = m.name;
        let variant = &m.variant;
        let size = m.size;
        quote! {
            {
                let value = record.#name;
                if let Some(tree) = self.indices.get_mut(&#field_enum_name::#variant) {
                    if let Some(map) = tree.downcast_mut::<
                        std::collections::BTreeMap<
                            ::fixed_record_main::Fixed<#size>,
                            std::collections::BTreeSet<usize>
                        >
                    >() {
                        if let Some(ids) = map.get_mut(&value) {
                            ids.remove(&id);
                            if ids.is_empty() {
                                map.remove(&value);
                            }
                        }
                    }
                }
            }
        }
    });
    let index_remove_blocks = metas.iter().map(|m| {
        let name = m.name;
        let variant = &m.variant;
        let size = m.size;
        quote! {
            {
                let value = record.#name;
                if let Some(tree) = self.indices.get_mut(&#field_enum_name::#variant) {
                    if let Some(map) = tree.downcast_mut::<
                        std::collections::BTreeMap<
                            ::fixed_record_main::Fixed<#size>,
                            std::collections::BTreeSet<usize>
                        >
                    >() {
                        if let Some(ids) = map.get_mut(&value) {
                            ids.remove(&id);
                            if ids.is_empty() {
                                map.remove(&value);
                            }
                        }
                    }
                }
            }
        }
    });
    let index_update_remove_blocks = metas.iter().map(|m| {
        let name = m.name;
        let variant = &m.variant;
        let size = m.size;
        quote! {
            {
                let value = old_record.#name;
                if let Some(tree) = self.indices.get_mut(&#field_enum_name::#variant) {
                    if let Some(map) = tree.downcast_mut::<
                        std::collections::BTreeMap<
                            ::fixed_record_main::Fixed<#size>,
                            std::collections::BTreeSet<usize>
                        >
                    >() {
                        if let Some(ids) = map.get_mut(&value) {
                            ids.remove(&id);
                            if ids.is_empty() {
                                map.remove(&value);
                            }
                        }
                    }
                }
            }
        }
    });
    let index_update_insert_blocks = metas.iter().map(|m| {
        let name = m.name;
        let variant = &m.variant;
        let size = m.size;
        quote! {
            {
                let value = record.#name;
                let tree = self.indices
                    .entry(#field_enum_name::#variant)
                    .or_insert_with(|| {
                        Box::new(
                            std::collections::BTreeMap::<
                                ::fixed_record_main::Fixed<#size>,
                                std::collections::BTreeSet<usize>
                            >::new()
                        )
                    });

                if let Some(map) = tree.downcast_mut::<
                    std::collections::BTreeMap<
                        ::fixed_record_main::Fixed<#size>,
                        std::collections::BTreeSet<usize>
                    >
                >() {
                    map.entry(value).or_default().insert(id);
                }
            }
        }
    });
    let try_find_by_arms = metas.iter().map(|m| {
        let variant = &m.variant;
        let size = m.size;
        quote! {
            #field_enum_name::#variant => {
                if raw_value.len() > #size {
                    return Err(::fixed_record_main::error::Error::FieldOverflow {
                        field: #struct_name::name_of(field),
                        size: #size,
                        actual: raw_value.len(),
                    });
                }

                let Some(tree) = self.indices.get(&field) else {
                    return Ok(Vec::new());
                };
                let Some(map) = tree.downcast_ref::<
                    std::collections::BTreeMap<
                        ::fixed_record_main::Fixed<#size>,
                        std::collections::BTreeSet<usize>
                    >
                >() else {
                    return Ok(Vec::new());
                };

                let ids: Vec<usize> = if raw_value.len() == #size {
                    let value = ::fixed_record_main::Fixed::<#size>::from_slice(raw_value)?;
                    map.get(&value)
                        .into_iter()
                        .flat_map(|ids| ids.iter().copied())
                        .collect()
                } else {
                    map.iter()
                        .filter(|(key, _)| {
                            let bytes = key.as_bytes();
                            bytes.starts_with(raw_value)
                                && bytes[raw_value.len()..]
                                    .iter()
                                    .all(|byte| *byte == 0x00 || *byte == b' ')
                        })
                        .flat_map(|(_, ids)| ids.iter().copied())
                        .collect()
                };

                Ok(ids
                    .into_iter()
                    .filter_map(|id| {
                        let entry = self.records.get(&id)?;
                        if entry.is_deleted {
                            None
                        } else {
                            Some(&entry.record)
                        }
                    })
                    .collect())
            }
        }
    });
    let try_find_by_prefix_arms = metas.iter().map(|m| {
        let variant = &m.variant;
        let size = m.size;
        quote! {
            #field_enum_name::#variant => {
                if raw_value.len() > #size {
                    return Err(::fixed_record_main::error::Error::FieldOverflow {
                        field: #struct_name::name_of(field),
                        size: #size,
                        actual: raw_value.len(),
                    });
                }

                let Some(tree) = self.indices.get(&field) else {
                    return Ok(Vec::new());
                };
                let Some(map) = tree.downcast_ref::<
                    std::collections::BTreeMap<
                        ::fixed_record_main::Fixed<#size>,
                        std::collections::BTreeSet<usize>
                    >
                >() else {
                    return Ok(Vec::new());
                };

                let ids: Vec<usize> = if raw_value.len() == #size {
                    let value = ::fixed_record_main::Fixed::<#size>::from_slice(raw_value)?;
                    map.get(&value)
                        .into_iter()
                        .flat_map(|ids| ids.iter().copied())
                        .collect()
                } else {
                    map.iter()
                        .filter(|(key, _)| key.as_bytes().starts_with(raw_value))
                        .flat_map(|(_, ids)| ids.iter().copied())
                        .collect()
                };

                Ok(ids
                    .into_iter()
                    .filter_map(|id| {
                        let entry = self.records.get(&id)?;
                        if entry.is_deleted {
                            None
                        } else {
                            Some(&entry.record)
                        }
                    })
                    .collect())
            }
        }
    });
    let try_first_sorted_by_arms = metas.iter().map(|m| {
        let variant = &m.variant;
        let size = m.size;
        quote! {
            #field_enum_name::#variant => {
                let tree = self.indices.get(&field)?;
                let map = tree.downcast_ref::<
                    std::collections::BTreeMap<
                        ::fixed_record_main::Fixed<#size>,
                        std::collections::BTreeSet<usize>
                    >
                >()?;

                map.values().flat_map(|ids| ids.iter()).find_map(|id| {
                    let entry = self.records.get(id)?;
                    if entry.is_deleted {
                        None
                    } else {
                        Some(&entry.record)
                    }
                })
            }
        }
    });
    let try_first_by_arms = metas.iter().map(|m| {
        let variant = &m.variant;
        let size = m.size;
        quote! {
            #field_enum_name::#variant => {
                if raw_value.len() > #size {
                    return Err(::fixed_record_main::error::Error::FieldOverflow {
                        field: #struct_name::name_of(field),
                        size: #size,
                        actual: raw_value.len(),
                    });
                }

                let Some(tree) = self.indices.get(&field) else {
                    return Ok(None);
                };
                let Some(map) = tree.downcast_ref::<
                    std::collections::BTreeMap<
                        ::fixed_record_main::Fixed<#size>,
                        std::collections::BTreeSet<usize>
                    >
                >() else {
                    return Ok(None);
                };

                let record = if raw_value.len() == #size {
                    let value = ::fixed_record_main::Fixed::<#size>::from_slice(raw_value)?;
                    map.get(&value)
                        .into_iter()
                        .flat_map(|ids| ids.iter())
                        .find_map(|id| {
                            let entry = self.records.get(id)?;
                            if entry.is_deleted {
                                None
                            } else {
                                Some(&entry.record)
                            }
                        })
                } else {
                    map.iter()
                        .filter(|(key, _)| {
                            let bytes = key.as_bytes();
                            bytes.starts_with(raw_value)
                                && bytes[raw_value.len()..]
                                    .iter()
                                    .all(|byte| *byte == 0x00 || *byte == b' ')
                        })
                        .flat_map(|(_, ids)| ids.iter())
                        .find_map(|id| {
                            let entry = self.records.get(id)?;
                            if entry.is_deleted {
                                None
                            } else {
                                Some(&entry.record)
                            }
                        })
                };

                Ok(record)
            }
        }
    });
    let try_first_by_prefix_arms = metas.iter().map(|m| {
        let variant = &m.variant;
        let size = m.size;
        quote! {
            #field_enum_name::#variant => {
                if raw_value.len() > #size {
                    return Err(::fixed_record_main::error::Error::FieldOverflow {
                        field: #struct_name::name_of(field),
                        size: #size,
                        actual: raw_value.len(),
                    });
                }

                let Some(tree) = self.indices.get(&field) else {
                    return Ok(None);
                };
                let Some(map) = tree.downcast_ref::<
                    std::collections::BTreeMap<
                        ::fixed_record_main::Fixed<#size>,
                        std::collections::BTreeSet<usize>
                    >
                >() else {
                    return Ok(None);
                };

                let record = if raw_value.len() == #size {
                    let value = ::fixed_record_main::Fixed::<#size>::from_slice(raw_value)?;
                    map.get(&value)
                        .into_iter()
                        .flat_map(|ids| ids.iter())
                        .find_map(|id| {
                            let entry = self.records.get(id)?;
                            if entry.is_deleted {
                                None
                            } else {
                                Some(&entry.record)
                            }
                        })
                } else {
                    map.iter()
                        .filter(|(key, _)| key.as_bytes().starts_with(raw_value))
                        .flat_map(|(_, ids)| ids.iter())
                        .find_map(|id| {
                            let entry = self.records.get(id)?;
                            if entry.is_deleted {
                                None
                            } else {
                                Some(&entry.record)
                            }
                        })
                };

                Ok(record)
            }
        }
    });

    // ゲッター群
    // ゲッター & セッター (ビルダー用) 群
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
            #[doc = "フィールドをバイト列として返します。"]
            pub fn #name(&self) -> &[u8] {
                self.#name.as_bytes()
            }

            #( #docs )*
            #[doc = "フィールドを UTF-8 文字列として参照します。"]
            pub fn #name_str(&self) -> Result<&str, ::fixed_record_main::error::Error> {
                self.#name.as_str()
            }

            #( #docs )*
            #[doc = "フィールドに文字列を先頭から上書きします。"]
            #[doc = "書き込み前のクリアは行わないため、短い文字列では後続バイトが残ります。"]
            pub fn #with_name(mut self, s: &str) -> Self {
                self.#name.write_bytes(s.as_bytes());
                self
            }

            #( #docs )*
            #[doc = "フィールドに数値をゼロ埋め文字列としてセットします。"]
            #[doc = "値がフィールド幅を超える場合は切り捨て、stderr に警告を出します。"]
            pub fn #with_name_int(self, val: i64) -> Self {
                self.#with_name_int_truncated(val)
            }

            #( #docs )*
            #[doc = "フィールドに数値をゼロ埋め文字列としてセットします。"]
            #[doc = "値がフィールド幅を超える場合は `Error::FieldOverflow` を返します。"]
            pub fn #try_with_name_int(self, val: i64) -> Result<Self, ::fixed_record_main::error::Error> {
                let s = format!("{:0>width$}", val, width = #size);
                let actual = s.as_bytes().len();
                if actual > #size {
                    return Err(::fixed_record_main::error::Error::FieldOverflow {
                        field: stringify!(#name),
                        size: #size,
                        actual,
                    });
                }
                Ok(self.#with_name(&s))
            }

            #( #docs )*
            #[doc = "フィールドに数値をゼロ埋め文字列としてセットし、幅を超えた場合は先頭側だけ残します。"]
            #[doc = "切り捨てが発生した場合は stderr に警告を出します。"]
            pub fn #with_name_int_truncated(self, val: i64) -> Self {
                let s = format!("{:0>width$}", val, width = #size);
                let actual = s.as_bytes().len();
                if actual > #size {
                    eprintln!(
                        "fixed_record_main: truncating field `{}` from {} bytes to {} bytes",
                        stringify!(#name),
                        actual,
                        #size
                    );
                }
                self.#with_name(&s)
            }

            #( #docs )*
            #[doc = "フィールドに符号付き数値を + または - を先頭にしてゼロ埋めでセットします。"]
            #[doc = "値がフィールド幅を超える場合は切り捨て、stderr に警告を出します。"]
            pub fn #with_name_signed(self, val: i64) -> Self {
                self.#with_name_signed_truncated(val)
            }

            #( #docs )*
            #[doc = "フィールドに符号付き数値を + または - を先頭にしてゼロ埋めでセットします。"]
            #[doc = "値がフィールド幅を超える場合は `Error::FieldOverflow` を返します。"]
            pub fn #try_with_name_signed(self, val: i64) -> Result<Self, ::fixed_record_main::error::Error> {
                let sign = if val < 0 { '-' } else { '+' };
                let abs = val.unsigned_abs();

                // 残りの幅 = 全体サイズ - 1（符号分）
                let rest = #size - 1;

                let s = format!("{}{:0>width$}", sign, abs, width = rest);
                let actual = s.as_bytes().len();
                if actual > #size {
                    return Err(::fixed_record_main::error::Error::FieldOverflow {
                        field: stringify!(#name),
                        size: #size,
                        actual,
                    });
                }
                Ok(self.#with_name(&s))
            }

            #( #docs )*
            #[doc = "フィールドに符号付き数値をセットし、幅を超えた場合は先頭側だけ残します。"]
            #[doc = "切り捨てが発生した場合は stderr に警告を出します。"]
            pub fn #with_name_signed_truncated(self, val: i64) -> Self {
                let sign = if val < 0 { '-' } else { '+' };
                let abs = val.unsigned_abs();

                // 残りの幅 = 全体サイズ - 1（符号分）
                let rest = #size - 1;

                let s = format!("{}{:0>width$}", sign, abs, width = rest);
                let actual = s.as_bytes().len();
                if actual > #size {
                    eprintln!(
                        "fixed_record_main: truncating field `{}` from {} bytes to {} bytes",
                        stringify!(#name),
                        actual,
                        #size
                    );
                }
                self.#with_name(&s)
            }
        }
    });
    // 各定数とマッチアーム
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

    Ok(quote! {
        impl #struct_name {
            #[doc = "レコード全体の合計バイト長を返します。"]
            pub const TOTAL_LEN: usize = #total_len;

            #[doc = "set_field_* が書き込み前にフィールドをクリアするときのバイト値です。"]
            pub const CLEAR_BYTE: u8 = #clear_byte;

            #(
                #[doc = "フィールドのバイト長定数です。"]
                #size_consts
            )*

            #( #field_methods )*

            #[doc = "指定したフィールドのバイト長を返します。"]
            pub const fn size_of(field: #field_enum_name) -> usize {
                match field { #( #size_arms ),* }
            }

            #[doc = "レコードの先頭から指定したフィールドまでのバイトオフセットを返します。"]
            pub const fn offset_of(field: #field_enum_name) -> usize {
                match field { #( #offset_arms ),* }
            }

            #[doc = "指定したフィールドの定義名を文字列として返します。"]
            pub const fn name_of(field: #field_enum_name) -> &'static str {
                match field { #( #name_arms ),* }
            }

            #[doc = "この構造体に定義されているすべてのフィールドのリストを返します。"]
            pub const fn all_fields() -> &'static [#field_enum_name] {
                &[ #( #field_enum_name::#all_variants ),* ]
            }
            #[doc = "全フィールドを 0x00 で埋めた新しいインスタンスを生成します。"]
            pub const fn zeroed() -> Self {
                Self {
                    #( #field_names: ::fixed_record_main::types::Fixed::zeroed() ),*
                }
            }

            #[doc = "全フィールドをスペース (0x20) で埋めた新しいインスタンスを生成します。"]
            pub const fn spaced() -> Self {
                Self {
                    #( #field_names: ::fixed_record_main::types::Fixed::spaced() ),*
                }
            }

            #[doc = "全フィールドを `CLEAR_BYTE` で埋めた新しいインスタンスを生成します。"]
            pub const fn cleared() -> Self {
                Self {
                    #( #field_names: ::fixed_record_main::types::Fixed::filled(Self::CLEAR_BYTE) ),*
                }
            }

            #[doc = "インスタンスを固定長バイト配列としてコピーして返します。"]
            pub fn to_bytes(&self) -> [u8; Self::TOTAL_LEN] {
                let mut out = [0u8; Self::TOTAL_LEN];
                #( #to_bytes_blocks )*
                out
            }

            #[doc = "インスタンスの合計バイト長を返します。"]
            pub const fn byte_len(&self) -> usize {
                Self::TOTAL_LEN
            }

            #[doc = "指定されたフィールドリストの合計バイト長を返します。"]
            pub fn byte_len_fields(fields: &[#field_enum_name]) -> usize {
                fields.iter().map(|field| Self::size_of(*field)).sum()
            }

            #[doc = "`CLEAR_BYTE` で初期化した新しいビルダーインスタンスを生成します。"]
            pub fn builder() -> Self {
                Self::cleared()
            }

            #[doc = "ビルドを完了し、インスタンスを返します（現在は self をそのまま返します）。"]
            pub fn build(self) -> Self {
                self
            }

            #[doc = "バイト列を読み取って、構造体の新しいインスタンス（所有権あり）を作成します。"]
            pub fn parse(src: &[u8]) -> Result<Self, ::fixed_record_main::error::Error> {
                if src.len() < Self::TOTAL_LEN {
                    return Err(::fixed_record_main::error::Error::TooShort);
                }
                Ok(Self {
                    #( #parse_field_inits ),*
                })
            }

            #[doc = "文字列から構造体へ変換します。内部的にはバイト列として読み取るため、コピーが発生します。"]
            pub fn parse_str(src: &str) -> Result<Self, ::fixed_record_main::error::Error> {
                Self::parse(src.as_bytes())
            }

            #unchecked_methods

            // --- 3. フィールド操作（動的アクセス） ---

            #[doc = "指定フィールドの生バイト列を返します。"]
            pub fn get_field_bytes(&self, field: #field_enum_name) -> &[u8] {
                match field {
                    #( #get_field_bytes_arms ),*
                }
            }

            #[doc = "指定フィールドを文字列として取得します（UTF-8チェック）。"]
            pub fn get_field_str(&self, field: #field_enum_name) -> Result<&str, ::fixed_record_main::error::Error> {
                std::str::from_utf8(self.get_field_bytes(field))
                    .map_err(|_| ::fixed_record_main::error::Error::Utf8Error)
            }

            #[doc = "フィールドから前後の空白やヌル文字を取り除いた文字列スライスを取得します。"]
            pub fn get_field_trimmed(&self, field: #field_enum_name) -> Result<&str, ::fixed_record_main::error::Error> {
                Ok(self.get_field_str(field)?.trim_matches(|c: char| c == ' ' || c == '\0'))
            }

            #[doc = "指定フィールドをトリミング済みの String として取得します。"]
            pub fn get_field_string_trimmed(&self, field: #field_enum_name) -> Result<String, ::fixed_record_main::error::Error> {
                self.get_field_trimmed(field).map(|s| s.to_string())
            }

            #[doc = "フィールドをトリミングした後、任意の型 T にパースして取得します。"]
            pub fn get_field_as<T: std::str::FromStr>(&self, field: #field_enum_name) -> Result<T, ::fixed_record_main::error::Error> {
                self.get_field_trimmed(field)?
                    .parse::<T>()
                    .map_err(|_| ::fixed_record_main::error::Error::ParseError)
            }

            #[doc = "指定したフィールドを 0x00 (Null) で埋めます。"]
            pub fn fill_field_zero(&mut self, field: #field_enum_name) {
                match field {
                    #(
                        #field_enum_name::#metas_variants => {
                            self.#field_names.fill_zero();
                        }
                    ),*
                }
            }

            #[doc = "指定したフィールドを 0x20 (半角スペース) で埋めます。"]
            pub fn fill_field_space(&mut self, field: #field_enum_name) {
                match field {
                    #(
                        #field_enum_name::#metas_variants => {
                            self.#field_names.fill_space();
                        }
                    ),*
                }
            }

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

            #[doc = "特定フィールドにバイト列を先頭から上書きします。"]
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

            #[doc = "特定フィールドに文字列を先頭から上書きします。"]
            #[doc = "書き込み前のクリアは行わないため、短い文字列では後続バイトが残ります。"]
            pub fn set_field_str_no_clear(&mut self, field: #field_enum_name, s: &str) {
                self.set_field_bytes_no_clear(field, s.as_bytes());
            }

           #[doc = "すべてのフィールドを 0x00 で一括上書きします。"]
            pub fn fill_zero(&mut self) {
                #( self.#field_names.fill_zero(); )*
            }

            #[doc = "すべてのフィールドを半角スペース (0x20) で一括上書きします。"]
            pub fn fill_space(&mut self) {
                #( self.#field_names.fill_space(); )*
            }

            // --- 4. 一括適用・流し込み ---

            #[doc = "自身をクロージャに渡して加工する汎用メソッド（メソッドチェーン用）。"]
            pub fn apply<F>(mut self, f: F) -> Self
            where F: FnOnce(&mut Self) {
                f(&mut self);
                self
            }

            #[doc = "先頭フィールドから順に、渡されたバイト列を各フィールドの長さ分ずつ流し込みます。"]
            pub fn apply_bytes(self, data: &[u8]) -> Self {
                self.apply_bytes_from(Self::all_fields()[0], data)
            }

            #[doc = "先頭フィールドから順に、渡された文字列を流し込みます。"]
            pub fn apply_str(self, s: &str) -> Self {
                self.apply_bytes(s.as_bytes())
            }

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

             #[doc = "開始フィールドを指定して、そこから順次strデータを流し込みます。"]
            pub fn apply_str_from(self, start_field: #field_enum_name, s: &str) -> Self {
                self.apply_bytes_from(start_field, s.as_bytes())
            }

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

            #[doc = "レコードの全フィールドを標準出力へダンプします。"]
            pub fn dump(&self) {
                println!("{}", self.to_dump_string());
            }

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
            #[doc = "フィールド名を文字列として返します。"]
            pub const fn as_str(&self) -> &'static str {
                match self {
                    #( Self::#metas_variants => #struct_name::name_of(*self), )*
                }
            }

            #[doc = "このフィールドの定義サイズを返します。"]
            pub const fn size(&self) -> usize {
                match self {
                    #( Self::#metas_variants => #struct_name::size_of(*self), )*
                }
            }
        }

        impl ::fixed_record_main::FixedRecord for #struct_name {
            type Field = #field_enum_name;

            const TOTAL_LEN: usize = #struct_name::TOTAL_LEN;

            #[doc = "固定長バイト列からレコードを作成します。"]
            fn parse(src: &[u8]) -> Result<Self, ::fixed_record_main::Error> {
                #struct_name::parse(src)
            }

            #[doc = "レコードを固定長バイト列としてコピーして返します。"]
            fn to_bytes(&self) -> Vec<u8> {
                #struct_name::to_bytes(self).to_vec()
            }

            #[doc = "指定フィールドの定義名を返します。"]
            fn field_name(field: Self::Field) -> &'static str {
                #struct_name::name_of(field)
            }

            #[doc = "指定フィールドのバイト列を返します。"]
            fn field_bytes(&self, field: Self::Field) -> &[u8] {
                self.get_field_bytes(field)
            }
        }

        impl Default for #struct_name {
            #[doc = "cleared() を呼び出して `CLEAR_BYTE` で初期化します。"]
            fn default() -> Self {
                Self::cleared()
            }
        }

        struct #entry_name {
            record: #struct_name,
            is_deleted: bool,
        }

        #[doc = "レコードのコレクションを保持し、検索・削除・ソート用インデックスを管理します。"]
        pub struct #list_name {
            records: std::collections::BTreeMap<usize, #entry_name>,
            next_id: usize,
            indices: std::collections::HashMap<#field_enum_name, Box<dyn std::any::Any>>,
            order: Vec<usize>,
        }

        impl #list_name {
            #[doc = "空のリストを作成します。"]
            pub fn new() -> Self {
                Self {
                    records: std::collections::BTreeMap::new(),
                    next_id: 0,
                    indices: std::collections::HashMap::new(),
                    order: Vec::new(),
                }
            }

            #[doc = "有効なレコード数を返します。"]
            pub fn len(&self) -> usize {
                self.records.values().filter(|entry| !entry.is_deleted).count()
            }

            #[doc = "有効なレコードがないかを返します。"]
            pub fn is_empty(&self) -> bool {
                self.len() == 0
            }

            #[doc = "有効なレコードのみを現在の順序で返すイテレータです。"]
            pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a #struct_name> + 'a {
                self.order.iter().filter_map(move |id| {
                    let entry = self.records.get(id)?;
                    if entry.is_deleted {
                        None
                    } else {
                        Some(&entry.record)
                    }
                })
            }

            #[doc = "レコードを追加し、採番された ID を返します。"]
            pub fn insert(&mut self, record: #struct_name) -> usize {
                let id = self.next_id;
                self.next_id += 1;

                #( #index_insert_blocks )*

                self.records.insert(id, #entry_name { record, is_deleted: false });
                self.order.push(id);
                id
            }

            #[doc = "指定 ID の有効なレコードを返します。"]
            pub fn get(&self, id: usize) -> Option<&#struct_name> {
                let entry = self.records.get(&id)?;
                if entry.is_deleted {
                    None
                } else {
                    Some(&entry.record)
                }
            }

            #[doc = "指定 ID の有効なレコードを置き換え、検索インデックスを更新します。"]
            pub fn update(&mut self, id: usize, record: #struct_name) -> bool {
                let Some(entry) = self.records.remove(&id) else {
                    return false;
                };

                if entry.is_deleted {
                    self.records.insert(id, entry);
                    return false;
                }

                let old_record = entry.record;
                #( #index_update_remove_blocks )*
                #( #index_update_insert_blocks )*

                self.records.insert(id, #entry_name { record, is_deleted: false });
                true
            }

            #[doc = "指定 ID のレコードを論理削除し、検索インデックスからも除外します。"]
            pub fn remove(&mut self, id: usize) -> bool {
                let record = {
                    let Some(entry) = self.records.get_mut(&id) else {
                        return false;
                    };
                    if entry.is_deleted {
                        return false;
                    }

                    entry.is_deleted = true;
                    entry.record
                };

                #( #index_remove_blocks )*
                true
            }

            #[doc = "指定フィールドが値と完全一致する有効なレコードを返します。"]
            pub fn find_by<const N: usize>(
                &self,
                field: #field_enum_name,
                value: impl Into<::fixed_record_main::Fixed<N>>,
            ) -> Vec<&#struct_name> {
                let value = value.into();
                self.indices.get(&field)
                    .and_then(|tree| {
                        tree.downcast_ref::<
                            std::collections::BTreeMap<
                                ::fixed_record_main::Fixed<N>,
                                std::collections::BTreeSet<usize>
                            >
                        >()
                    })
                    .and_then(|map| map.get(&value))
                    .map(|ids| {
                        ids.iter()
                            .filter_map(|id| {
                                let entry = self.records.get(id)?;
                                if entry.is_deleted {
                                    None
                                } else {
                                    Some(&entry.record)
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            }

            #[doc = "指定フィールドが値と一致する有効なレコードを返します。"]
            #[doc = "検索値がフィールド幅より短い場合は、後続バイトが 0x00 または半角スペースのレコードも一致します。"]
            #[doc = "検索値がフィールド幅を超える場合は `Error::FieldOverflow` を返します。"]
            pub fn try_find_by(
                &self,
                field: #field_enum_name,
                value: impl AsRef<[u8]>,
            ) -> Result<Vec<&#struct_name>, ::fixed_record_main::error::Error> {
                let raw_value = value.as_ref();
                match field {
                    #( #try_find_by_arms ),*
                }
            }

            #[doc = "指定フィールドが値で始まる有効なレコードを返します。"]
            #[doc = "検索値がフィールド幅より短い場合は、後続バイトの内容に関係なく一致します。"]
            #[doc = "検索値がフィールド幅を超える場合は `Error::FieldOverflow` を返します。"]
            pub fn try_find_by_prefix(
                &self,
                field: #field_enum_name,
                value: impl AsRef<[u8]>,
            ) -> Result<Vec<&#struct_name>, ::fixed_record_main::error::Error> {
                let raw_value = value.as_ref();
                match field {
                    #( #try_find_by_prefix_arms ),*
                }
            }

            #[doc = "指定フィールドの値が範囲内にある有効なレコードを返します。"]
            pub fn find_range_by<const N: usize, R>(
                &self,
                field: #field_enum_name,
                range: R,
            ) -> Vec<&#struct_name>
            where
                R: std::ops::RangeBounds<::fixed_record_main::Fixed<N>>,
            {
                self.indices.get(&field)
                    .and_then(|tree| {
                        tree.downcast_ref::<
                            std::collections::BTreeMap<
                                ::fixed_record_main::Fixed<N>,
                                std::collections::BTreeSet<usize>
                            >
                        >()
                    })
                    .map(|map| {
                        map.range(range)
                            .flat_map(|(_, ids)| ids.iter())
                            .filter_map(|id| {
                                let entry = self.records.get(id)?;
                                if entry.is_deleted {
                                    None
                                } else {
                                    Some(&entry.record)
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            }

            #[doc = "指定フィールドで昇順に並ぶ有効レコードのイテレータを返します。"]
            pub fn iter_sorted_by<'a, const N: usize>(
                &'a self,
                field: #field_enum_name,
            ) -> impl Iterator<Item = &'a #struct_name> + 'a {
                self.indices.get(&field)
                    .and_then(|tree| {
                        tree.downcast_ref::<
                            std::collections::BTreeMap<
                                ::fixed_record_main::Fixed<N>,
                                std::collections::BTreeSet<usize>
                            >
                        >()
                    })
                    .into_iter()
                    .flat_map(move |map| {
                        map.values().flat_map(move |ids| {
                            ids.iter().filter_map(move |id| {
                                let entry = self.records.get(id)?;
                                if entry.is_deleted {
                                    None
                                } else {
                                    Some(&entry.record)
                                }
                            })
                        })
                    })
            }

            #[doc = "現在の有効レコードを定義順の全フィールド比較でソートします。"]
            pub fn sort(&mut self) {
                let records = &self.records;
                self.order
                    .retain(|id| records.get(id).is_some_and(|entry| !entry.is_deleted));
                self.order.sort_by(|left, right| {
                    let left = &records.get(left).unwrap().record;
                    let right = &records.get(right).unwrap().record;
                    left.compare_all_fields(right)
                });
            }

            #[doc = "指定したフィールド優先順で現在の有効レコードをソートします。"]
            pub fn sort_by(&mut self, fields: &[#field_enum_name]) {
                let records = &self.records;
                self.order
                    .retain(|id| records.get(id).is_some_and(|entry| !entry.is_deleted));
                self.order.sort_by(|left, right| {
                    let left = &records.get(left).unwrap().record;
                    let right = &records.get(right).unwrap().record;
                    left.compare_by_fields(right, fields)
                });
            }

            #[doc = "有効なレコードのうち、現在のID順で最初のものを返します。"]
            pub fn first(&self) -> Option<&#struct_name> {
                self.records
                    .values()
                    .find(|entry| !entry.is_deleted)
                    .map(|entry| &entry.record)
            }

            #[doc = "指定フィールドの昇順で最初の有効レコードを返します。"]
            pub fn first_by<const N: usize>(&self, field: #field_enum_name) -> Option<&#struct_name> {
                self.iter_sorted_by::<N>(field).next()
            }

            #[doc = "指定フィールドの昇順で最初の有効レコードを返します。"]
            #[doc = "`first_by` と違い、呼び出し側でフィールド幅を指定する必要はありません。"]
            pub fn try_first_sorted_by(&self, field: #field_enum_name) -> Option<&#struct_name> {
                match field {
                    #( #try_first_sorted_by_arms ),*
                }
            }

            #[doc = "指定フィールドが値と一致する有効レコードのうち、指定フィールドの昇順で最初のものを返します。"]
            #[doc = "検索値がフィールド幅より短い場合は、後続バイトが 0x00 または半角スペースのレコードも一致します。"]
            #[doc = "検索値がフィールド幅を超える場合は `Error::FieldOverflow` を返します。"]
            pub fn try_first_by(
                &self,
                field: #field_enum_name,
                value: impl AsRef<[u8]>,
            ) -> Result<Option<&#struct_name>, ::fixed_record_main::error::Error> {
                let raw_value = value.as_ref();
                match field {
                    #( #try_first_by_arms ),*
                }
            }

            #[doc = "指定フィールドが値で始まる有効レコードのうち、指定フィールドの昇順で最初のものを返します。"]
            #[doc = "検索値がフィールド幅を超える場合は `Error::FieldOverflow` を返します。"]
            pub fn try_first_by_prefix(
                &self,
                field: #field_enum_name,
                value: impl AsRef<[u8]>,
            ) -> Result<Option<&#struct_name>, ::fixed_record_main::error::Error> {
                let raw_value = value.as_ref();
                match field {
                    #( #try_first_by_prefix_arms ),*
                }
            }

            #[doc = "物理的に保持している全 ID を返します。"]
            pub fn all_ids(&self) -> Vec<usize> {
                self.records.keys().copied().collect()
            }

            #[doc = "論理削除済みレコードを物理削除し、インデックスも掃除します。"]
            pub fn vacuum(&mut self) {
                let ids: Vec<usize> = self.records
                    .iter()
                    .filter(|(_, entry)| entry.is_deleted)
                    .map(|(id, _)| *id)
                    .collect();

                for id in ids {
                    if let Some(entry) = self.records.remove(&id) {
                        let record = entry.record;
                        #( #index_vacuum_blocks )*
                    }
                }

                let records = &self.records;
                self.order.retain(|id| records.contains_key(id));
            }
        }

        impl Default for #list_name {
            #[doc = "空のリストを作成します。"]
            fn default() -> Self {
                Self::new()
            }
        }
    })
}
