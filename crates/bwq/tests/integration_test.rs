use std::fs;
use std::process::Command;

use tempfile::TempDir;

/// Get the path to the bwq binary for testing
fn bwq_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bwq"))
}

fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if chars.next() == Some('[') {
                for next_ch in chars.by_ref() {
                    if next_ch.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

fn check_query(query: &str) -> (String, String, i32) {
    let output = bwq_cmd()
        .args(["check", "--query", query])
        .output()
        .expect("Failed to execute bwq");

    let stdout = strip_ansi_codes(&String::from_utf8_lossy(&output.stdout));
    let stderr = strip_ansi_codes(&String::from_utf8_lossy(&output.stderr));
    let exit_code = output.status.code().unwrap_or(-1);

    (stdout, stderr, exit_code)
}

fn check_query_json(query: &str) -> (String, String, i32) {
    let output = bwq_cmd()
        .args(["check", "--query", query, "--output-format", "json"])
        .output()
        .expect("Failed to execute bwq");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string(); // Don't strip ANSI from JSON
    let stderr = strip_ansi_codes(&String::from_utf8_lossy(&output.stderr));
    let exit_code = output.status.code().unwrap_or(-1);

    (stdout, stderr, exit_code)
}

fn check_file(file_path: &str) -> (String, String, i32) {
    let output = bwq_cmd()
        .args(["check", file_path])
        .output()
        .expect("Failed to execute bwq");

    let stdout = strip_ansi_codes(&String::from_utf8_lossy(&output.stdout));
    let stderr = strip_ansi_codes(&String::from_utf8_lossy(&output.stderr));
    let exit_code = output.status.code().unwrap_or(-1);

    (stdout, stderr, exit_code)
}

/// Assert command output matches expected format (similar to Ruff's assert_cmd_snapshot)
fn assert_cmd_output(cmd_result: (String, String, i32), expected: &str) {
    let (stdout, stderr, exit_code) = cmd_result;
    let success = exit_code == 0;

    let actual_output = format!(
        "success: {}\nexit_code: {}\n----- stdout -----\n{}\n----- stderr -----\n{}",
        success,
        exit_code,
        stdout.trim_end(),
        stderr.trim_end()
    );

    let expected_clean = expected.trim();
    let actual_clean = actual_output.trim();

    if expected_clean != actual_clean {
        println!("=== EXPECTED ===");
        println!("{expected_clean}");
        println!("=== ACTUAL ===");
        println!("{actual_clean}");
        panic!("Command output does not match expected format");
    }
}

#[test]
fn test_error_display_simple() {
    let cmd_result = check_query("rating:15");

    assert_cmd_output(
        cmd_result,
        r"
success: false
exit_code: 1
----- stdout -----
error[E009]: Rating must be between 0 and 5
  --> 1:1
  |
1 | rating:15
  | ^^^^^^^^^
  |
----- stderr -----
",
    );
}

#[test]
fn test_warning_display() {
    let cmd_result = check_query("apple this");

    assert_cmd_output(
        cmd_result,
        r"
success: true
exit_code: 0
----- stdout -----
warning[W001]: Potential typo: Two or more terms without an operator between them are implicitly ANDed. Consider using explicit 'AND' operator for clarity
  --> 1:6
  |
1 | apple this
  |      ^
  |
----- stderr -----
",
    );
}

#[test]
fn test_multiline_error() {
    let query = "apple AND juice AND\nrating:15\nAND something AND else";
    let cmd_result = check_query(query);

    assert_cmd_output(
        cmd_result,
        r"
success: false
exit_code: 1
----- stdout -----
error[E009]: Rating must be between 0 and 5
  --> 2:1
  |
1 | apple AND juice AND
2 | rating:15
  | ^^^^^^^^^
3 | AND something AND else
  |
----- stderr -----
",
    );
}

#[test]
fn test_long_line_truncation() {
    let long_query = format!(
        "this is a very long query with many terms {} rating:15 more terms here",
        "word ".repeat(50)
    );
    let (stdout, stderr, exit_code) = check_query(&long_query);

    assert_eq!(exit_code, 1);
    assert!(stderr.is_empty());

    // Check that the line is truncated but error location is preserved
    assert!(stdout.contains("error[E009]: Rating must be between 0 and 5"));
    assert!(stdout.contains("…")); // Should contain ellipsis for truncation
    assert!(stdout.contains("rating:15"));
    assert!(stdout.contains("^^^^^^^^^"));
}

#[test]
fn test_double_digit_line_numbers() {
    let query = (0..15)
        .map(|i| format!("line{i}"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\nrating:15";
    let (stdout, stderr, exit_code) = check_query(&query);

    assert_eq!(exit_code, 1);
    assert!(stderr.is_empty());

    assert!(stdout.contains("error[E009]: Rating must be between 0 and 5"));
    assert!(stdout.contains("  --> 16:1"));
    assert!(stdout.contains("14 | line13"));
    assert!(stdout.contains("15 | line14"));
    assert!(stdout.contains("16 | rating:15"));
    assert!(stdout.contains("   | ^^^^^^^^^")); // Pipe should be aligned with double-digit line numbers
}

#[test]
fn test_json_output() {
    let (stdout, stderr, exit_code) = check_query_json("rating:15");

    assert_eq!(exit_code, 1);
    assert!(stderr.is_empty());

    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert!(json["errors"].is_array());
    assert!(json["warnings"].is_array());
    assert_eq!(json["errors"].as_array().unwrap().len(), 1);

    let error = &json["errors"][0];
    assert_eq!(error["code"], "E009");
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("Rating must be between 0 and 5")
    );
    assert_eq!(error["span"]["start"]["line"], 1);
    assert_eq!(error["span"]["start"]["column"], 1);
}

#[test]
fn test_file_based_checking() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.bwq");

    fs::write(&file_path, "rating:15")?;

    let (stdout, stderr, exit_code) = check_file(file_path.to_str().unwrap());

    assert_eq!(exit_code, 1);
    assert!(stderr.is_empty());

    // Check that file path is shown in output
    assert!(stdout.contains("error[E009]: Rating must be between 0 and 5"));
    assert!(stdout.contains(&format!("  --> {}:1:1", file_path.display())));
    assert!(stdout.contains("1 | rating:15"));
    assert!(stdout.contains("  | ^^^^^^^^^"));

    Ok(())
}

#[test]
fn test_multiple_files() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Create files with different issues
    let file1 = temp_dir.path().join("error.bwq");
    let file2 = temp_dir.path().join("warning.bwq");
    let file3 = temp_dir.path().join("valid.bwq");

    fs::write(&file1, "rating:15")?;
    fs::write(&file2, "apple this")?;
    fs::write(&file3, "apple AND juice")?;

    let (stdout, stderr, exit_code) = check_file(temp_dir.path().to_str().unwrap());

    assert_eq!(exit_code, 1); // Should fail due to error in file1
    assert!(stderr.is_empty());

    // Should contain error from file1
    assert!(stdout.contains("error[E009]: Rating must be between 0 and 5"));
    assert!(stdout.contains(&format!("  --> {}:1:1", file1.display())));

    // Should contain warning from file2
    assert!(stdout.contains("warning[W001]:"));
    assert!(stdout.contains(&format!("  --> {}:1:6", file2.display())));

    assert!(stdout.contains("Summary: "));

    Ok(())
}

#[test]
fn test_valid_query() {
    let (stdout, stderr, exit_code) = check_query("apple AND juice");

    assert_eq!(exit_code, 0);
    assert!(stderr.is_empty());
    assert_eq!(stdout.trim(), "All checks passed!");
}

#[test]
fn test_no_warnings_flag() {
    let (stdout, stderr, exit_code) = bwq_cmd()
        .args(["check", "--query", "apple this", "--no-warnings"])
        .output()
        .map(|output| {
            let stdout = strip_ansi_codes(&String::from_utf8_lossy(&output.stdout));
            let stderr = strip_ansi_codes(&String::from_utf8_lossy(&output.stderr));
            let exit_code = output.status.code().unwrap_or(-1);
            (stdout, stderr, exit_code)
        })
        .unwrap();

    assert_eq!(exit_code, 0);
    assert!(stderr.is_empty());
    assert_eq!(stdout.trim(), "All checks passed!");
}

#[test]
fn test_error_spans_complex_query() {
    let query = "(apple OR orange) AND rating:15 AND juice";
    let (stdout, stderr, exit_code) = check_query(query);

    assert_eq!(exit_code, 1);
    assert!(stderr.is_empty());

    assert!(stdout.contains("error[E009]: Rating must be between 0 and 5"));
    assert!(stdout.contains("  --> 1:23")); // Should point to start of "rating:15"
    assert!(stdout.contains("rating:15"));
    assert!(stdout.contains("^^^^^^^^^"));
}
