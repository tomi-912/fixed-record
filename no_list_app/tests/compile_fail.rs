#[test]
fn list_type_is_not_generated_without_list_feature() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/list_type_is_not_generated.rs");
}
