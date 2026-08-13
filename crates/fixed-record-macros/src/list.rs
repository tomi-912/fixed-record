use crate::core::FieldMeta;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::DeriveInput;

/// Generates the optional `{StructName}List` helper and its index operations.
/// optional な `{StructName}List` 補助型と index 操作を生成します。
pub(super) fn gen_list_impl(input: &DeriveInput, metas: &[FieldMeta<'_>]) -> TokenStream {
    if !cfg!(feature = "list") {
        return quote!();
    }

    let struct_name = &input.ident;
    let struct_vis = &input.vis;
    let field_enum_name = format_ident!("{}Field", struct_name);
    let entry_name = format_ident!("{}Entry", struct_name);
    let list_name = format_ident!("{}List", struct_name);

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
                                ::fixed_record::Fixed<#size>,
                                std::collections::BTreeSet<usize>
                            >::new()
                        )
                    });

                if let Some(map) = tree.downcast_mut::<
                    std::collections::BTreeMap<
                        ::fixed_record::Fixed<#size>,
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
                            ::fixed_record::Fixed<#size>,
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
                            ::fixed_record::Fixed<#size>,
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
                            ::fixed_record::Fixed<#size>,
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
                                ::fixed_record::Fixed<#size>,
                                std::collections::BTreeSet<usize>
                            >::new()
                        )
                    });

                if let Some(map) = tree.downcast_mut::<
                    std::collections::BTreeMap<
                        ::fixed_record::Fixed<#size>,
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
                    return Err(::fixed_record::error::Error::FieldOverflow {
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
                        ::fixed_record::Fixed<#size>,
                        std::collections::BTreeSet<usize>
                    >
                >() else {
                    return Ok(Vec::new());
                };

                let ids: Vec<usize> = if raw_value.len() == #size {
                    let value = ::fixed_record::Fixed::<#size>::from_slice(raw_value)?;
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
                    return Err(::fixed_record::error::Error::FieldOverflow {
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
                        ::fixed_record::Fixed<#size>,
                        std::collections::BTreeSet<usize>
                    >
                >() else {
                    return Ok(Vec::new());
                };

                let ids: Vec<usize> = if raw_value.len() == #size {
                    let value = ::fixed_record::Fixed::<#size>::from_slice(raw_value)?;
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
                        ::fixed_record::Fixed<#size>,
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
                    return Err(::fixed_record::error::Error::FieldOverflow {
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
                        ::fixed_record::Fixed<#size>,
                        std::collections::BTreeSet<usize>
                    >
                >() else {
                    return Ok(None);
                };

                let record = if raw_value.len() == #size {
                    let value = ::fixed_record::Fixed::<#size>::from_slice(raw_value)?;
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
                    return Err(::fixed_record::error::Error::FieldOverflow {
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
                        ::fixed_record::Fixed<#size>,
                        std::collections::BTreeSet<usize>
                    >
                >() else {
                    return Ok(None);
                };

                let record = if raw_value.len() == #size {
                    let value = ::fixed_record::Fixed::<#size>::from_slice(raw_value)?;
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

    quote! {
        struct #entry_name {
            record: #struct_name,
            is_deleted: bool,
        }

        #[doc = "Stores a collection of records and manages indexes for search, removal, and sorting."]
        #[doc = "レコードのコレクションを保持し、検索・削除・ソート用インデックスを管理します。"]
        #struct_vis struct #list_name {
            records: std::collections::BTreeMap<usize, #entry_name>,
            next_id: usize,
            indices: std::collections::HashMap<#field_enum_name, Box<dyn std::any::Any>>,
            order: Vec<usize>,
        }

        impl #list_name {
            #[doc = "Creates an empty list."]
            #[doc = "空のリストを作成します。"]
            pub fn new() -> Self {
                Self {
                    records: std::collections::BTreeMap::new(),
                    next_id: 0,
                    indices: std::collections::HashMap::new(),
                    order: Vec::new(),
                }
            }

            #[doc = "Returns the number of active records."]
            #[doc = "有効なレコード数を返します。"]
            pub fn len(&self) -> usize {
                self.records.values().filter(|entry| !entry.is_deleted).count()
            }

            #[doc = "Returns whether there are no active records."]
            #[doc = "有効なレコードがないかを返します。"]
            pub fn is_empty(&self) -> bool {
                self.len() == 0
            }

            #[doc = "Returns an iterator over active records in the current order."]
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

            #[doc = "Inserts a record and returns its assigned ID."]
            #[doc = "レコードを追加し、採番された ID を返します。"]
            pub fn insert(&mut self, record: #struct_name) -> usize {
                let id = self.next_id;
                self.next_id += 1;

                #( #index_insert_blocks )*

                self.records.insert(id, #entry_name { record, is_deleted: false });
                self.order.push(id);
                id
            }

            #[doc = "Returns the active record with the specified ID."]
            #[doc = "指定 ID の有効なレコードを返します。"]
            pub fn get(&self, id: usize) -> Option<&#struct_name> {
                let entry = self.records.get(&id)?;
                if entry.is_deleted {
                    None
                } else {
                    Some(&entry.record)
                }
            }

            #[doc = "Replaces the active record with the specified ID and updates search indexes."]
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

            #[doc = "Logically removes the record with the specified ID and excludes it from search indexes."]
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

            #[doc = "Returns active records whose specified field exactly matches the value."]
            #[doc = "指定フィールドが値と完全一致する有効なレコードを返します。"]
            pub fn find_by<const N: usize>(
                &self,
                field: #field_enum_name,
                value: impl Into<::fixed_record::Fixed<N>>,
            ) -> Vec<&#struct_name> {
                let value = value.into();
                self.indices.get(&field)
                    .and_then(|tree| {
                        tree.downcast_ref::<
                            std::collections::BTreeMap<
                                ::fixed_record::Fixed<N>,
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

            #[doc = "Returns active records whose specified field matches the value."]
            #[doc = "指定フィールドが値と一致する有効なレコードを返します。"]
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

            #[doc = "Returns active records whose specified field starts with the value."]
            #[doc = "指定フィールドが値で始まる有効なレコードを返します。"]
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

            #[doc = "Returns active records whose specified field value is within the range."]
            #[doc = "指定フィールドの値が範囲内にある有効なレコードを返します。"]
            pub fn find_range_by<const N: usize, R>(
                &self,
                field: #field_enum_name,
                range: R,
            ) -> Vec<&#struct_name>
            where
                R: std::ops::RangeBounds<::fixed_record::Fixed<N>>,
            {
                self.indices.get(&field)
                    .and_then(|tree| {
                        tree.downcast_ref::<
                            std::collections::BTreeMap<
                                ::fixed_record::Fixed<N>,
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

            #[doc = "Returns an iterator over active records sorted by the specified field in ascending order."]
            #[doc = "指定フィールドで昇順に並ぶ有効レコードのイテレータを返します。"]
            pub fn iter_sorted_by<'a, const N: usize>(
                &'a self,
                field: #field_enum_name,
            ) -> impl Iterator<Item = &'a #struct_name> + 'a {
                self.indices.get(&field)
                    .and_then(|tree| {
                        tree.downcast_ref::<
                            std::collections::BTreeMap<
                                ::fixed_record::Fixed<N>,
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

            #[doc = "Sorts the current active records by comparing all fields in declaration order."]
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

            #[doc = "Sorts the current active records by the specified field priority order."]
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

            #[doc = "Returns the first active record in the current ID order."]
            #[doc = "有効なレコードのうち、現在の ID 順で最初のものを返します。"]
            pub fn first(&self) -> Option<&#struct_name> {
                self.records
                    .values()
                    .find(|entry| !entry.is_deleted)
                    .map(|entry| &entry.record)
            }

            #[doc = "Returns the first active record in ascending order by the specified field."]
            #[doc = "指定フィールドの昇順で最初の有効レコードを返します。"]
            pub fn first_by<const N: usize>(&self, field: #field_enum_name) -> Option<&#struct_name> {
                self.iter_sorted_by::<N>(field).next()
            }

            #[doc = "Returns the first active record in ascending order by the specified field."]
            #[doc = "指定フィールドの昇順で最初の有効レコードを返します。"]
            #[doc = "Unlike `first_by`, callers do not need to specify the field width."]
            #[doc = "`first_by` と違い、呼び出し側でフィールド幅を指定する必要はありません。"]
            pub fn try_first_sorted_by(&self, field: #field_enum_name) -> Option<&#struct_name> {
                match field {
                    #( #try_first_sorted_by_arms ),*
                }
            }

            #[doc = "Returns the first active matching record in ascending order by the specified field."]
            #[doc = "指定フィールドが値と一致する有効レコードのうち、指定フィールドの昇順で最初のものを返します。"]
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

            #[doc = "Returns the first active prefix match in ascending order by the specified field."]
            #[doc = "指定フィールドが値で始まる有効レコードのうち、指定フィールドの昇順で最初のものを返します。"]
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

            #[doc = "Returns all physically stored IDs, including logically removed records."]
            #[doc = "論理削除済みレコードを含め、物理的に保持している全 ID を返します。"]
            pub fn all_ids(&self) -> Vec<usize> {
                self.records.keys().copied().collect()
            }

            #[doc = "Physically removes logically deleted records and cleans indexes."]
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
            #[doc = "Creates an empty list."]
            #[doc = "空のリストを作成します。"]
            fn default() -> Self {
                Self::new()
            }
        }
    }
}
