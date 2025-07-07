use bwq::error::{LintReport, LintWarning};
use bwq::{analyze_query, is_valid_query, BrandwatchLinter};
use std::fs;
use test_case::test_case;

/// Test context for consistent query validation testing
pub struct QueryTest {
    linter: BrandwatchLinter,
}

impl Default for QueryTest {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryTest {
    pub fn new() -> Self {
        Self {
            linter: BrandwatchLinter::new(),
        }
    }

    pub fn assert_valid(&mut self, query: &str) -> &LintReport {
        match self.linter.lint(query) {
            Ok(report) => {
                assert!(
                    !report.has_errors(),
                    "Expected query to be valid but found errors: {} - {:?}",
                    query,
                    report.errors
                );
                // return reference to the report for potential chaining
                // this is a bit tricky with borrowing, so we'll return the last report
                // for now, create a static empty report reference
                // TODO: store the report in the struct?
                assert!(is_valid_query(query), "Query should be valid: {}", query);
                &EMPTY_REPORT
            }
            Err(error) => {
                panic!(
                    "Expected query to be valid but got parse error: {} - {}",
                    query, error
                );
            }
        }
    }

    /// Assert query is valid but has warnings
    pub fn assert_valid_with_warning(&mut self, query: &str) -> &LintReport {
        match self.linter.lint(query) {
            Ok(report) => {
                assert!(
                    !report.has_errors(),
                    "Expected query to be valid but found errors: {} - {:?}",
                    query,
                    report.errors
                );
                assert!(
                    report.has_warnings(),
                    "Expected query to have warnings: {}",
                    query
                );
                &EMPTY_REPORT
            }
            Err(error) => {
                panic!(
                    "Expected query to be valid but got parse error: {} - {}",
                    query, error
                );
            }
        }
    }

    /// Assert query has error with specific code
    pub fn assert_error_code(&mut self, query: &str, expected_code: &str) {
        match self.linter.lint(query) {
            Ok(report) => {
                assert!(
                    report.has_errors(),
                    "Expected query to have errors: {}",
                    query
                );
                assert!(
                    report.errors.iter().any(|e| e.code() == expected_code),
                    "Expected error with code '{}' for query: {}, but got errors: {:?}",
                    expected_code,
                    query,
                    report.errors.iter().map(|e| e.code()).collect::<Vec<_>>()
                );
            }
            Err(error) => {
                // For parse/lex errors, check the error directly
                assert_eq!(
                    error.code(),
                    expected_code,
                    "Expected error code '{}' for query: {}, but got: {}",
                    expected_code,
                    query,
                    error.code()
                );
            }
        }
    }

    /// Assert query has warning with specific code
    pub fn assert_warning_code(&mut self, query: &str, expected_code: &str) {
        let report = self
            .linter
            .lint(query)
            .expect("Query should parse successfully");
        assert!(
            !report.warnings.is_empty(),
            "Expected query to have warnings: {}",
            query
        );
        assert!(
            report.warnings.iter().any(|w| w.code() == expected_code),
            "Expected warning with code '{}' for query: {}, but got warnings: {:?}",
            expected_code,
            query,
            report.warnings.iter().map(|w| w.code()).collect::<Vec<_>>()
        );
    }

