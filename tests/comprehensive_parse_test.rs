// Comprehensive parsing tests extracted from JavaScript test suite
// These tests verify that all format strings from the original test suite can be parsed

use numfmt_rs::parse_pattern;

#[test]
fn test_all_valid_formats_can_parse() {
    // Load all format strings from JSON
    let formats_json = include_str!("./fixtures/numfmt/formats.json");
    let formats: Vec<String> =
        serde_json::from_str(formats_json).expect("Failed to parse formats.json");

    let mut failed = Vec::new();
    let mut succeeded = 0;

    for (idx, format) in formats.iter().enumerate() {
        match parse_pattern(format) {
            Ok(_pattern) => {
                succeeded += 1;
            }
            Err(e) => {
                failed.push((idx, format.clone(), e.to_string()));
            }
        }
    }

    // Print summary
    eprintln!("\n=== Parse Test Summary ===");
    eprintln!("Total formats: {}", formats.len());
    eprintln!("Succeeded: {}", succeeded);
    eprintln!("Failed: {}", failed.len());

    if !failed.is_empty() {
        eprintln!("\n=== Failed Formats ===");
        for (idx, fmt, err) in &failed {
            eprintln!("#{}: '{}' - {}", idx, fmt, err);
        }
    }

    // Assert all passed
    assert!(
        failed.is_empty(),
        "\n{} format strings failed to parse. See details above.",
        failed.len()
    );
}

#[test]
fn test_specific_format_samples() {
    // Test some key formats individually for better error messages
    let samples = vec![
        "#,##0.00",
        "0.00%",
        "yyyy-mm-dd",
        "h:mm:ss AM/PM",
        "#,##0;(#,##0)",
        "0.00E+00",
        "[Red]#,##0.00",
        "[$$-409]#,##0.00",
        "# ?/?",
        "m/d/yy",
        "@",
        "General",
    ];

    for format in samples {
        let result = parse_pattern(format);
        assert!(
            result.is_ok(),
            "Failed to parse '{}': {:?}",
            format,
            result.err()
        );
    }
}

#[test]
fn test_tokenize_all_formats() {
    use numfmt_rs::tokenize;

    let formats_json = include_str!("./fixtures/numfmt/formats.json");
    let formats: Vec<String> =
        serde_json::from_str(formats_json).expect("Failed to parse formats.json");

    let mut failed = Vec::new();
    let mut succeeded = 0;

    for (idx, format) in formats.iter().enumerate() {
        match tokenize(format) {
            Ok(tokens) => {
                succeeded += 1;
                // Verify tokens cover the entire input
                let reconstructed: String = tokens.iter().map(|t| t.raw.clone()).collect();
                if reconstructed != *format {
                    eprintln!(
                        "Warning: Token reconstruction mismatch for '{}'\nGot: '{}'",
                        format, reconstructed
                    );
                }
            }
            Err(e) => {
                failed.push((idx, format.clone(), e.to_string()));
            }
        }
    }

    eprintln!("\n=== Tokenize Test Summary ===");
    eprintln!("Total formats: {}", formats.len());
    eprintln!("Succeeded: {}", succeeded);
    eprintln!("Failed: {}", failed.len());

    if !failed.is_empty() {
        eprintln!("\n=== Failed Tokenization ===");
        for (idx, fmt, err) in &failed {
            eprintln!("#{}: '{}' - {}", idx, fmt, err);
        }
    }

    assert!(
        failed.is_empty(),
        "\n{} format strings failed to tokenize",
        failed.len()
    );
}
