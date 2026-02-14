use crate::commands::attach::AttachOptions;

#[test]
fn test_attach_options_struct() {
    let opts = AttachOptions {
        name: "test-session".to_string(),
    };
    assert_eq!(opts.name, "test-session");
}