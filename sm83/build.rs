use std::{ffi::OsStr, io::Write};

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let test_file = std::path::Path::new(&out_dir).join("generated_tests.rs");
    let mut f = std::fs::File::create(test_file).unwrap();

    let test_suites = std::fs::read_dir("tests/data")
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.unwrap();
            let file_type = entry.file_type().unwrap();
            let path = entry.path();
            if file_type.is_file() && path.extension().is_some_and(|e| e == OsStr::new("toml")) {
                Some(path)
            } else {
                None
            }
        });

    println!("cargo:rerun-if-changed=tests/data");

    for test_suite in test_suites {
        let test_suite_name = test_suite.file_stem().and_then(|e| e.to_str()).unwrap();
        let test_file = test_suite.canonicalize().unwrap();

        write!(
            f,
            "
#[test]
fn {test_suite_name}_test() {{
    let tests = include_str!(\"{test_file}\");
    let test_suite = \"{test_suite_name}\";
    run_test(test_suite, tests);
}}",
            test_suite_name = test_suite_name,
            test_file = test_file.display()
        )
        .unwrap();
    }
}