    /// Assert query has no warnings
    pub fn assert_no_warnings(&mut self, query: &str) {
        let report = self
            .linter
            .lint(query)
            .expect("Query should parse successfully");
        assert!(
            report.warnings.is_empty(),
            "Expected no warnings for query: {}, but got: {:?}",
            query,
            report.warnings
        );
    }
}

// Static empty report for return references (simplified approach)
static EMPTY_REPORT: LintReport = LintReport {
    errors: Vec::new(),
    warnings: Vec::new(),
};

/// Test expectation for parameterized testing
#[derive(Debug, Clone)]
pub enum TestExpectation {
    /// Query should be valid with no errors or warnings
    Valid,
    /// Query should be valid but have a warning with specific code
    ValidWithWarning(&'static str),
    /// Query should have an error with specific code
    ErrorCode(&'static str),
    /// Query should have both error and warning with specific codes
    ErrorCodeWithWarning(&'static str, &'static str),
}

impl TestExpectation {
    /// Apply this expectation to a query using the test context
    pub fn assert(&self, test: &mut QueryTest, query: &str) {
        match self {
            TestExpectation::Valid => {
                test.assert_valid(query);
                test.assert_no_warnings(query);
            }
            TestExpectation::ValidWithWarning(warning_code) => {
                test.assert_valid_with_warning(query);
                test.assert_warning_code(query, warning_code);
            }
            TestExpectation::ErrorCode(error_code) => {
                test.assert_error_code(query, error_code);
            }
            TestExpectation::ErrorCodeWithWarning(error_code, warning_code) => {
                test.assert_error_code(query, error_code);
                test.assert_warning_code(query, warning_code);
            }
        }
    }
}

#[test]
fn test_basic_boolean_operators() {
    assert!(is_valid_query("apple AND juice"));
    assert!(is_valid_query("apple OR orange"));
    assert!(is_valid_query("apple NOT bitter"));
    assert!(is_valid_query("(apple OR orange) AND juice"));

    // pure negative query
    assert!(!is_valid_query("NOT bitter"));
}

#[test]
fn test_quoted_phrases() {
    assert!(is_valid_query("\"apple juice\""));
    assert!(is_valid_query("\"organic fruit\" AND healthy"));
    assert!(is_valid_query("\"multi word phrase\" OR simple"));
}

#[test]
fn test_proximity_operators() {
    assert!(is_valid_query("\"apple juice\"~5"));
    assert!(is_valid_query("apple NEAR/3 juice"));
    assert!(is_valid_query("apple NEAR/2f juice"));
    assert!(is_valid_query("\"apple juice\"~10"));
    assert!(is_valid_query(
        "((apple OR orange) NEAR/5 (smartphone OR phone))"
    ));

    // valid NEAR with proper parentheses
    assert!(is_valid_query("(apple NEAR/5 juice) AND orange"));
    assert!(is_valid_query(
        "continent:europe AND (sustainability NEAR/10 climate)"
    ));
}

#[test]
fn test_wildcards_and_replacement() {
    assert!(is_valid_query("appl*"));
    assert!(is_valid_query("customi?e"));
    assert!(is_valid_query("complain*"));

    let mut linter = BrandwatchLinter::new();
    let report = linter.lint("*invalid").unwrap();
    assert!(report.has_errors());
}

#[test]
fn test_field_operators() {
    assert!(is_valid_query("title:\"apple juice\""));
    assert!(is_valid_query("site:twitter.com"));
    assert!(is_valid_query("author:  brandwatch")); // whitespace after colon is allowed
    assert!(is_valid_query("language:en"));
    assert!(is_valid_query("country:usa"));
    assert!(is_valid_query("region:usa.ca"));
    assert!(is_valid_query("city:\"usa.ca.san francisco\""));
}

#[test]
fn test_range_operators() {
    assert!(is_valid_query("rating:[3 TO 5]"));
    assert!(is_valid_query("authorFollowers:[1000 TO 50000]"));
    assert!(is_valid_query("latitude:[41 TO 44]"));
    assert!(is_valid_query("longitude:[-73 TO -69]"));
    assert!(is_valid_query("minuteOfDay:[1110 TO 1140]"));
}

#[test]
fn test_advanced_operators() {
    assert!(is_valid_query("authorGender:F"));
    assert!(is_valid_query("authorGender:X")); // BW API accepts any gender value
    assert!(is_valid_query("authorVerified:true"));
    assert!(is_valid_query("authorVerifiedType:blue"));
    assert!(is_valid_query("engagementType:RETWEET"));
    assert!(is_valid_query("blogName:comedycentral"));
    assert!(is_valid_query("tags:photography"));
    assert!(is_valid_query("subreddit:nba"));
}

#[test]
fn test_case_sensitive_matching() {
    assert!(is_valid_query("{BrandWatch}"));
    assert!(is_valid_query("apple AND {BT}"));
}

#[test]
fn test_comments() {
    assert!(is_valid_query("apple <<<This is a comment>>> AND juice"));
    assert!(is_valid_query("<<<Brand monitoring>>> \"brand name\""));
}

#[test]
fn test_special_characters() {
    assert!(is_valid_query("#MondayMotivation"));
    assert!(is_valid_query("@brandwatch"));
    assert!(is_valid_query("#hashtag AND @mention"));
}

#[test]
fn test_complex_queries() {
    assert!(is_valid_query(
        r#"(apple OR orange) AND "fruit juice" NOT bitter AND site:twitter.com"#
    ));

    // Mixed NEAR/AND requires parentheses
    assert!(!is_valid_query(
        r#"title:"smartphone review" AND (iPhone OR Samsung) NEAR/5 (camera OR battery)"#
    ));

    // Properly parenthesized NEAR/AND should work
    assert!(is_valid_query(
        r#"title:"smartphone review" AND ((iPhone OR Samsung) NEAR/5 (camera OR battery))"#
    ));

    assert!(is_valid_query(
        r#"authorFollowers:[1000 TO 100000] AND engagementType:RETWEET AND language:en"#
    ));

    assert!(is_valid_query(
        r#"("brand name" OR @brandhandle) AND sentiment:positive NOT complaint*"#
    ));
}

#[test]
fn test_invalid_queries() {
    let invalid_queries = vec![
        "*invalid",                 // Wildcard at beginning
        "apple AND",                // Missing right operand
        "OR juice",                 // Missing left operand
        "apple AND ()",             // Empty parentheses
        "rating:6", // Invalid rating (should be 0-5) - NOTE: BW API is more permissive
        "authorFollowers:[3 TO 1]", // Invalid range (start > end)
        "NOT bitter", // Pure negative query (NOT is binary)
    ];

    for query in invalid_queries {
        let analysis = analyze_query(query);
        assert!(!analysis.is_valid, "Query should be invalid: {}", query);
    }
}

#[test]
fn test_validation_warnings() {
    let mut test = QueryTest::new();
    test.assert_warning_code("ab*", "W003");
    test.assert_warning_code("authorFollowers:[1 TO 2000000000]", "W003");
    test.assert_error_code("languag:e", "E012");
}

#[test]
fn test_json_output_validation() {
    let analysis = analyze_query("rating:6 AND *invalid");

    assert_eq!(analysis.errors.len(), 2);

    let error_codes: Vec<&str> = analysis.errors.iter().map(|e| e.code()).collect();
    assert!(error_codes.contains(&"E012")); // Rating validation error
    assert!(error_codes.contains(&"E006")); // Wildcard placement error

    // test JSON serialization includes codes
    let json_query = r#"rating:6 AND ab*"#.to_string();
    let analysis = analyze_query(&json_query);
    assert!(!analysis.errors.is_empty()); // Rating error
    assert!(!analysis.warnings.is_empty()); // Performance warning

    assert!(analysis.errors.iter().any(|e| e.code() == "E012"));
    assert!(analysis.warnings.iter().any(|w| w.code() == "W003"));
}

#[test_case("rating:3", TestExpectation::Valid; "valid rating 3")]
#[test_case("rating:0", TestExpectation::Valid; "valid rating 0")]
#[test_case("rating:[2 TO 4]", TestExpectation::Valid; "valid rating range")]
#[test_case("rating:6", TestExpectation::ErrorCode("E012"); "rating too high")]
#[test_case("rating:[-1 TO 3]", TestExpectation::ErrorCode("E012"); "rating range with negative")]
fn test_rating_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("latitude:[40 TO 42]", TestExpectation::Valid; "valid latitude range")]
#[test_case("longitude:[-73 TO -69]", TestExpectation::Valid; "valid longitude range")]
#[test_case("continent:europe", TestExpectation::Valid; "valid continent")]
#[test_case("latitude:[100 TO 110]", TestExpectation::ErrorCode("E012"); "latitude out of range")]
#[test_case("longitude:[-200 TO -150]", TestExpectation::ErrorCode("E012"); "longitude out of range")]
fn test_location_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("authorVerified:true", TestExpectation::Valid; "valid boolean true")]
#[test_case("authorVerified:false", TestExpectation::Valid; "valid boolean false")]
#[test_case("authorVerified:yes", TestExpectation::ErrorCode("E012"); "invalid boolean yes")]
#[test_case("authorVerified:1", TestExpectation::ErrorCode("E012"); "invalid boolean number")]
fn test_boolean_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("language:en", TestExpectation::Valid; "valid 2-char language code")]
#[test_case("language:fr", TestExpectation::Valid; "valid french language code")]
#[test_case("language:es", TestExpectation::Valid; "valid spanish language code")]
#[test_case("language:ENG", TestExpectation::ValidWithWarning("W001"); "uppercase language code warning")]
#[test_case("language:english", TestExpectation::ValidWithWarning("W001"); "full language name warning")]
fn test_language_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("*invalid", TestExpectation::ErrorCode("E006"); "wildcard at start")]
#[test_case("ab*", TestExpectation::ValidWithWarning("W003"); "short wildcard performance warning")]
#[test_case("test*", TestExpectation::Valid; "normal wildcard")]
fn test_wildcard_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("engagementType:COMMENT", TestExpectation::Valid; "valid engagement comment")]
#[test_case("engagementType:REPLY", TestExpectation::Valid; "valid engagement reply")]
#[test_case("engagementType:RETWEET", TestExpectation::Valid; "valid engagement retweet")]
#[test_case("engagementType:QUOTE", TestExpectation::Valid; "valid engagement quote")]
#[test_case("engagementType:LIKE", TestExpectation::Valid; "valid engagement like")]
fn test_engagement_type_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("authorVerifiedType:blue", TestExpectation::Valid; "valid verified type blue")]
#[test_case("authorVerifiedType:business", TestExpectation::Valid; "valid verified type business")]
#[test_case("authorVerifiedType:government", TestExpectation::Valid; "valid verified type government")]
#[test_case("authorVerifiedType:gold", TestExpectation::ErrorCode("E012"); "invalid verified type gold")]
fn test_verified_type_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("minuteOfDay:[0 TO 1439]", TestExpectation::Valid; "valid minute of day full range")]
#[test_case("minuteOfDay:[720 TO 780]", TestExpectation::Valid; "valid minute of day noon to 1pm")]
#[test_case("minuteOfDay:[-1 TO 100]", TestExpectation::ErrorCode("E012"); "minute of day with negative")]
#[test_case("minuteOfDay:[0 TO 1440]", TestExpectation::ErrorCode("E012"); "minute of day over max")]
fn test_minute_of_day_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("country:gbr", TestExpectation::Valid; "valid country code")]
#[test_case("region:usa.fl", TestExpectation::Valid; "valid region code")]
#[test_case("city:\"deu.berlin.berlin\"", TestExpectation::Valid; "valid city code")]
fn test_additional_location_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test]
fn test_analysis_result_formatting() {
    let analysis = analyze_query("apple AND juice");
    assert_eq!(analysis.summary(), "Query is valid with no issues");
    assert!(!analysis.has_issues());

    let analysis = analyze_query("*invalid");
    assert!(analysis.summary().contains("error"));
    assert!(analysis.has_issues());

    let formatted_issues = analysis.format_issues();
    assert!(formatted_issues.contains("Errors:"));
}

