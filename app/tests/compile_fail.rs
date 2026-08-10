/// proc macro の不正入力が panic ではなく読みやすい compile error になることを確認します。
#[test]
fn fixed_record_main_reports_friendly_input_errors() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/non_struct.rs");
    t.compile_fail("tests/ui/tuple_struct.rs");
    t.compile_fail("tests/ui/non_fixed_field.rs");
    t.compile_fail("tests/ui/non_literal_len.rs");
}
