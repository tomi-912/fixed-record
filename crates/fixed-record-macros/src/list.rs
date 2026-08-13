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
    let list_name = format_ident!("{}List", struct_name);
    let indices_name = format_ident!("{}ListIndices", struct_name);

    let index_fields = metas.iter().map(|meta| {
        let name = meta.name;
        let size = meta.size;
        quote! {
            #name: std::collections::BTreeMap<::fixed_record::Fixed<#size>, Vec<usize>>
        }
    });
    let index_record_fields = metas.iter().map(|meta| {
        let name = meta.name;
        quote! {
            Self::index_value(&mut indices.#name, id, record.#name);
        }
    });
    let unindex_record_fields = metas.iter().map(|meta| {
        let name = meta.name;
        quote! {
            Self::unindex_value(&mut indices.#name, id, record.#name);
        }
    });
    let increment_index_fields = metas.iter().map(|meta| {
        let name = meta.name;
        quote! {
            Self::increment_value_ids_at_or_after(&mut self.indices.#name, index);
        }
    });
    let decrement_index_fields = metas.iter().map(|meta| {
        let name = meta.name;
        quote! {
            Self::decrement_value_ids_after(&mut self.indices.#name, index);
        }
    });
    let indexed_prefix_arms = metas.iter().map(|meta| {
        let name = meta.name;
        let variant = &meta.variant;
        quote! {
            #field_enum_name::#variant => {
                Self::indexed_prefix_ids_for(&self.indices.#name, prefix, padding_only)
            }
        }
    });
    let first_indexed_prefix_arms = metas.iter().map(|meta| {
        let name = meta.name;
        let variant = &meta.variant;
        quote! {
            #field_enum_name::#variant => {
                Self::first_indexed_prefix_id_for(&self.indices.#name, prefix, padding_only)
            }
        }
    });
    let find_by_arms = metas.iter().map(|meta| {
        let name = meta.name;
        let size = meta.size;
        let variant = &meta.variant;
        quote! {
            #field_enum_name::#variant => {
                let key = ::fixed_record::Fixed::<#size>::from_slice(raw_value)?;
                self.indices.#name.get(&key)
            }
        }
    });
    let find_range_by_arms = metas.iter().map(|meta| {
        let name = meta.name;
        let size = meta.size;
        let variant = &meta.variant;
        quote! {
            #field_enum_name::#variant => {
                Self::range_ids_for::<#size, N, R>(&self.indices.#name, &range)
            }
        }
    });
    let iter_sorted_by_arms = metas.iter().map(|meta| {
        let name = meta.name;
        let variant = &meta.variant;
        quote! {
            #field_enum_name::#variant => {
                for ids in self.indices.#name.values() {
                    records.extend(ids.iter().filter_map(|id| self.get(*id)));
                }
            }
        }
    });
    let first_sorted_by_arms = metas.iter().map(|meta| {
        let name = meta.name;
        let variant = &meta.variant;
        quote! {
            #field_enum_name::#variant => self
                .indices
                .#name
                .first_key_value()?
                .1
                .first()
                .copied()
        }
    });

    quote! {
        #[doc(hidden)]
        #[derive(Default)]
        struct #indices_name {
            #( #index_fields ),*
        }

        #[doc = "Stores boxed records in a vector and maintains field indexes for search and sorting."]
        #[doc = "Box 化したレコードを vector に保持し、検索・ソート用のフィールド索引を管理します。"]
        #[doc = "Each field index maps actual field bytes to the current vector indexes of matching records."]
        #[doc = "各フィールド索引は、実際のフィールドバイト列から、一致するレコードの現在の vector index へ対応付けます。"]
        #[doc = "Sorting moves boxes in the vector, not the record values allocated behind them."]
        #[doc = "ソート時は vector 内の Box が移動し、Box の先にあるレコード本体は移動しません。"]
        #[doc = "Record IDs are the current vector indexes, so IDs can change after removal or sorting."]
        #[doc = "レコード ID は現在の vector index です。そのため削除やソート後に ID は変わる可能性があります。"]
        #struct_vis struct #list_name {
            records: Vec<Box<#struct_name>>,
            indices: #indices_name,
        }

        impl #list_name {
            #[doc = "Creates an empty list."]
            #[doc = "空のリストを作成します。"]
            pub fn new() -> Self {
                Self {
                    records: Vec::new(),
                    indices: #indices_name::default(),
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
                self.records.iter().map(|record| record.as_ref())
            }

            #[doc = "Adds one record to every field index."]
            #[doc = "1件のレコードを全フィールド索引へ追加します。"]
            fn index_record(indices: &mut #indices_name, id: usize, record: &#struct_name) {
                #( #index_record_fields )*
            }

            #[doc = "Adds one record ID to a typed field-value index."]
            #[doc = "型付きフィールド値索引へレコード ID を1件追加します。"]
            fn index_value<const N: usize>(
                field_index: &mut std::collections::BTreeMap<
                    ::fixed_record::Fixed<N>,
                    Vec<usize>,
                >,
                id: usize,
                value: ::fixed_record::Fixed<N>,
            ) {
                let ids = field_index.entry(value).or_default();
                if let Err(position) = ids.binary_search(&id) {
                    ids.insert(position, id);
                }
            }

            #[doc = "Removes one record from every field index."]
            #[doc = "1件のレコードを全フィールド索引から削除します。"]
            fn unindex_record(indices: &mut #indices_name, id: usize, record: &#struct_name) {
                #( #unindex_record_fields )*
            }

            #[doc = "Removes one record ID from a typed field-value index."]
            #[doc = "型付きフィールド値索引からレコード ID を1件削除します。"]
            fn unindex_value<const N: usize>(
                field_index: &mut std::collections::BTreeMap<
                    ::fixed_record::Fixed<N>,
                    Vec<usize>,
                >,
                id: usize,
                value: ::fixed_record::Fixed<N>,
            ) {
                let remove_value = if let Some(ids) = field_index.get_mut(&value) {
                    if let Ok(position) = ids.binary_search(&id) {
                        ids.remove(position);
                    }
                    ids.is_empty()
                } else {
                    false
                };

                if remove_value {
                    field_index.remove(&value);
                }
            }

            #[doc = "Rebuilds all field indexes from the current vector order."]
            #[doc = "現在の vector 順序から全フィールド索引を再構築します。"]
            fn rebuild_indices(&mut self) {
                let mut indices = #indices_name::default();
                for (id, record) in self.records.iter().enumerate() {
                    Self::index_record(&mut indices, id, record.as_ref());
                }
                self.indices = indices;
            }

            #[doc = "Increments indexed record IDs at or after an insertion position."]
            #[doc = "挿入位置以降の索引内レコード ID を1つ繰り上げます。"]
            fn increment_index_ids_at_or_after(&mut self, index: usize) {
                #( #increment_index_fields )*
            }

            #[doc = "Increments record IDs in one field index at or after an insertion position."]
            #[doc = "1つのフィールド索引で挿入位置以降のレコード ID を繰り上げます。"]
            fn increment_value_ids_at_or_after<const N: usize>(
                field_index: &mut std::collections::BTreeMap<
                    ::fixed_record::Fixed<N>,
                    Vec<usize>,
                >,
                index: usize,
            ) {
                for ids in field_index.values_mut() {
                    let first_shifted = ids.partition_point(|id| *id < index);
                    for id in &mut ids[first_shifted..] {
                        *id += 1;
                    }
                }
            }

            #[doc = "Decrements indexed record IDs after a removal position."]
            #[doc = "削除位置より後ろの索引内レコード ID を1つ繰り下げます。"]
            fn decrement_index_ids_after(&mut self, index: usize) {
                #( #decrement_index_fields )*
            }

            #[doc = "Decrements record IDs in one field index after a removal position."]
            #[doc = "1つのフィールド索引で削除位置より後ろのレコード ID を繰り下げます。"]
            fn decrement_value_ids_after<const N: usize>(
                field_index: &mut std::collections::BTreeMap<
                    ::fixed_record::Fixed<N>,
                    Vec<usize>,
                >,
                index: usize,
            ) {
                for ids in field_index.values_mut() {
                    let first_shifted = ids.partition_point(|id| *id <= index);
                    for id in &mut ids[first_shifted..] {
                        *id -= 1;
                    }
                }
            }

            #[doc = "Returns the exclusive upper bound for byte keys that share a prefix."]
            #[doc = "同じ prefix を持つバイトキーを範囲検索するための排他的上限を返します。"]
            fn prefix_upper_bound(prefix: &[u8]) -> Option<Vec<u8>> {
                let mut upper = prefix.to_vec();
                for index in (0..upper.len()).rev() {
                    if upper[index] != u8::MAX {
                        upper[index] += 1;
                        upper.truncate(index + 1);
                        return Some(upper);
                    }
                }
                None
            }

            #[doc = "Returns indexed record IDs whose field keys share a prefix."]
            #[doc = "フィールドキーが prefix を共有するレコード ID を索引から返します。"]
            fn indexed_prefix_ids(
                &self,
                field: #field_enum_name,
                prefix: &[u8],
                padding_only: bool,
            ) -> Vec<usize> {
                match field {
                    #( #indexed_prefix_arms ),*
                }
            }

            #[doc = "Returns record IDs in one typed field index whose keys share a prefix."]
            #[doc = "1つの型付きフィールド索引からキーが prefix を共有するレコード ID を返します。"]
            fn indexed_prefix_ids_for<const N: usize>(
                field_index: &std::collections::BTreeMap<
                    ::fixed_record::Fixed<N>,
                    Vec<usize>,
                >,
                prefix: &[u8],
                padding_only: bool,
            ) -> Vec<usize> {
                use std::ops::Bound::{Excluded, Included, Unbounded};

                let mut lower = ::fixed_record::Fixed::<N>::zeroed();
                lower.write_bytes(prefix);
                let upper = Self::prefix_upper_bound(prefix);
                let bounds = match upper {
                    Some(upper) => {
                        let mut value = ::fixed_record::Fixed::<N>::zeroed();
                        value.write_bytes(&upper);
                        (Included(lower), Excluded(value))
                    }
                    None => (Included(lower), Unbounded),
                };
                let mut ids = Vec::new();

                for (value, indexed_ids) in field_index.range(bounds) {
                    if !padding_only
                        || value.as_bytes()[prefix.len()..]
                            .iter()
                            .all(|byte| *byte == 0x00 || *byte == b' ')
                    {
                        ids.extend_from_slice(indexed_ids);
                    }
                }

                ids
            }

            #[doc = "Returns the first indexed record ID whose field key shares a prefix."]
            #[doc = "フィールドキーが prefix を共有する最初のレコード ID を索引から返します。"]
            fn first_indexed_prefix_id(
                &self,
                field: #field_enum_name,
                prefix: &[u8],
                padding_only: bool,
            ) -> Option<usize> {
                match field {
                    #( #first_indexed_prefix_arms ),*
                }
            }

            #[doc = "Returns the first record ID in one typed field index whose key shares a prefix."]
            #[doc = "1つの型付きフィールド索引からキーが prefix を共有する最初のレコード ID を返します。"]
            fn first_indexed_prefix_id_for<const N: usize>(
                field_index: &std::collections::BTreeMap<
                    ::fixed_record::Fixed<N>,
                    Vec<usize>,
                >,
                prefix: &[u8],
                padding_only: bool,
            ) -> Option<usize> {
                use std::ops::Bound::{Excluded, Included, Unbounded};

                let mut lower = ::fixed_record::Fixed::<N>::zeroed();
                lower.write_bytes(prefix);
                let upper = Self::prefix_upper_bound(prefix);
                let bounds = match upper {
                    Some(upper) => {
                        let mut value = ::fixed_record::Fixed::<N>::zeroed();
                        value.write_bytes(&upper);
                        (Included(lower), Excluded(value))
                    }
                    None => (Included(lower), Unbounded),
                };

                for (value, indexed_ids) in field_index.range(bounds) {
                    if (!padding_only
                        || value.as_bytes()[prefix.len()..]
                            .iter()
                            .all(|byte| *byte == 0x00 || *byte == b' '))
                        && let Some(id) = indexed_ids.first()
                    {
                        return Some(*id);
                    }
                }

                None
            }

            #[doc = "Maps current record IDs to borrowed records in current list order."]
            #[doc = "現在のレコード ID を、現在の List 順序に並ぶレコード参照へ変換します。"]
            fn records_for_ids<'a>(&'a self, mut ids: Vec<usize>) -> Vec<&#struct_name> {
                ids.sort_unstable();
                ids.into_iter().filter_map(|id| self.get(id)).collect()
            }

            #[doc = "Validates a search value against the selected field width."]
            #[doc = "検索値が選択フィールドの幅に収まることを検証します。"]
            fn validate_search_width(
                field: #field_enum_name,
                value: &[u8],
            ) -> Result<(), ::fixed_record::error::Error> {
                let size = #struct_name::size_of(field);
                if value.len() > size {
                    return Err(::fixed_record::error::Error::FieldOverflow {
                        field: #struct_name::name_of(field),
                        size,
                        actual: value.len(),
                    });
                }
                Ok(())
            }

            #[doc = "Appends a record and returns its current index as the ID."]
            #[doc = "レコードを末尾へ追加し、現在の index を ID として返します。"]
            pub fn push(&mut self, record: #struct_name) -> usize {
                let id = self.records.len();
                self.records.push(Box::new(record));
                Self::index_record(&mut self.indices, id, self.records[id].as_ref());
                id
            }

            #[doc = "Inserts a record at the specified index and updates shifted field indexes."]
            #[doc = "指定 index にレコードを挿入し、移動したフィールド索引を更新します。"]
            #[doc = "Returns `false` without changing the list when `index` is greater than `len()`."]
            #[doc = "`index` が `len()` より大きい場合は List を変更せず `false` を返します。"]
            pub fn insert(&mut self, index: usize, record: #struct_name) -> bool {
                if index > self.records.len() {
                    return false;
                }
                if index == self.records.len() {
                    self.push(record);
                    return true;
                }

                self.records.insert(index, Box::new(record));
                self.increment_index_ids_at_or_after(index);
                Self::index_record(&mut self.indices, index, self.records[index].as_ref());
                true
            }

            #[doc = "Returns the record at the specified current index."]
            #[doc = "指定された現在の index にあるレコードを返します。"]
            pub fn get(&self, id: usize) -> Option<&#struct_name> {
                self.records.get(id).map(|record| record.as_ref())
            }

            #[doc = "Replaces the record at the specified current index and updates all field indexes."]
            #[doc = "指定された現在の index にあるレコードを置き換え、全フィールド索引を更新します。"]
            pub fn update(&mut self, id: usize, record: #struct_name) -> bool {
                let Some(slot) = self.records.get_mut(id) else {
                    return false;
                };

                Self::unindex_record(&mut self.indices, id, slot.as_ref());
                **slot = record;
                Self::index_record(&mut self.indices, id, slot.as_ref());
                true
            }

            #[doc = "Removes the record at the specified current index and updates shifted field indexes."]
            #[doc = "指定された現在の index にあるレコードを削除し、移動したフィールド索引を更新します。"]
            pub fn remove(&mut self, id: usize) -> bool {
                if id >= self.records.len() {
                    return false;
                }

                let record = self.records.remove(id);
                Self::unindex_record(&mut self.indices, id, record.as_ref());
                self.decrement_index_ids_after(id);
                true
            }

            #[doc = "Removes and returns the last record without rebuilding the field indexes."]
            #[doc = "フィールド索引を再構築せず、末尾のレコードを削除して返します。"]
            pub fn pop(&mut self) -> Option<#struct_name> {
                let id = self.records.len().checked_sub(1)?;
                let record = self.records.pop()?;
                Self::unindex_record(&mut self.indices, id, record.as_ref());
                Some(*record)
            }

            #[doc = "Returns records whose specified field exactly matches the value using the field index."]
            #[doc = "フィールド索引を使い、指定フィールドが値と完全一致するレコードを返します。"]
            #[doc = "Returns `Error::TooShort` when the search value is shorter than the field."]
            #[doc = "検索値がフィールド幅より短い場合は `Error::TooShort` を返します。"]
            #[doc = "Returns `Error::FieldOverflow` when the search value is wider than the field."]
            #[doc = "検索値がフィールド幅を超える場合は `Error::FieldOverflow` を返します。"]
            pub fn find_by(
                &self,
                field: #field_enum_name,
                value: impl AsRef<[u8]>,
            ) -> Result<Vec<&#struct_name>, ::fixed_record::error::Error> {
                let raw_value = value.as_ref();
                let size = #struct_name::size_of(field);
                if raw_value.len() < size {
                    return Err(::fixed_record::error::Error::TooShort);
                }
                Self::validate_search_width(field, raw_value)?;

                let ids = match field {
                    #( #find_by_arms ),*
                };
                let Some(ids) = ids else {
                    return Ok(Vec::new());
                };
                Ok(ids.iter().filter_map(|id| self.get(*id)).collect())
            }

            #[doc = "Returns records whose specified field matches the value using the field index."]
            #[doc = "フィールド索引を使い、指定フィールドが値と一致するレコードを返します。"]
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
                Self::validate_search_width(field, raw_value)?;
                let ids = self.indexed_prefix_ids(field, raw_value, true);
                Ok(self.records_for_ids(ids))
            }

            #[doc = "Returns records whose specified field starts with the value using the field index."]
            #[doc = "フィールド索引を使い、指定フィールドが値で始まるレコードを返します。"]
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
                Self::validate_search_width(field, raw_value)?;
                let ids = self.indexed_prefix_ids(field, raw_value, false);
                Ok(self.records_for_ids(ids))
            }

            #[doc = "Returns records whose specified field value is within the indexed range."]
            #[doc = "指定フィールドの値が索引上の範囲内にあるレコードを返します。"]
            pub fn find_range_by<const N: usize, R>(
                &self,
                field: #field_enum_name,
                range: R,
            ) -> Vec<&#struct_name>
            where
                R: std::ops::RangeBounds<::fixed_record::Fixed<N>>,
            {
                if N != #struct_name::size_of(field) {
                    return Vec::new();
                }
                let ids = match field {
                    #( #find_range_by_arms ),*
                };
                self.records_for_ids(ids)
            }

            #[doc = "Returns record IDs within a range from one typed field index."]
            #[doc = "1つの型付きフィールド索引から範囲内のレコード ID を返します。"]
            fn range_ids_for<const M: usize, const N: usize, R>(
                field_index: &std::collections::BTreeMap<
                    ::fixed_record::Fixed<M>,
                    Vec<usize>,
                >,
                range: &R,
            ) -> Vec<usize>
            where
                R: std::ops::RangeBounds<::fixed_record::Fixed<N>>,
            {
                use std::ops::Bound::{Excluded, Included, Unbounded};

                if M != N {
                    return Vec::new();
                }
                let start = match range.start_bound() {
                    Included(value) => {
                        let Ok(value) = ::fixed_record::Fixed::<M>::from_slice(value.as_bytes()) else {
                            return Vec::new();
                        };
                        Included(value)
                    }
                    Excluded(value) => {
                        let Ok(value) = ::fixed_record::Fixed::<M>::from_slice(value.as_bytes()) else {
                            return Vec::new();
                        };
                        Excluded(value)
                    }
                    Unbounded => Unbounded,
                };
                let end = match range.end_bound() {
                    Included(value) => {
                        let Ok(value) = ::fixed_record::Fixed::<M>::from_slice(value.as_bytes()) else {
                            return Vec::new();
                        };
                        Included(value)
                    }
                    Excluded(value) => {
                        let Ok(value) = ::fixed_record::Fixed::<M>::from_slice(value.as_bytes()) else {
                            return Vec::new();
                        };
                        Excluded(value)
                    }
                    Unbounded => Unbounded,
                };
                let start_is_excluded = matches!(&start, Excluded(_));
                let end_is_excluded = matches!(&end, Excluded(_));
                let invalid_range = match (&start, &end) {
                    (
                        Included(start_value) | Excluded(start_value),
                        Included(end_value) | Excluded(end_value),
                    ) => {
                        start_value > end_value
                            || (start_value == end_value
                                && start_is_excluded
                                && end_is_excluded)
                    }
                    _ => false,
                };
                if invalid_range {
                    return Vec::new();
                }

                let mut ids = Vec::new();
                for indexed_ids in field_index.range((start, end)).map(|(_, ids)| ids) {
                    ids.extend_from_slice(indexed_ids);
                }
                ids
            }

            #[doc = "Returns an iterator over records in indexed ascending order by the specified field."]
            #[doc = "指定フィールドの索引上の昇順でレコードを返すイテレータです。"]
            pub fn iter_sorted_by<'a, const N: usize>(
                &'a self,
                field: #field_enum_name,
            ) -> impl Iterator<Item = &'a #struct_name> + 'a {
                let mut records = Vec::with_capacity(self.records.len());
                match field {
                    #( #iter_sorted_by_arms ),*
                }
                records.into_iter()
            }

            #[doc = "Sorts records by comparing all fields in declaration order and rebuilds the indexes."]
            #[doc = "レコードを定義順の全フィールド比較でソートし、索引を再構築します。"]
            pub fn sort(&mut self) {
                self.records.sort_by(|left, right| {
                    left.as_ref().compare_all_fields(right.as_ref())
                });
                self.rebuild_indices();
            }

            #[doc = "Sorts records by the specified field priority order and rebuilds the indexes."]
            #[doc = "指定したフィールド優先順でレコードをソートし、索引を再構築します。"]
            pub fn sort_by(&mut self, fields: &[#field_enum_name]) {
                self.records.sort_by(|left, right| {
                    left.as_ref().compare_by_fields(right.as_ref(), fields)
                });
                self.rebuild_indices();
            }

            #[doc = "Returns the first record in the current order."]
            #[doc = "現在の順序で最初のレコードを返します。"]
            pub fn first(&self) -> Option<&#struct_name> {
                self.records.first().map(|record| record.as_ref())
            }

            #[doc = "Returns the first record in indexed ascending order by the specified field."]
            #[doc = "指定フィールドの索引上の昇順で最初のレコードを返します。"]
            pub fn first_by<const N: usize>(&self, field: #field_enum_name) -> Option<&#struct_name> {
                self.try_first_sorted_by(field)
            }

            #[doc = "Returns the first record in indexed ascending order by the specified field."]
            #[doc = "指定フィールドの索引上の昇順で最初のレコードを返します。"]
            #[doc = "Unlike `first_by`, callers do not need to specify the field width."]
            #[doc = "`first_by` と違い、呼び出し側でフィールド幅を指定する必要はありません。"]
            pub fn try_first_sorted_by(&self, field: #field_enum_name) -> Option<&#struct_name> {
                let id = match field {
                    #( #first_sorted_by_arms ),*
                }?;
                self.get(id)
            }

            #[doc = "Returns the first indexed record whose specified field matches the value."]
            #[doc = "指定フィールドが値と一致するレコードのうち、索引上で最初のものを返します。"]
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
                Self::validate_search_width(field, raw_value)?;
                Ok(self
                    .first_indexed_prefix_id(field, raw_value, true)
                    .and_then(|id| self.get(id)))
            }

            #[doc = "Returns the first indexed prefix match for the specified field."]
            #[doc = "指定フィールドが値で始まるレコードのうち、索引上で最初のものを返します。"]
            #[doc = "Returns `Error::FieldOverflow` when the search value is wider than the field."]
            #[doc = "検索値がフィールド幅を超える場合は `Error::FieldOverflow` を返します。"]
            pub fn try_first_by_prefix(
                &self,
                field: #field_enum_name,
                value: impl AsRef<[u8]>,
            ) -> Result<Option<&#struct_name>, ::fixed_record::error::Error> {
                let raw_value = value.as_ref();
                Self::validate_search_width(field, raw_value)?;
                Ok(self
                    .first_indexed_prefix_id(field, raw_value, false)
                    .and_then(|id| self.get(id)))
            }

            #[doc = "Returns all current record IDs."]
            #[doc = "現在の全レコード ID を返します。"]
            pub fn all_ids(&self) -> Vec<usize> {
                (0..self.records.len()).collect()
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