#[test]
fn test_performance_edge_cases() {
    let mut linter = BrandwatchLinter::new();
    let report = linter.lint("apple NEAR/150 juice").unwrap();
    assert!(report.warnings.is_empty());

    let mut test = QueryTest::new();
    test.assert_warning_code("apple* OR juice*", "W003");

    let report = linter.lint("a").unwrap();
    assert!(report.warnings.is_empty());
}

#[test]
fn test_wildcard_position_validation() {
    assert!(is_valid_query("tes*t"));
    let mut linter = BrandwatchLinter::new();
    let report = linter.lint("tes*t").unwrap();
    assert!(report.warnings.is_empty());

    assert!(is_valid_query("#test*"));
    let report = linter.lint("#test*").unwrap();
    assert!(report.warnings.is_empty());

    assert!(is_valid_query("#*test"));
    let mut test = QueryTest::new();
    test.assert_warning_code("#*test", "W003");
}

#[test]
fn test_empty_and_whitespace_queries() {
    assert!(!is_valid_query(""));
    assert!(!is_valid_query("   "));
    assert!(!is_valid_query("\n\t"));
}

#[test]
fn test_implicit_and_behavior() {
    let mut linter = BrandwatchLinter::new();

    let report = linter.lint("apple banana").unwrap();
    assert!(!report.has_errors(), "Implicit AND should be valid");
    assert!(
        report.has_warnings(),
        "Implicit AND should generate warnings"
    );

    let report = linter.lint("apple banana OR cherry").unwrap();
    assert!(
        report.has_errors(),
        "Mixed implicit AND with OR should fail without parentheses"
    );

    let report = linter.lint("(apple banana) OR cherry").unwrap();
    assert!(
        !report.has_errors(),
        "Properly parenthesized implicit AND should be valid"
    );
    assert!(
        report.has_warnings(),
        "Implicit AND should still generate warnings"
    );

    let report = linter.lint("apple AND banana").unwrap();
    assert!(
        !report.has_warnings(),
        "Explicit AND should not generate warnings"
    );

    // Test case sensitivity - lowercase operators
    let report = linter.lint("apple and juice").unwrap();
    assert!(
        !report.has_errors(),
        "Lowercase 'and' should be treated as implicit AND"
    );
    assert!(
        report.has_warnings(),
        "Should warn about implicit AND usage"
    );
}

