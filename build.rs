use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated_tests.rs");
    let mut output = fs::File::create(dest_path).unwrap();

    let fixtures_dir = Path::new("tests/fixtures/numfmt/generated");
    
    // Tell Cargo to rerun if test files change
    println!("cargo:rerun-if-changed={}", fixtures_dir.display());

    if !fixtures_dir.exists() {
        panic!("Test fixtures directory not found: {}", fixtures_dir.display());
    }

    let mut entries: Vec<_> = fs::read_dir(fixtures_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s == "json")
                .unwrap_or(false)
        })
        .collect();
    
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_str().unwrap();
        let stem = path.file_stem().unwrap().to_str().unwrap();
        
        // Convert filename to module name (e.g., "comma-test" -> "comma_test")
        let module_name = stem.replace('-', "_");

        // Read the JSON file to count test cases
        let content = fs::read_to_string(&path).unwrap();
        let test_cases: Vec<serde_json::Value> = serde_json::from_str(&content).unwrap();
        let test_count = test_cases.len();

        writeln!(output, "#[cfg(test)]").unwrap();
        writeln!(output, "mod {} {{", module_name).unwrap();
        writeln!(output, "    use super::*;").unwrap();
        writeln!(output).unwrap();
        writeln!(output, "    fn get_test_cases() -> &'static Vec<TestCase> {{").unwrap();
        writeln!(output, "        use std::sync::OnceLock;").unwrap();
        writeln!(output, "        static TEST_CASES: OnceLock<Vec<TestCase>> = OnceLock::new();").unwrap();
        writeln!(output, "        TEST_CASES.get_or_init(|| load_test_cases(\"{}\"))", file_name).unwrap();
        writeln!(output, "    }}").unwrap();
        writeln!(output).unwrap();
        writeln!(output, "    fn run_test(index: usize) {{").unwrap();
        writeln!(output, "        let test_cases = get_test_cases();").unwrap();
        writeln!(output, "        assert!(").unwrap();
        writeln!(output, "            index < test_cases.len(),").unwrap();
        writeln!(output, "            \"Test index {{}} out of bounds (total: {{}})\",").unwrap();
        writeln!(output, "            index,").unwrap();
        writeln!(output, "            test_cases.len()").unwrap();
        writeln!(output, "        );").unwrap();
        writeln!(output, "        let test_case = &test_cases[index];").unwrap();
        writeln!(output, "        let value = convert_json_value(&test_case.value);").unwrap();
        writeln!(output).unwrap();
        writeln!(output, "        let result = if let Some(ref opts) = test_case.options {{").unwrap();
        writeln!(output, "            let options = parse_options(Some(opts));").unwrap();
        writeln!(output, "            format_with_options(&test_case.pattern, value, options)").unwrap();
        writeln!(output, "        }} else {{").unwrap();
        writeln!(output, "            format(&test_case.pattern, value)").unwrap();
        writeln!(output, "        }};").unwrap();
        writeln!(output).unwrap();
        writeln!(output, "        match result {{").unwrap();
        writeln!(output, "            Ok(output) => {{").unwrap();
        writeln!(output, "                assert_eq!(").unwrap();
        writeln!(output, "                    output, test_case.expected,").unwrap();
        writeln!(output, "                    \"Test case {{}}: pattern='{{}}', value={{:?}}, options={{:?}}\",").unwrap();
        writeln!(output, "                    index, test_case.pattern, test_case.value, test_case.options").unwrap();
        writeln!(output, "                );").unwrap();
        writeln!(output, "            }}").unwrap();
        writeln!(output, "            Err(e) => {{").unwrap();
        writeln!(output, "                panic!(").unwrap();
        writeln!(output, "                    \"Test case {{}} failed: pattern='{{}}', value={{:?}}, options={{:?}}, error: {{:?}}\",").unwrap();
        writeln!(output, "                    index, test_case.pattern, test_case.value, test_case.options, e").unwrap();
        writeln!(output, "                );").unwrap();
        writeln!(output, "            }}").unwrap();
        writeln!(output, "        }}").unwrap();
        writeln!(output, "    }}").unwrap();
        writeln!(output).unwrap();

        // Generate individual test functions
        for i in 0..test_count {
            writeln!(output, "    #[test]").unwrap();
            writeln!(output, "    fn test_case_{:04}() {{", i).unwrap();
            writeln!(output, "        run_test({});", i).unwrap();
            writeln!(output, "    }}").unwrap();
            writeln!(output).unwrap();
        }

        writeln!(output, "}}").unwrap();
        writeln!(output).unwrap();
    }

    println!("cargo:rerun-if-changed=build.rs");
}
