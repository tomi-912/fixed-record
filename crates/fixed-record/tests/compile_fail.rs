/// Verifies that invalid proc macro input produces readable compile errors instead of panics.
/// proc macro の不正入力が panic ではなく読みやすい compile error になることを確認します。
#[test]
fn fixed_record_reports_friendly_input_errors() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/non_struct.rs");
    t.compile_fail("tests/ui/tuple_struct.rs");
    t.compile_fail("tests/ui/non_fixed_field.rs");
    t.compile_fail("tests/ui/non_literal_len.rs");
    t.compile_fail("tests/ui/zero_len_field.rs");
    t.compile_fail("tests/ui/negative_len_field.rs");
    t.compile_fail("tests/ui/invalid_clear_byte_name.rs");
    t.compile_fail("tests/ui/immutable_list_remove.rs");
    t.compile_fail("tests/ui/immutable_list_update.rs");
    t.compile_fail("tests/ui/private_record_generated_types_are_private.rs");
    t.compile_fail("tests/ui/sequence_check_wrong_field.rs");
    t.compile_fail("tests/ui/sequence_check_too_many_fields.rs");
}
