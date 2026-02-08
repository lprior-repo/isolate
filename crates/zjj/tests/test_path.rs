#[test]
fn test_print_path() {
    println!("PATH: {}", std::env::var("PATH").unwrap_or_default());
}