#[test]
fn test_operators_on_groupings() {
    assert!(is_valid_query("((smartphone OR phone) NEAR/5 (review OR rating)) AND ((camera OR battery) NEAR/3 (excellent OR amazing))"));

    // Tilde proximity with AND should work without parentheses
    assert!(is_valid_query("\"apple juice\"~5 AND (organic OR natural)"));
    assert!(is_valid_query("(apple AND juice)~2 AND test"));

    assert!(is_valid_query("juice NOT (apple AND (bitter OR sour))"));

    assert!(is_valid_query(
        "((brand OR company) NEAR/2f (announcement OR news)) AND (exciting OR important)"
    ));

    // Boolean operators on complex proximity groups
    assert!(is_valid_query(
        "((apple NEAR/2 juice) OR (orange NEAR/3 smoothie)) AND fresh"
    ));
    assert!(is_valid_query(
        "((complain* NEAR/5 product*) NOT (resolve* NEAR/3 solution*)) AND site:twitter.com"
    ));
}

#[test]
fn test_complex_field_operator_combinations() {
    // Multiple field operators with groupings
    assert!(is_valid_query("((country:usa OR country:gbr) AND (language:en OR language:es)) AND ((authorGender:F AND authorVerified:true) OR authorFollowers:[10000 TO 100000])"));

    // Location fields with complex logic
    assert!(is_valid_query("((continent:europe AND country:gbr) OR (continent:north_america AND country:usa)) AND ((city:\"new york\" OR city:london) AND language:en)"));

    // Time and engagement combinations
    assert!(is_valid_query("((minuteOfDay:[480 TO 720] OR minuteOfDay:[1080 TO 1320]) AND (engagementType:RETWEET OR engagementType:QUOTE)) AND ((authorFollowers:[1000 TO 50000] AND authorVerified:true) OR rating:[4 TO 5])"));

    // Complex Reddit-specific combinations
    assert!(is_valid_query("((subreddit:technology OR subreddit:programming) AND (redditAuthorFlair:developer OR redditAuthorFlair:engineer)) AND ((redditspoiler:false AND subredditNSFW:false) OR authorVerified:true)"));
}

