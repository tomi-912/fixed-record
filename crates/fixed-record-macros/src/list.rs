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
    let rebuild_guard_name = format_ident!("{}ListRebuildGuard", struct_name);
    let edit_guard_name = format_ident!("{}ListEditGuard", struct_name);

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
    let find_exact_by_arms = metas.iter().map(|meta| {
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
    let first_exact_by_arms = metas.iter().map(|meta| {
        let name = meta.name;
        let size = meta.size;
        let variant = &meta.variant;
        quote! {
            #field_enum_name::#variant => {
                let key = ::fixed_record::Fixed::<#size>::from_slice(raw_value)?;
                self.indices.#name.get(&key).and_then(|ids| ids.first()).copied()
            }
        }
    });
    let find_range_by_arms = metas.iter().map(|meta| {
        let name = meta.name;
        let size = meta.size;
        let variant = &meta.variant;
        quote! {
            #field_enum_name::#variant => {
                Self::range_ids_for::<#size, R>(&self.indices.#name, range, field_name)?
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

        #[doc(hidden)]
        struct #rebuild_guard_name<'a> {
            list: &'a mut #list_name,
        }

        impl Drop for #rebuild_guard_name<'_> {
            fn drop(&mut self) {
                self.list.rebuild_indices();
            }
        }

        #[doc(hidden)]
        struct #edit_guard_name<'a> {
            list: &'a mut #list_name,
            originals: Vec<(usize, #struct_name)>,
        }

        impl Drop for #edit_guard_name<'_> {
            fn drop(&mut self) {
                for (id, record) in &self.originals {
                    #list_name::unindex_record(&mut self.list.indices, *id, record);
                }

                let records = &self.list.records;
                let indices = &mut self.list.indices;
                for (id, _) in &self.originals {
                    if let Some(record) = records.get(*id) {
                        #list_name::index_record(indices, *id, record.as_ref());
                    }
                }
            }
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

            #[doc = "Creates a list from records in their current order and builds all field indexes."]
            #[doc = "レコードの現在順からリストを作成し、全フィールド索引を構築します。"]
            pub fn from_records(records: Vec<#struct_name>) -> Self {
                let mut indices = #indices_name::default();
                let records = records
                    .into_iter()
                    .enumerate()
                    .map(|(id, record)| {
                        Self::index_record(&mut indices, id, &record);
                        Box::new(record)
                    })
                    .collect();
                Self { records, indices }
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

            #[doc = "Removes all records and clears all field indexes."]
            #[doc = "全レコードを削除し、全フィールド索引を空にします。"]
            pub fn clear(&mut self) {
                self.records.clear();
                self.indices = #indices_name::default();
            }

            #[doc = "Retains only records for which the predicate returns `true` and rebuilds all field indexes."]
            #[doc = "predicate が `true` を返すレコードだけを残し、全フィールド索引を再構築します。"]
            #[doc = "Indexes are rebuilt by a drop guard even if the predicate unwinds."]
            #[doc = "predicate が unwind した場合も drop guard により索引を再構築します。"]
            #[deprecated(
                note = "performs a linear scan and rebuilds all field indexes; prefer indexed operations"
            )]
            pub fn retain(&mut self, mut keep: impl FnMut(&#struct_name) -> bool) {
                let mut guard = #rebuild_guard_name { list: self };
                guard
                    .list
                    .records
                    .retain(|record| keep(record.as_ref()));
            }

            #[doc = "Returns an iterator over records in the current order."]
            #[doc = "現在の順序でレコードを返すイテレータです。"]
            pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a #struct_name> + 'a {
                self.records.iter().map(|record| record.as_ref())
            }

            #[doc = "Returns the first record matching the predicate by scanning in the current order."]
            #[doc = "現在の順序で走査し、predicate に一致する最初のレコードを返します。"]
            #[deprecated(
                note = "performs a linear scan; use try_first_by*, first_sorted_by, or another indexed lookup"
            )]
            pub fn find(
                &self,
                mut predicate: impl FnMut(&#struct_name) -> bool,
            ) -> Option<&#struct_name> {
                self.records
                    .iter()
                    .map(|record| record.as_ref())
                    .find(|record| predicate(*record))
            }

            #[doc = "Returns all records matching the predicate by scanning in the current order."]
            #[doc = "現在の順序で走査し、predicate に一致する全レコードを返します。"]
            #[deprecated(
                note = "performs a linear scan; use try_find_by*, try_find_range_by, or another indexed lookup"
            )]
            pub fn find_all(
                &self,
                mut predicate: impl FnMut(&#struct_name) -> bool,
            ) -> Vec<&#struct_name> {
                self.records
                    .iter()
                    .map(|record| record.as_ref())
                    .filter(|record| predicate(*record))
                    .collect()
            }

            #[doc = "Mutates every record in the current order and rebuilds all field indexes afterward."]
            #[doc = "現在の順序で全レコードを変更し、処理後に全フィールド索引を再構築します。"]
            #[doc = "Mutable record references are confined to the callback and cannot be retained by the caller."]
            #[doc = "レコードの mutable 参照は callback 内に限定され、呼び出し側では保持できません。"]
            #[doc = "Indexes are rebuilt by a drop guard even if the callback unwinds."]
            #[doc = "callback が unwind した場合も drop guard により索引を再構築します。"]
            pub fn for_each_mut(&mut self, mut edit: impl FnMut(&mut #struct_name)) {
                let mut guard = #rebuild_guard_name { list: self };
                for record in &mut guard.list.records {
                    edit(record.as_mut());
                }
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

            #[doc = "Mutates records selected by private current indexes and repairs affected field indexes afterward."]
            #[doc = "非公開の現在 index で選択したレコードを変更し、処理後に影響するフィールド索引を修復します。"]
            fn edit_records_by_ids(
                &mut self,
                mut ids: Vec<usize>,
                mut edit: impl FnMut(&mut #struct_name),
            ) -> usize {
                ids.sort_unstable();
                ids.dedup();

                let originals: Vec<_> = ids
                    .into_iter()
                    .filter_map(|id| self.records.get(id).map(|record| (id, **record)))
                    .collect();
                let edited = originals.len();
                if originals.is_empty() {
                    return 0;
                }

                let mut guard = #edit_guard_name {
                    list: self,
                    originals,
                };
                for index in 0..guard.originals.len() {
                    let id = guard.originals[index].0;
                    edit(guard.list.records[id].as_mut());
                }
                edited
            }

            #[doc = "Mutates one record selected by a private current index and repairs its field indexes afterward."]
            #[doc = "非公開の現在 index で選択した1件を変更し、処理後にそのフィールド索引を修復します。"]
            fn edit_record_by_id(
                &mut self,
                id: Option<usize>,
                edit: impl FnOnce(&mut #struct_name),
            ) -> bool {
                let Some(id) = id else {
                    return false;
                };
                let Some(record) = self.records.get(id) else {
                    return false;
                };

                let mut guard = #edit_guard_name {
                    originals: vec![(id, **record)],
                    list: self,
                };
                edit(guard.list.records[id].as_mut());
                true
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

            #[doc = "Validates that a search value exactly matches the selected field width."]
            #[doc = "検索値が選択フィールドの幅と完全に一致することを検証します。"]
            fn validate_exact_search_width(
                field: #field_enum_name,
                value: &[u8],
            ) -> Result<(), ::fixed_record::error::Error> {
                let size = #struct_name::size_of(field);
                if value.len() < size {
                    return Err(::fixed_record::error::Error::TooShort);
                }
                Self::validate_search_width(field, value)
            }

            #[doc = "Returns private current indexes for an exact field match."]
            #[doc = "フィールドの完全一致に対応する非公開の現在 index を返します。"]
            fn try_find_ids_by(
                &self,
                field: #field_enum_name,
                value: &[u8],
            ) -> Result<Vec<usize>, ::fixed_record::error::Error> {
                Self::validate_exact_search_width(field, value)?;
                let raw_value = value;
                let ids = match field {
                    #( #find_exact_by_arms ),*
                };
                Ok(ids.cloned().unwrap_or_default())
            }

            #[doc = "Returns private current indexes matching a shortened padded field value."]
            #[doc = "短縮した padding 付きフィールド値に一致する非公開の現在 index を返します。"]
            fn try_find_ids_by_padded(
                &self,
                field: #field_enum_name,
                value: &[u8],
            ) -> Result<Vec<usize>, ::fixed_record::error::Error> {
                Self::validate_search_width(field, value)?;
                Ok(self.indexed_prefix_ids(field, value, true))
            }

            #[doc = "Returns private current indexes matching a field prefix."]
            #[doc = "フィールド prefix に一致する非公開の現在 index を返します。"]
            fn try_find_ids_by_prefix(
                &self,
                field: #field_enum_name,
                value: &[u8],
            ) -> Result<Vec<usize>, ::fixed_record::error::Error> {
                Self::validate_search_width(field, value)?;
                Ok(self.indexed_prefix_ids(field, value, false))
            }

            #[doc = "Returns private current indexes within a field range."]
            #[doc = "フィールド範囲内にある非公開の現在 index を返します。"]
            fn try_find_range_ids_by<R>(
                &self,
                field: #field_enum_name,
                range: &R,
            ) -> Result<Vec<usize>, ::fixed_record::error::Error>
            where
                R: ::fixed_record::traits::ByteRangeBounds,
            {
                let field_name = #struct_name::name_of(field);
                let ids = match field {
                    #( #find_range_by_arms ),*
                };
                Ok(ids)
            }

            #[doc = "Returns the private current index of the first exact field match."]
            #[doc = "フィールドの完全一致で最初の非公開の現在 index を返します。"]
            fn try_first_id_by(
                &self,
                field: #field_enum_name,
                value: &[u8],
            ) -> Result<Option<usize>, ::fixed_record::error::Error> {
                Self::validate_exact_search_width(field, value)?;
                let raw_value = value;
                Ok(match field {
                    #( #first_exact_by_arms ),*
                })
            }

            #[doc = "Returns the private current index of the first shortened padded-value match."]
            #[doc = "短縮した padding 付き値に一致する最初の非公開の現在 index を返します。"]
            fn try_first_id_by_padded(
                &self,
                field: #field_enum_name,
                value: &[u8],
            ) -> Result<Option<usize>, ::fixed_record::error::Error> {
                Self::validate_search_width(field, value)?;
                Ok(self.first_indexed_prefix_id(field, value, true))
            }

            #[doc = "Returns the private current index of the first field-prefix match."]
            #[doc = "フィールド prefix に一致する最初の非公開の現在 index を返します。"]
            fn try_first_id_by_prefix(
                &self,
                field: #field_enum_name,
                value: &[u8],
            ) -> Result<Option<usize>, ::fixed_record::error::Error> {
                Self::validate_search_width(field, value)?;
                Ok(self.first_indexed_prefix_id(field, value, false))
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
            pub fn try_find_by(
                &self,
                field: #field_enum_name,
                value: impl AsRef<[u8]>,
            ) -> Result<Vec<&#struct_name>, ::fixed_record::error::Error> {
                let ids = self.try_find_ids_by(field, value.as_ref())?;
                Ok(self.records_for_ids(ids))
            }

            #[doc = "Returns records whose specified field matches a possibly shortened padded value."]
            #[doc = "指定フィールドが、短縮可能な padding 付きの値と一致するレコードを返します。"]
            #[doc = "When the search value is shorter than the field width, trailing `0x00` or space bytes are accepted."]
            #[doc = "検索値がフィールド幅より短い場合は、後続バイトが `0x00` または半角スペースのレコードも一致します。"]
            #[doc = "Returns `Error::FieldOverflow` when the search value is wider than the field."]
            #[doc = "検索値がフィールド幅を超える場合は `Error::FieldOverflow` を返します。"]
            pub fn try_find_by_padded(
                &self,
                field: #field_enum_name,
                value: impl AsRef<[u8]>,
            ) -> Result<Vec<&#struct_name>, ::fixed_record::error::Error> {
                let ids = self.try_find_ids_by_padded(field, value.as_ref())?;
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
                let ids = self.try_find_ids_by_prefix(field, value.as_ref())?;
                Ok(self.records_for_ids(ids))
            }

            #[doc = "Returns records whose specified field value is within the indexed range."]
            #[doc = "指定フィールドの値が索引上の範囲内にあるレコードを返します。"]
            #[doc = "Short bounds allow any trailing field bytes."]
            #[doc = "短い境界値では、フィールドの後続バイトを任意として扱います。"]
            #[doc = "Returns `Error::FieldOverflow` when either bound is wider than the field."]
            #[doc = "いずれかの境界値がフィールド幅を超える場合は `Error::FieldOverflow` を返します。"]
            #[doc = "Returns `Error::InvalidRange` when the start value is greater than the end value."]
            #[doc = "開始値が終了値より大きい場合は `Error::InvalidRange` を返します。"]
            pub fn try_find_range_by<R>(
                &self,
                field: #field_enum_name,
                range: R,
            ) -> Result<Vec<&#struct_name>, ::fixed_record::error::Error>
            where
                R: ::fixed_record::traits::ByteRangeBounds,
            {
                let ids = self.try_find_range_ids_by(field, &range)?;
                Ok(self.records_for_ids(ids))
            }

            #[doc = "Returns record IDs within a range from one typed field index."]
            #[doc = "1つの型付きフィールド索引から範囲内のレコード ID を返します。"]
            fn range_ids_for<const N: usize, R>(
                field_index: &std::collections::BTreeMap<
                    ::fixed_record::Fixed<N>,
                    Vec<usize>,
                >,
                range: &R,
                field_name: &'static str,
            ) -> Result<Vec<usize>, ::fixed_record::error::Error>
            where
                R: ::fixed_record::traits::ByteRangeBounds,
            {
                use std::ops::Bound::{Excluded, Included, Unbounded};

                let start_bound = range.start_bound_bytes();
                let end_bound = range.end_bound_bytes();
                let start_bytes = match start_bound {
                    Included(value) | Excluded(value) => Some(value),
                    Unbounded => None,
                };
                let end_bytes = match end_bound {
                    Included(value) | Excluded(value) => Some(value),
                    Unbounded => None,
                };

                for value in [start_bytes, end_bytes].into_iter().flatten() {
                    if value.len() > N {
                        return Err(::fixed_record::error::Error::FieldOverflow {
                            field: field_name,
                            size: N,
                            actual: value.len(),
                        });
                    }
                }

                let start = match start_bound {
                    Included(value) => {
                        let mut bound = ::fixed_record::Fixed::<N>::zeroed();
                        bound.write_bytes(value);
                        Included(bound)
                    }
                    Excluded(value) => {
                        let mut bound = ::fixed_record::Fixed::<N>::zeroed();
                        bound.write_bytes(value);
                        Excluded(bound)
                    }
                    Unbounded => Unbounded,
                };
                let end = match end_bound {
                    Included(value) => {
                        let mut bound = ::fixed_record::Fixed::<N>::filled(u8::MAX);
                        bound.write_bytes(value);
                        Included(bound)
                    }
                    Excluded(value) => {
                        let mut bound = ::fixed_record::Fixed::<N>::filled(u8::MAX);
                        bound.write_bytes(value);
                        Excluded(bound)
                    }
                    Unbounded => Unbounded,
                };
                let start_is_excluded = matches!(&start, Excluded(_));
                let end_is_excluded = matches!(&end, Excluded(_));
                let bounds_order = match (&start, &end) {
                    (
                        Included(start_value) | Excluded(start_value),
                        Included(end_value) | Excluded(end_value),
                    ) => Some(start_value.cmp(end_value)),
                    _ => None,
                };
                if bounds_order == Some(std::cmp::Ordering::Greater) {
                    return Err(::fixed_record::error::Error::InvalidRange {
                        field: field_name,
                        start: start_bytes.unwrap_or_default().to_vec(),
                        end: end_bytes.unwrap_or_default().to_vec(),
                    });
                }
                if bounds_order == Some(std::cmp::Ordering::Equal)
                    && start_is_excluded
                    && end_is_excluded
                {
                    return Ok(Vec::new());
                }

                let mut ids = Vec::new();
                for indexed_ids in field_index.range((start, end)).map(|(_, ids)| ids) {
                    ids.extend_from_slice(indexed_ids);
                }
                Ok(ids)
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

            #[doc = "Returns the first record whose specified field exactly matches the value."]
            #[doc = "指定フィールドが値と完全一致する最初のレコードを返します。"]
            #[doc = "Returns `Error::TooShort` when the search value is shorter than the field."]
            #[doc = "検索値がフィールド幅より短い場合は `Error::TooShort` を返します。"]
            #[doc = "Returns `Error::FieldOverflow` when the search value is wider than the field."]
            #[doc = "検索値がフィールド幅を超える場合は `Error::FieldOverflow` を返します。"]
            pub fn try_first_by(
                &self,
                field: #field_enum_name,
                value: impl AsRef<[u8]>,
            ) -> Result<Option<&#struct_name>, ::fixed_record::error::Error> {
                let id = self.try_first_id_by(field, value.as_ref())?;
                Ok(id.and_then(|id| self.get(id)))
            }

            #[doc = "Returns the first record in indexed ascending order by the specified field."]
            #[doc = "指定フィールドの索引上の昇順で最初のレコードを返します。"]
            pub fn first_sorted_by(&self, field: #field_enum_name) -> Option<&#struct_name> {
                let id = match field {
                    #( #first_sorted_by_arms ),*
                }?;
                self.get(id)
            }

            #[doc = "Returns the first indexed record matching a possibly shortened padded value."]
            #[doc = "短縮可能な padding 付きの値と一致する、索引上で最初のレコードを返します。"]
            #[doc = "When the search value is shorter than the field width, trailing `0x00` or space bytes are accepted."]
            #[doc = "検索値がフィールド幅より短い場合は、後続バイトが `0x00` または半角スペースのレコードも一致します。"]
            #[doc = "Returns `Error::FieldOverflow` when the search value is wider than the field."]
            #[doc = "検索値がフィールド幅を超える場合は `Error::FieldOverflow` を返します。"]
            pub fn try_first_by_padded(
                &self,
                field: #field_enum_name,
                value: impl AsRef<[u8]>,
            ) -> Result<Option<&#struct_name>, ::fixed_record::error::Error> {
                let id = self.try_first_id_by_padded(field, value.as_ref())?;
                Ok(id.and_then(|id| self.get(id)))
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
                let id = self.try_first_id_by_prefix(field, value.as_ref())?;
                Ok(id.and_then(|id| self.get(id)))
            }

            #[doc = "Mutates every record whose specified field exactly matches the value."]
            #[doc = "指定フィールドが値と完全一致する全レコードを変更します。"]
            #[doc = "Returns the number of edited records without exposing their internal current indexes."]
            #[doc = "内部の現在 index を公開せず、変更したレコード数を返します。"]
            pub fn try_edit_by(
                &mut self,
                field: #field_enum_name,
                value: impl AsRef<[u8]>,
                edit: impl FnMut(&mut #struct_name),
            ) -> Result<usize, ::fixed_record::error::Error> {
                let ids = self.try_find_ids_by(field, value.as_ref())?;
                Ok(self.edit_records_by_ids(ids, edit))
            }

            #[doc = "Mutates every record matching a possibly shortened padded field value."]
            #[doc = "短縮可能な padding 付きフィールド値に一致する全レコードを変更します。"]
            #[doc = "Returns the number of edited records without exposing their internal current indexes."]
            #[doc = "内部の現在 index を公開せず、変更したレコード数を返します。"]
            pub fn try_edit_by_padded(
                &mut self,
                field: #field_enum_name,
                value: impl AsRef<[u8]>,
                edit: impl FnMut(&mut #struct_name),
            ) -> Result<usize, ::fixed_record::error::Error> {
                let ids = self.try_find_ids_by_padded(field, value.as_ref())?;
                Ok(self.edit_records_by_ids(ids, edit))
            }

            #[doc = "Mutates every record whose specified field starts with the value."]
            #[doc = "指定フィールドが値で始まる全レコードを変更します。"]
            #[doc = "Returns the number of edited records without exposing their internal current indexes."]
            #[doc = "内部の現在 index を公開せず、変更したレコード数を返します。"]
            pub fn try_edit_by_prefix(
                &mut self,
                field: #field_enum_name,
                value: impl AsRef<[u8]>,
                edit: impl FnMut(&mut #struct_name),
            ) -> Result<usize, ::fixed_record::error::Error> {
                let ids = self.try_find_ids_by_prefix(field, value.as_ref())?;
                Ok(self.edit_records_by_ids(ids, edit))
            }

            #[doc = "Mutates every record whose specified field value is within the indexed range."]
            #[doc = "指定フィールドの値が索引上の範囲内にある全レコードを変更します。"]
            #[doc = "Returns the number of edited records without exposing their internal current indexes."]
            #[doc = "内部の現在 index を公開せず、変更したレコード数を返します。"]
            pub fn try_edit_range_by<R>(
                &mut self,
                field: #field_enum_name,
                range: R,
                edit: impl FnMut(&mut #struct_name),
            ) -> Result<usize, ::fixed_record::error::Error>
            where
                R: ::fixed_record::traits::ByteRangeBounds,
            {
                let ids = self.try_find_range_ids_by(field, &range)?;
                Ok(self.edit_records_by_ids(ids, edit))
            }

            #[doc = "Mutates the first record whose specified field exactly matches the value."]
            #[doc = "指定フィールドが値と完全一致する最初のレコードを変更します。"]
            #[doc = "Returns whether a record was edited without exposing its internal current index."]
            #[doc = "内部の現在 index を公開せず、レコードを変更したかを返します。"]
            pub fn try_edit_first_by(
                &mut self,
                field: #field_enum_name,
                value: impl AsRef<[u8]>,
                edit: impl FnOnce(&mut #struct_name),
            ) -> Result<bool, ::fixed_record::error::Error> {
                let id = self.try_first_id_by(field, value.as_ref())?;
                Ok(self.edit_record_by_id(id, edit))
            }

            #[doc = "Mutates the first record matching a possibly shortened padded field value."]
            #[doc = "短縮可能な padding 付きフィールド値に一致する最初のレコードを変更します。"]
            #[doc = "Returns whether a record was edited without exposing its internal current index."]
            #[doc = "内部の現在 index を公開せず、レコードを変更したかを返します。"]
            pub fn try_edit_first_by_padded(
                &mut self,
                field: #field_enum_name,
                value: impl AsRef<[u8]>,
                edit: impl FnOnce(&mut #struct_name),
            ) -> Result<bool, ::fixed_record::error::Error> {
                let id = self.try_first_id_by_padded(field, value.as_ref())?;
                Ok(self.edit_record_by_id(id, edit))
            }

            #[doc = "Mutates the first record whose specified field starts with the value."]
            #[doc = "指定フィールドが値で始まる最初のレコードを変更します。"]
            #[doc = "Returns whether a record was edited without exposing its internal current index."]
            #[doc = "内部の現在 index を公開せず、レコードを変更したかを返します。"]
            pub fn try_edit_first_by_prefix(
                &mut self,
                field: #field_enum_name,
                value: impl AsRef<[u8]>,
                edit: impl FnOnce(&mut #struct_name),
            ) -> Result<bool, ::fixed_record::error::Error> {
                let id = self.try_first_id_by_prefix(field, value.as_ref())?;
                Ok(self.edit_record_by_id(id, edit))
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

        impl From<Vec<#struct_name>> for #list_name {
            #[doc = "Creates a list from a vector of records."]
            #[doc = "レコードの vector からリストを作成します。"]
            fn from(records: Vec<#struct_name>) -> Self {
                Self::from_records(records)
            }
        }
    }
}
