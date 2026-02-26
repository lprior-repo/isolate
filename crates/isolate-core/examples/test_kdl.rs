use isolate_core::kdl_validation::validate_kdl_syntax;

fn main() {
    let valid_kdl = "layout { pane { command \"bash\" } }";
    let result = validate_kdl_syntax(valid_kdl);
    println!("Result: {result:?}");

    let valid_kdl2 = r#"layout {
    pane {
        command "bash"
    }
}"#;
    let result2 = validate_kdl_syntax(valid_kdl2);
    println!("Result2: {result2:?}");
}