#[test]
fn test_comment_integration_in_complex_queries() {
    // Comments in deeply nested queries
    assert!(is_valid_query("apple <<<fruit category>>> AND ((juice <<<beverage>>> OR smoothie <<<drink>>>)) NOT <<<exclude>>> bitter"));

    // Multiple comments in complex structure
    assert!(is_valid_query("((brand <<<company>>> AND product <<<item>>>) OR (service <<<offering>>> AND quality <<<standard>>>)) AND <<<monitoring>>> positive"));
}

#[test]
fn test_hashtag_mention_complex_combinations() {
    // Hashtags and mentions in complex boolean logic
    assert!(is_valid_query("(((#MondayMotivation OR #InspirationalQuote) AND (@company OR @brand)) AND ((positive OR inspiring) NOT (spam OR promotional))) AND ((site:twitter.com OR site:instagram.com) AND language:en)"));

    // Mixed social signals
    assert!(is_valid_query("((#technology AND #innovation) OR (@techcompany AND @startup)) AND ((breakthrough OR revolutionary) NEAR/5 (product OR service))"));
}

#[test]
fn test_performance_warnings_in_complex_queries() {
    let mut linter = BrandwatchLinter::new();
    let report = linter
        .lint("((ab* OR bc*) AND (cd* OR de*)) AND ((e NEAR/200 f) OR (g NEAR/150 h))")
        .unwrap();
    assert!(!report.has_errors());
    assert!(
        !report.warnings.is_empty(),
        "Should have performance warnings"
    );

    let report = linter
        .lint("((a OR b) AND (c OR d)) AND ((e NEAR/5 f) OR (g AND h))")
        .unwrap();
    assert!(!report.has_errors());
    assert!(report.warnings.is_empty(), "Should have no warnings");
}

