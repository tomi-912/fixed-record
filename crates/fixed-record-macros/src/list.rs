use crate::core::FieldMeta;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::DeriveInput;

/// Generates the optional `{StructName}List` helper.
/// optional な `{StructName}List` 補助型を生成します。
pub(super) fn gen_list_impl(input: &DeriveInput, metas: &[FieldMeta<'_>]) -> TokenStream {
    if !cfg!(feature = "list") {
        return quote!();
    }

    let struct_name = &input.ident;
    let struct_vis = &input.vis;
    let field_enum_name = format_ident!("{}Field", struct_name);
    let entry_name = format_ident!("{}Entry", struct_name);
    let list_name = format_ident!("{}List", struct_name);

    let try_find_by_arms = metas.iter().map(|m| {
        let variant = &m.variant;
        let size = m.size;
        quote! {
            #field_enum_name::#variant => {
                if raw_value.len() > #size {
                    return Err(::fixed_record::error::Error::FieldOverflow {
                        field: #struct_name::name_of(field),
                        size: #size,
                        actual: raw_value.len(),
                    });
                }

                Ok(self.records
                    .iter()
                    .filter_map(|entry| {
                        let bytes = entry.record.field_bytes(field);
                        let is_match = if raw_value.len() == #size {
                            bytes == raw_value
                        } else {
                            bytes.starts_with(raw_value)
                                && bytes[raw_value.len()..]
                                    .iter()
                                    .all(|byte| *byte == 0x00 || *byte == b' ')
                        };

                        if is_match {
                            Some(&entry.record)
                        } else {
                            None
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
                    return Err(::fixed_record::error::Error::FieldOverflow {
                        field: #struct_name::name_of(field),
                        size: #size,
                        actual: raw_value.len(),
                    });
                }

                Ok(self.records
                    .iter()
                    .filter_map(|entry| {
                        if entry.record.field_bytes(field).starts_with(raw_value) {
                            Some(&entry.record)
                        } else {
                            None
                        }
                    })
                    .collect())
            }
        }
    });
    let try_first_sorted_by_arms = metas.iter().map(|m| {
        let variant = &m.variant;
        quote! {
            #field_enum_name::#variant => {
                self.records
                    .iter()
                    .min_by(|left, right| {
                        left.record
                            .field_bytes(field)
                            .cmp(right.record.field_bytes(field))
                    })
                    .map(|entry| &entry.record)
            }
        }
    });
    let try_first_by_arms = metas.iter().map(|m| {
        let variant = &m.variant;
        let size = m.size;
        quote! {
            #field_enum_name::#variant => {
                if raw_value.len() > #size {
                    return Err(::fixed_record::error::Error::FieldOverflow {
                        field: #struct_name::name_of(field),
                        size: #size,
                        actual: raw_value.len(),
                    });
                }

                Ok(self.records
                    .iter()
                    .filter(|entry| {
                        let bytes = entry.record.field_bytes(field);
                        if raw_value.len() == #size {
                            bytes == raw_value
                        } else {
                            bytes.starts_with(raw_value)
                                && bytes[raw_value.len()..]
                                    .iter()
                                    .all(|byte| *byte == 0x00 || *byte == b' ')
                        }
                    })
                    .min_by(|left, right| {
                        left.record
                            .field_bytes(field)
                            .cmp(right.record.field_bytes(field))
                    })
                    .map(|entry| &entry.record))
            }
        }
    });
    let try_first_by_prefix_arms = metas.iter().map(|m| {
        let variant = &m.variant;
        let size = m.size;
        quote! {
            #field_enum_name::#variant => {
                if raw_value.len() > #size {
                    return Err(::fixed_record::error::Error::FieldOverflow {
                        field: #struct_name::name_of(field),
                        size: #size,
                        actual: raw_value.len(),
                    });
                }

                Ok(self.records
                    .iter()
                    .filter(|entry| entry.record.field_bytes(field).starts_with(raw_value))
                    .min_by(|left, right| {
                        left.record
                            .field_bytes(field)
                            .cmp(right.record.field_bytes(field))
                    })
                    .map(|entry| &entry.record))
            }
        }
    });

    quote! {
        struct #entry_name {
            id: usize,
            record: #struct_name,
        }

        #[doc = "Stores records in a vector and provides collection helpers for search, removal, and sorting."]
        #[doc = "レコードを vector に保持し、検索・削除・ソート用の補助 API を提供します。"]
        #struct_vis struct #list_name {
            records: Vec<Box<#entry_name>>,
            next_id: usize,
        }

        impl #list_name {
            #[doc = "Creates an empty list."]
            #[doc = "空のリストを作成します。"]
            pub fn new() -> Self {
                Self {
                    records: Vec::new(),
                    next_id: 0,
                }
            }

            #[doc = "Returns the number of records."]
            #[doc = "レコード数を返します。"]
            pub fn len(&self) -> usize {
                self.records.len()
            }

            #[doc = "Returns whether there are no records."]
            #[doc = "レコードがないかを返します。"]
            pub fn is_empty(&self) -> bool {
                self.records.is_empty()
            }

            #[doc = "Returns an iterator over records in the current order."]
            #[doc = "現在の順序でレコードを返すイテレータです。"]
            pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a #struct_name> + 'a {
                self.records.iter().map(|entry| &entry.record)
            }

            #[doc = "Inserts a record and returns its assigned ID."]
            #[doc = "レコードを追加し、採番された ID を返します。"]
            pub fn insert(&mut self, record: #struct_name) -> usize {
                let id = self.next_id;
                self.next_id += 1;
                self.records.push(Box::new(#entry_name { id, record }));
                id
            }

            #[doc = "Returns the record with the specified ID."]
            #[doc = "指定 ID のレコードを返します。"]
            pub fn get(&self, id: usize) -> Option<&#struct_name> {
                self.records
                    .iter()
                    .find(|entry| entry.id == id)
                    .map(|entry| &entry.record)
            }

            #[doc = "Replaces the record with the specified ID."]
            #[doc = "指定 ID のレコードを置き換えます。"]
            pub fn update(&mut self, id: usize, record: #struct_name) -> bool {
                let Some(entry) = self.records.iter_mut().find(|entry| entry.id == id) else {
                    return false;
                };

                entry.record = record;
                true
            }

            #[doc = "Removes the record with the specified ID."]
            #[doc = "指定 ID のレコードを削除します。"]
            pub fn remove(&mut self, id: usize) -> bool {
                let Some(position) = self.records.iter().position(|entry| entry.id == id) else {
                    return false;
                };

                self.records.remove(position);
                true
            }

            #[doc = "Returns records whose specified field exactly matches the value."]
            #[doc = "指定フィールドが値と完全一致するレコードを返します。"]
            pub fn find_by<const N: usize>(
                &self,
                field: #field_enum_name,
                value: impl Into<::fixed_record::Fixed<N>>,
            ) -> Vec<&#struct_name> {
                let value = value.into();
                self.records
                    .iter()
                    .filter_map(|entry| {
                        if entry.record.field_bytes(field) == value.as_bytes() {
                            Some(&entry.record)
                        } else {
                            None
                        }
                    })
                    .collect()
            }

            #[doc = "Returns records whose specified field matches the value."]
            #[doc = "指定フィールドが値と一致するレコードを返します。"]
            #[doc = "When the search value is shorter than the field width, trailing `0x00` or space bytes are accepted."]
            #[doc = "検索値がフィールド幅より短い場合は、後続バイトが `0x00` または半角スペースのレコードも一致します。"]
            #[doc = "Returns `Error::FieldOverflow` when the search value is wider than the field."]
            #[doc = "検索値がフィールド幅を超える場合は `Error::FieldOverflow` を返します。"]
            pub fn try_find_by(
                &self,
                field: #field_enum_name,
                value: impl AsRef<[u8]>,
            ) -> Result<Vec<&#struct_name>, ::fixed_record::error::Error> {
                let raw_value = value.as_ref();
                match field {
                    #( #try_find_by_arms ),*
                }
            }

            #[doc = "Returns records whose specified field starts with the value."]
            #[doc = "指定フィールドが値で始まるレコードを返します。"]
            #[doc = "When the search value is shorter than the field width, trailing bytes may contain any value."]
            #[doc = "検索値がフィールド幅より短い場合は、後続バイトの内容に関係なく一致します。"]
            #[doc = "Returns `Error::FieldOverflow` when the search value is wider than the field."]
            #[doc = "検索値がフィールド幅を超える場合は `Error::FieldOverflow` を返します。"]
            pub fn try_find_by_prefix(
                &self,
                field: #field_enum_name,
                value: impl AsRef<[u8]>,
            ) -> Result<Vec<&#struct_name>, ::fixed_record::error::Error> {
                let raw_value = value.as_ref();
                match field {
                    #( #try_find_by_prefix_arms ),*
                }
            }

            #[doc = "Returns records whose specified field value is within the range."]
            #[doc = "指定フィールドの値が範囲内にあるレコードを返します。"]
            pub fn find_range_by<const N: usize, R>(
                &self,
                field: #field_enum_name,
                range: R,
            ) -> Vec<&#struct_name>
            where
                R: std::ops::RangeBounds<::fixed_record::Fixed<N>>,
            {
                self.records
                    .iter()
                    .filter_map(|entry| {
                        let value = ::fixed_record::Fixed::<N>::from_slice(entry.record.field_bytes(field)).ok()?;
                        if range.contains(&value) {
                            Some(&entry.record)
                        } else {
                            None
                        }
                    })
                    .collect()
            }

            #[doc = "Returns an iterator over records sorted by the specified field in ascending order."]
            #[doc = "指定フィールドで昇順に並ぶレコードのイテレータを返します。"]
            pub fn iter_sorted_by<'a, const N: usize>(
                &'a self,
                field: #field_enum_name,
            ) -> impl Iterator<Item = &'a #struct_name> + 'a {
                let mut entries: Vec<&#entry_name> = self.records.iter().map(|entry| entry.as_ref()).collect();
                entries.sort_by(|left, right| {
                    left.record
                        .field_bytes(field)
                        .cmp(right.record.field_bytes(field))
                });
                entries.into_iter().map(|entry| &entry.record)
            }

            #[doc = "Sorts records by comparing all fields in declaration order."]
            #[doc = "レコードを定義順の全フィールド比較でソートします。"]
            pub fn sort(&mut self) {
                self.records.sort_by(|left, right| {
                    left.record.compare_all_fields(&right.record)
                });
            }

            #[doc = "Sorts records by the specified field priority order."]
            #[doc = "指定したフィールド優先順でレコードをソートします。"]
            pub fn sort_by(&mut self, fields: &[#field_enum_name]) {
                self.records.sort_by(|left, right| {
                    left.record.compare_by_fields(&right.record, fields)
                });
            }

            #[doc = "Returns the first record in the current order."]
            #[doc = "現在の順序で最初のレコードを返します。"]
            pub fn first(&self) -> Option<&#struct_name> {
                self.records.first().map(|entry| &entry.record)
            }

            #[doc = "Returns the first record in ascending order by the specified field."]
            #[doc = "指定フィールドの昇順で最初のレコードを返します。"]
            pub fn first_by<const N: usize>(&self, field: #field_enum_name) -> Option<&#struct_name> {
                self.iter_sorted_by::<N>(field).next()
            }

            #[doc = "Returns the first record in ascending order by the specified field."]
            #[doc = "指定フィールドの昇順で最初のレコードを返します。"]
            #[doc = "Unlike `first_by`, callers do not need to specify the field width."]
            #[doc = "`first_by` と違い、呼び出し側でフィールド幅を指定する必要はありません。"]
            pub fn try_first_sorted_by(&self, field: #field_enum_name) -> Option<&#struct_name> {
                match field {
                    #( #try_first_sorted_by_arms ),*
                }
            }

            #[doc = "Returns the first matching record in ascending order by the specified field."]
            #[doc = "指定フィールドが値と一致するレコードのうち、指定フィールドの昇順で最初のものを返します。"]
            #[doc = "When the search value is shorter than the field width, trailing `0x00` or space bytes are accepted."]
            #[doc = "検索値がフィールド幅より短い場合は、後続バイトが `0x00` または半角スペースのレコードも一致します。"]
            #[doc = "Returns `Error::FieldOverflow` when the search value is wider than the field."]
            #[doc = "検索値がフィールド幅を超える場合は `Error::FieldOverflow` を返します。"]
            pub fn try_first_by(
                &self,
                field: #field_enum_name,
                value: impl AsRef<[u8]>,
            ) -> Result<Option<&#struct_name>, ::fixed_record::error::Error> {
                let raw_value = value.as_ref();
                match field {
                    #( #try_first_by_arms ),*
                }
            }

            #[doc = "Returns the first prefix match in ascending order by the specified field."]
            #[doc = "指定フィールドが値で始まるレコードのうち、指定フィールドの昇順で最初のものを返します。"]
            #[doc = "Returns `Error::FieldOverflow` when the search value is wider than the field."]
            #[doc = "検索値がフィールド幅を超える場合は `Error::FieldOverflow` を返します。"]
            pub fn try_first_by_prefix(
                &self,
                field: #field_enum_name,
                value: impl AsRef<[u8]>,
            ) -> Result<Option<&#struct_name>, ::fixed_record::error::Error> {
                let raw_value = value.as_ref();
                match field {
                    #( #try_first_by_prefix_arms ),*
                }
            }

            #[doc = "Returns all record IDs in the current order."]
            #[doc = "現在の順序で全レコード ID を返します。"]
            pub fn all_ids(&self) -> Vec<usize> {
                self.records.iter().map(|entry| entry.id).collect()
            }
        }

        impl Default for #list_name {
            #[doc = "Creates an empty list."]
            #[doc = "空のリストを作成します。"]
            fn default() -> Self {
                Self::new()
            }
        }
    }
}
