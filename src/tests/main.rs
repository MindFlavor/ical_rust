use super::main;

#[test]
fn test_issue18_main_does_not_panic_when_file_missing() {
    main();
}
