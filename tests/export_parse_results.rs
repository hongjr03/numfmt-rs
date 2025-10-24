use serde::{Deserialize, Serialize};
use std::fs;
use numfmt_rs::{parse_pattern, tokenize};

#[derive(Debug, Serialize, Deserialize)]
struct TestResult {
    index: usize,
    input: String,
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pattern: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tokens: Option<serde_json::Value>,
}

#[test]
fn export_parsing_results_to_json() {
    // Load all format strings from JSON
    let formats_json = include_str!("./fixtures/numfmt/formats.json");
    let formats: Vec<String> =
        serde_json::from_str(formats_json).expect("Failed to parse formats.json");

    let mut results = Vec::new();

    for (idx, format) in formats.iter().enumerate() {
        let tokens_result = tokenize(format);
        let parse_result = parse_pattern(format);

        let (success, error, pattern, tokens) = match (&parse_result, &tokens_result) {
            (Ok(pat), Ok(toks)) => {
                let pattern_json = serde_json::to_value(
                    pat.partitions
                        .iter()
                        .map(|s| format!("{:?}", s.tokens))
                        .collect::<Vec<String>>(),
                )
                .ok();
                let tokens_json = serde_json::to_value(
                    toks.iter()
                        .map(|x| format!("{:?}", x.value))
                        .collect::<String>(),
                )
                .ok();
                (true, None, pattern_json, tokens_json)
            }
            (Err(e), _) => (false, Some(e.to_string()), None, None),
            (_, Err(e)) => (false, Some(e.to_string()), None, None),
        };

        results.push(TestResult {
            index: idx,
            input: format.clone(),
            success,
            error,
            pattern,
            tokens,
        });
    }

    // Export to JSON file
    let json = serde_json::to_string_pretty(&results).expect("Failed to serialize results");
    fs::write("test_parse_results.json", json).expect("Failed to write JSON file");

    // Also create a summary
    let succeeded = results.iter().filter(|r| r.success).count();
    let failed = results.iter().filter(|r| !r.success).count();

    let summary = serde_json::json!({
        "total": formats.len(),
        "succeeded": succeeded,
        "failed": failed,
        "success_rate": format!("{:.2}%", (succeeded as f64 / formats.len() as f64) * 100.0),
    });

    fs::write(
        "test_summary.json",
        serde_json::to_string_pretty(&summary).unwrap(),
    )
    .expect("Failed to write summary");

    eprintln!("\n=== Export Complete ===");
    eprintln!("Results exported to: test_parse_results.json");
    eprintln!("Summary exported to: test_summary.json");
    eprintln!(
        "Total: {}, Succeeded: {}, Failed: {}",
        formats.len(),
        succeeded,
        failed
    );
}