#[test]
fn test_operator_precedence_validation() {
    let mut test = QueryTest::new();

    let mixed_and_or_cases = vec![
        "apple OR banana AND juice",
        "apple AND banana OR juice AND smoothie",
        "apple NOT bitter AND sweet OR sour",
    ];

    for query in mixed_and_or_cases {
        test.assert_error_code(query, "E015");
    }

    let properly_parenthesized_cases = vec![
        "(apple OR banana) AND juice",
        "(apple AND banana) OR (juice AND smoothie)",
        "apple NOT (bitter AND sweet) OR sour",
    ];

    for query in properly_parenthesized_cases {
        test.assert_valid(query);
        test.assert_no_warnings(query);
    }
}

#[test]
fn test_tilde_proximity_syntax() {
    // Valid tilde usage - standard cases
    assert!(is_valid_query("\"apple juice\"~5"));
    assert!(is_valid_query("\"organic fruit\"~10"));
    assert!(is_valid_query("((apple OR orange) AND phone)~5"));
    assert!(is_valid_query("(brand OR company)~3"));
    assert!(is_valid_query("((tech OR technology) AND innovation)~7"));

    // Valid tilde with boolean operations (no parentheses needed)
    assert!(is_valid_query("\"apple juice\"~5 AND test"));
    assert!(is_valid_query("(apple AND juice)~2 AND test"));

    let mut linter = BrandwatchLinter::new();

    // Valid but with warnings - single term usage
    let report = linter.lint("apple~5").unwrap();
    assert!(!report.has_errors());
    assert!(report.has_warnings());
    assert!(report.warnings.iter().any(|w| match w {
        LintWarning::PotentialTypo { suggestion, .. } =>
            suggestion.contains("Single term tilde may produce unexpected fuzzy matching"),
        _ => false,
    }));

    // Valid but with warnings - single quoted word
    let report = linter.lint("\"apple\"~5").unwrap();
    assert!(!report.has_errors());
    assert!(report.has_warnings());
    assert!(report.warnings.iter().any(|w| match w {
        LintWarning::PotentialTypo { suggestion, .. } => suggestion.contains("no effect"),
        _ => false,
    }));

    // Valid with implicit AND and warnings
    let report = linter.lint("apple~5 juice").unwrap();
    assert!(!report.has_errors());
    assert!(report.has_warnings());
    assert!(report.warnings.iter().any(|w| match w {
        LintWarning::PotentialTypo { suggestion, .. } => suggestion.contains("Single term tilde"),
        _ => false,
    }));
    assert!(report.warnings.iter().any(|w| match w {
        LintWarning::PotentialTypo { suggestion, .. } =>
            suggestion.contains("explicit 'AND' operator"),
        _ => false,
    }));

    // Invalid: tilde without distance number
    let mut test = QueryTest::new();
    test.assert_error_code("\"apple juice\"~", "E003");
    test.assert_error_code("apple~", "E003");

    // Invalid: tilde with spaces (separate tokens)
    test.assert_error_code("apple ~ juice", "E003");

    // Space between tilde and number
    test.assert_error_code("apple~ 5", "E003");

    // Space before tilde
    test.assert_error_code("apple ~5", "E003");

    // Invalid characters after number (lexer error)
    test.assert_error_code("apple~5t", "E001");
}

#[test]
fn test_near_operator_interaction_validation() {
    let mut test = QueryTest::new();

    let mixed_near_boolean_cases = vec!["(apple OR orange) NEAR/5 (juice OR drink) AND fresh"];

    for query in mixed_near_boolean_cases {
        test.assert_error_code(query, "E013");
    }

    let valid_near_cases = vec![
        "((apple OR orange) NEAR/5 (juice OR drink)) AND fresh", // Proper parentheses
        "(apple NEAR/3 banana) OR (juice NEAR/2 smoothie)",      // Properly parenthesized NEAR/OR
        "(apple NEAR/5 juice) AND (banana NEAR/3 smoothie)",     // Separate NEAR operations
    ];

    for query in valid_near_cases {
        test.assert_valid(query);
        test.assert_no_warnings(query);
    }
}

#[test]
fn test_bq_file_fixtures() {
    let valid_multiline = fs::read_to_string("tests/fixtures/valid_multiline.bwq").unwrap();
    assert!(
        is_valid_query(&valid_multiline),
        "Multi-line query should be valid"
    );

    let complex_near = fs::read_to_string("tests/fixtures/complex_near.bwq").unwrap();
    assert!(
        is_valid_query(&complex_near),
        "Complex NEAR query should be valid"
    );

    let field_operations = fs::read_to_string("tests/fixtures/field_operations.bwq").unwrap();
    assert!(
        is_valid_query(&field_operations),
        "Field operations query should be valid"
    );

    let comments_and_wildcards =
        fs::read_to_string("tests/fixtures/comments_and_wildcards.bwq").unwrap();
    assert!(
        is_valid_query(&comments_and_wildcards),
        "Comments and wildcards query should be valid"
    );

    let invalid_mixed = fs::read_to_string("tests/fixtures/invalid_mixed_operators.bwq").unwrap();
    assert!(
        !is_valid_query(&invalid_mixed),
        "Mixed operators without parentheses should be invalid"
    );
}

#[test]
fn test_bq_file_analysis() {
    let complex_near = fs::read_to_string("tests/fixtures/complex_near.bwq").unwrap();
    let analysis = analyze_query(&complex_near);
    assert!(analysis.is_valid);
    assert!(!analysis.has_issues());

    let invalid_mixed = fs::read_to_string("tests/fixtures/invalid_mixed_operators.bwq").unwrap();
    let analysis = analyze_query(&invalid_mixed);
    assert!(!analysis.is_valid);
    assert!(analysis.has_issues());
    assert!(!analysis.errors.is_empty());
}
