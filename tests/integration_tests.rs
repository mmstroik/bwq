use bwq::error::LintReport;
use bwq::BrandwatchLinter;
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
                &EMPTY_REPORT
            }
            Err(error) => {
                panic!("Expected query to be valid but got parse error: {query} - {error}");
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
                    "Expected query to have warnings: {query}"
                );
                &EMPTY_REPORT
            }
            Err(error) => {
                panic!("Expected query to be valid but got parse error: {query} - {error}");
            }
        }
    }

    /// Assert query has error with specific code
    pub fn assert_error_code(&mut self, query: &str, expected_code: &str) {
        match self.linter.lint(query) {
            Ok(report) => {
                assert!(
                    report.has_errors(),
                    "Expected query to have errors: {query}"
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
            "Expected query to have warnings: {query}"
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

    /// Assert query is valid with no errors AND no warnings (common pattern)
    pub fn assert_valid_no_warnings(&mut self, query: &str) {
        self.assert_valid(query);
        self.assert_no_warnings(query);
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
    /// Query should be valid with no errors AND no warnings (clearer intent)
    ValidNoWarnings,
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
                // Note: Valid allows warnings, use ValidNoWarnings if you want to enforce no warnings
            }
            TestExpectation::ValidNoWarnings => {
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

#[test_case("apple AND juice", TestExpectation::ValidNoWarnings; "basic AND operation")]
#[test_case("apple OR orange", TestExpectation::ValidNoWarnings; "basic OR operation")]
#[test_case("apple NOT bitter", TestExpectation::ValidNoWarnings; "basic NOT operation")]
#[test_case("(apple OR orange) AND juice", TestExpectation::ValidNoWarnings; "parenthesized boolean operations")]
#[test_case("NOT bitter", TestExpectation::ErrorCode("E016"); "pure negative query error")]
fn test_basic_boolean_syntax(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("\"apple juice\"", TestExpectation::ValidNoWarnings; "basic quoted phrase")]
#[test_case("\"organic fruit\" AND healthy", TestExpectation::ValidNoWarnings; "quoted phrase with AND")]
#[test_case("\"multi word phrase\" OR simple", TestExpectation::ValidNoWarnings; "quoted phrase with OR")]
fn test_quoted_phrase_syntax(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("\"apple juice\"~5", TestExpectation::ValidNoWarnings; "basic tilde proximity")]
#[test_case("apple NEAR/3 juice", TestExpectation::ValidNoWarnings; "basic NEAR operator")]
#[test_case("apple NEAR/2f juice", TestExpectation::ValidNoWarnings; "NEAR with fuzzy flag")]
#[test_case("\"apple juice\"~10", TestExpectation::ValidNoWarnings; "tilde with larger distance")]
#[test_case("((apple OR orange) NEAR/5 (smartphone OR phone))", TestExpectation::ValidNoWarnings; "NEAR with grouped terms")]
#[test_case("(apple NEAR/5 juice) AND orange", TestExpectation::ValidNoWarnings; "NEAR with boolean AND")]
#[test_case("continent:europe AND (sustainability NEAR/10 climate)", TestExpectation::ValidNoWarnings; "field with NEAR operation")]
fn test_proximity_operator_syntax(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("appl*", TestExpectation::ValidNoWarnings; "asterisk wildcard at end")]
#[test_case("customi?e", TestExpectation::ValidNoWarnings; "question mark wildcard in middle")]
#[test_case("complain*", TestExpectation::ValidNoWarnings; "asterisk wildcard normal usage")]
#[test_case("*invalid", TestExpectation::ErrorCode("E006"); "invalid wildcard at beginning")]
fn test_wildcard_syntax(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test]
fn test_field_operators() {
    let mut test = QueryTest::new();

    test.assert_valid_no_warnings("title:\"apple juice\"");

    test.assert_valid_no_warnings("site:twitter.com");

    test.assert_valid_no_warnings("author:  brandwatch"); // whitespace after colon is allowed

    test.assert_valid_no_warnings("language:en");

    test.assert_valid_no_warnings("country:usa");

    test.assert_valid_no_warnings("region:usa.ca");

    test.assert_valid_no_warnings("city:\"usa.ca.san francisco\"");
}

#[test]
fn test_range_operators() {
    let mut test = QueryTest::new();

    test.assert_valid_no_warnings("rating:[3 TO 5]");

    test.assert_valid_no_warnings("authorFollowers:[1000 TO 50000]");

    test.assert_valid_no_warnings("latitude:[41 TO 44]");

    test.assert_valid_no_warnings("longitude:[-73 TO -69]");

    test.assert_valid_no_warnings("minuteOfDay:[1110 TO 1140]");
}

#[test]
fn test_advanced_operators() {
    let mut test = QueryTest::new();

    test.assert_valid_no_warnings("authorGender:F");

    test.assert_valid_no_warnings("authorGender:X"); // BW API accepts any gender value

    test.assert_valid_no_warnings("authorVerified:true");

    test.assert_valid_no_warnings("authorVerifiedType:blue");

    test.assert_valid_no_warnings("engagementType:RETWEET");

    test.assert_valid_no_warnings("blogName:comedycentral");

    test.assert_valid_no_warnings("tags:photography");

    test.assert_valid_no_warnings("subreddit:nba");
}

#[test]
fn test_case_sensitive_matching() {
    let mut test = QueryTest::new();

    test.assert_valid_no_warnings("{BrandWatch}");

    test.assert_valid_no_warnings("apple AND {BT}");
}

#[test]
fn test_comments() {
    let mut test = QueryTest::new();

    test.assert_valid_no_warnings("apple <<<This is a comment>>> AND juice");

    test.assert_valid_no_warnings("<<<Brand monitoring>>> \"brand name\"");
}

#[test_case("#MondayMotivation", TestExpectation::ValidNoWarnings; "hashtag syntax")]
#[test_case("@brandwatch", TestExpectation::ValidNoWarnings; "mention syntax")]
#[test_case("#hashtag AND @mention", TestExpectation::ValidNoWarnings; "hashtag and mention combined")]
fn test_special_character_syntax(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test]
fn test_complex_queries() {
    let mut test = QueryTest::new();

    test.assert_valid_no_warnings(r#"(apple OR orange) AND "fruit juice" NOT bitter AND site:twitter.com"#);

    // Mixed NEAR/AND requires parentheses
    test.assert_error_code(
        r#"title:"smartphone review" AND (iPhone OR Samsung) NEAR/5 (camera OR battery)"#,
        "E013",
    );

    // Properly parenthesized NEAR/AND should work
    test.assert_valid_no_warnings(r#"title:"smartphone review" AND ((iPhone OR Samsung) NEAR/5 (camera OR battery))"#);

    test.assert_valid_no_warnings(r#"authorFollowers:[1000 TO 100000] AND engagementType:RETWEET AND language:en"#);

    test.assert_valid_no_warnings(r#"("brand name" OR @brandhandle) AND sentiment:positive NOT complaint*"#);
}

// Common invalid query patterns
#[test_case("*invalid", TestExpectation::ErrorCode("E006"); "wildcard at beginning")]
#[test_case("apple AND", TestExpectation::ErrorCode("E010"); "missing right operand")]
#[test_case("OR juice", TestExpectation::ErrorCode("E010"); "missing left operand")]
#[test_case("apple AND ()", TestExpectation::ErrorCode("E010"); "empty parentheses")]
#[test_case("rating:6", TestExpectation::ErrorCode("E012"); "invalid rating value")]
#[test_case("authorFollowers:[3 TO 1]", TestExpectation::ErrorCode("E014"); "invalid range order")]
#[test_case("NOT bitter", TestExpectation::ErrorCode("E016"); "pure negative query")]
fn test_invalid_query_patterns(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
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
    let mut test = QueryTest::new();

    // Test query with multiple errors
    test.assert_error_code("rating:6 AND *invalid", "E012"); // Rating validation error
    test.assert_error_code("rating:6 AND *invalid", "E006"); // Wildcard placement error

    // Test query with error and warning
    test.assert_error_code("rating:6 AND ab*", "E012"); // Rating validation error
    test.assert_warning_code("rating:6 AND ab*", "W003"); // Performance warning
}

#[test_case("rating:3", TestExpectation::ValidNoWarnings; "valid rating 3")]
#[test_case("rating:0", TestExpectation::ValidNoWarnings; "valid rating 0")]
#[test_case("rating:[2 TO 4]", TestExpectation::ValidNoWarnings; "valid rating range")]
#[test_case("rating:6", TestExpectation::ErrorCode("E012"); "rating too high")]
#[test_case("rating:[-1 TO 3]", TestExpectation::ErrorCode("E012"); "rating range with negative")]
fn test_rating_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("latitude:[40 TO 42]", TestExpectation::ValidNoWarnings; "valid latitude range")]
#[test_case("longitude:[-73 TO -69]", TestExpectation::ValidNoWarnings; "valid longitude range")]
#[test_case("continent:europe", TestExpectation::ValidNoWarnings; "valid continent")]
#[test_case("latitude:[100 TO 110]", TestExpectation::ErrorCode("E012"); "latitude out of range")]
#[test_case("longitude:[-200 TO -150]", TestExpectation::ErrorCode("E012"); "longitude out of range")]
fn test_location_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("authorVerified:true", TestExpectation::ValidNoWarnings; "valid boolean true")]
#[test_case("authorVerified:false", TestExpectation::ValidNoWarnings; "valid boolean false")]
#[test_case("authorVerified:yes", TestExpectation::ErrorCode("E012"); "invalid boolean yes")]
#[test_case("authorVerified:1", TestExpectation::ErrorCode("E012"); "invalid boolean number")]
fn test_boolean_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("language:en", TestExpectation::ValidNoWarnings; "valid 2-char language code")]
#[test_case("language:fr", TestExpectation::ValidNoWarnings; "valid french language code")]
#[test_case("language:es", TestExpectation::ValidNoWarnings; "valid spanish language code")]
#[test_case("language:ENG", TestExpectation::ValidWithWarning("W001"); "uppercase language code warning")]
#[test_case("language:english", TestExpectation::ValidWithWarning("W001"); "full language name warning")]
fn test_language_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("*invalid", TestExpectation::ErrorCode("E006"); "wildcard at start")]
#[test_case("ab*", TestExpectation::ValidWithWarning("W003"); "short wildcard performance warning")]
#[test_case("test*", TestExpectation::ValidNoWarnings; "normal wildcard")]
fn test_wildcard_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("engagementType:COMMENT", TestExpectation::ValidNoWarnings; "valid engagement comment")]
#[test_case("engagementType:REPLY", TestExpectation::ValidNoWarnings; "valid engagement reply")]
#[test_case("engagementType:RETWEET", TestExpectation::ValidNoWarnings; "valid engagement retweet")]
#[test_case("engagementType:QUOTE", TestExpectation::ValidNoWarnings; "valid engagement quote")]
#[test_case("engagementType:LIKE", TestExpectation::ValidNoWarnings; "valid engagement like")]
fn test_engagement_type_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("authorVerifiedType:blue", TestExpectation::ValidNoWarnings; "valid verified type blue")]
#[test_case("authorVerifiedType:business", TestExpectation::ValidNoWarnings; "valid verified type business")]
#[test_case("authorVerifiedType:government", TestExpectation::ValidNoWarnings; "valid verified type government")]
#[test_case("authorVerifiedType:gold", TestExpectation::ErrorCode("E012"); "invalid verified type gold")]
fn test_verified_type_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("minuteOfDay:[0 TO 1439]", TestExpectation::ValidNoWarnings; "valid minute of day full range")]
#[test_case("minuteOfDay:[720 TO 780]", TestExpectation::ValidNoWarnings; "valid minute of day noon to 1pm")]
#[test_case("minuteOfDay:[-1 TO 100]", TestExpectation::ErrorCode("E012"); "minute of day with negative")]
#[test_case("minuteOfDay:[0 TO 1440]", TestExpectation::ErrorCode("E012"); "minute of day over max")]
fn test_minute_of_day_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("country:gbr", TestExpectation::ValidNoWarnings; "valid country code")]
#[test_case("region:usa.fl", TestExpectation::ValidNoWarnings; "valid region code")]
#[test_case("city:\"deu.berlin.berlin\"", TestExpectation::ValidNoWarnings; "valid city code")]
fn test_additional_location_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test]
fn test_analysis_result_formatting() {
    let mut test = QueryTest::new();

    // Valid query with no issues
    test.assert_valid_no_warnings("apple AND juice");

    // Invalid query with errors
    test.assert_error_code("*invalid", "E006");
}

#[test]
fn test_performance_edge_cases() {
    let mut test = QueryTest::new();

    // NEAR with large distance should not generate warnings
    test.assert_valid_no_warnings("apple NEAR/150 juice");

    // Multiple wildcards should generate performance warnings
    test.assert_warning_code("apple* OR juice*", "W003");

    // Single character should not generate warnings
    test.assert_valid_no_warnings("a");
}

#[test]
fn test_wildcard_position_validation() {
    let mut test = QueryTest::new();

    // Wildcard in middle is valid with no warnings
    test.assert_valid_no_warnings("tes*t");

    // Wildcard at end with hashtag prefix is valid with no warnings
    test.assert_valid_no_warnings("#test*");

    // Wildcard at beginning with hashtag prefix generates performance warning
    test.assert_valid("#*test");
    test.assert_warning_code("#*test", "W003");
}

#[test_case("", TestExpectation::ErrorCode("E010"); "empty query")]
#[test_case("   ", TestExpectation::ErrorCode("E010"); "whitespace only query")]
#[test_case("\n\t", TestExpectation::ErrorCode("E010"); "newline and tab only")]
fn test_empty_and_whitespace_queries(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test]
fn test_implicit_and_behavior() {
    let mut test = QueryTest::new();

    // Implicit AND should be valid but generate warnings
    test.assert_valid("apple banana");
    test.assert_warning_code("apple banana", "W001");

    // Mixed implicit AND with OR should fail without parentheses
    test.assert_error_code("apple banana OR cherry", "E015");

    // Properly parenthesized implicit AND should be valid
    test.assert_valid("(apple banana) OR cherry");
    test.assert_warning_code("(apple banana) OR cherry", "W001");

    // Explicit AND should not generate warnings
    test.assert_valid_no_warnings("apple AND banana");

    // Test case sensitivity - lowercase operators
    test.assert_valid("apple and juice");
    test.assert_warning_code("apple and juice", "W001");
}

#[test]
fn test_operators_on_groupings() {
    let mut test = QueryTest::new();

    test.assert_valid("((smartphone OR phone) NEAR/5 (review OR rating)) AND ((camera OR battery) NEAR/3 (excellent OR amazing))");
    test.assert_no_warnings("((smartphone OR phone) NEAR/5 (review OR rating)) AND ((camera OR battery) NEAR/3 (excellent OR amazing))");

    // Tilde proximity with AND should work without parentheses
    test.assert_valid("\"apple juice\"~5 AND (organic OR natural)");
    test.assert_no_warnings("\"apple juice\"~5 AND (organic OR natural)");

    test.assert_valid_no_warnings("(apple AND juice)~2 AND test");

    test.assert_valid("juice NOT (apple AND (bitter OR sour))");
    test.assert_no_warnings("juice NOT (apple AND (bitter OR sour))");

    test.assert_valid(
        "((brand OR company) NEAR/2f (announcement OR news)) AND (exciting OR important)",
    );
    test.assert_no_warnings(
        "((brand OR company) NEAR/2f (announcement OR news)) AND (exciting OR important)",
    );

    // Boolean operators on complex proximity groups
    test.assert_valid("((apple NEAR/2 juice) OR (orange NEAR/3 smoothie)) AND fresh");
    test.assert_no_warnings("((apple NEAR/2 juice) OR (orange NEAR/3 smoothie)) AND fresh");

    test.assert_valid(
        "((complain* NEAR/5 product*) NOT (resolve* NEAR/3 solution*)) AND site:twitter.com",
    );
    test.assert_no_warnings(
        "((complain* NEAR/5 product*) NOT (resolve* NEAR/3 solution*)) AND site:twitter.com",
    );
}

#[test]
fn test_complex_field_operator_combinations() {
    let mut test = QueryTest::new();

    // Multiple field operators with groupings
    test.assert_valid("((country:usa OR country:gbr) AND (language:en OR language:es)) AND ((authorGender:F AND authorVerified:true) OR authorFollowers:[10000 TO 100000])");
    test.assert_no_warnings("((country:usa OR country:gbr) AND (language:en OR language:es)) AND ((authorGender:F AND authorVerified:true) OR authorFollowers:[10000 TO 100000])");

    // Location fields with complex logic
    test.assert_valid("((continent:europe AND country:gbr) OR (continent:north_america AND country:usa)) AND ((city:\"new york\" OR city:london) AND language:en)");
    test.assert_no_warnings("((continent:europe AND country:gbr) OR (continent:north_america AND country:usa)) AND ((city:\"new york\" OR city:london) AND language:en)");

    // Time and engagement combinations
    test.assert_valid("((minuteOfDay:[480 TO 720] OR minuteOfDay:[1080 TO 1320]) AND (engagementType:RETWEET OR engagementType:QUOTE)) AND ((authorFollowers:[1000 TO 50000] AND authorVerified:true) OR rating:[4 TO 5])");
    test.assert_no_warnings("((minuteOfDay:[480 TO 720] OR minuteOfDay:[1080 TO 1320]) AND (engagementType:RETWEET OR engagementType:QUOTE)) AND ((authorFollowers:[1000 TO 50000] AND authorVerified:true) OR rating:[4 TO 5])");

    // Complex Reddit-specific combinations
    test.assert_valid("((subreddit:technology OR subreddit:programming) AND (redditAuthorFlair:developer OR redditAuthorFlair:engineer)) AND ((redditspoiler:false AND subredditNSFW:false) OR authorVerified:true)");
    test.assert_no_warnings("((subreddit:technology OR subreddit:programming) AND (redditAuthorFlair:developer OR redditAuthorFlair:engineer)) AND ((redditspoiler:false AND subredditNSFW:false) OR authorVerified:true)");
}

#[test]
fn test_comment_integration_in_complex_queries() {
    let mut test = QueryTest::new();

    // Comments in deeply nested queries
    test.assert_valid("apple <<<fruit category>>> AND ((juice <<<beverage>>> OR smoothie <<<drink>>>)) NOT <<<exclude>>> bitter");
    test.assert_no_warnings("apple <<<fruit category>>> AND ((juice <<<beverage>>> OR smoothie <<<drink>>>)) NOT <<<exclude>>> bitter");

    // Multiple comments in complex structure
    test.assert_valid("((brand <<<company>>> AND product <<<item>>>) OR (service <<<offering>>> AND quality <<<standard>>>)) AND <<<monitoring>>> positive");
    test.assert_no_warnings("((brand <<<company>>> AND product <<<item>>>) OR (service <<<offering>>> AND quality <<<standard>>>)) AND <<<monitoring>>> positive");
}

#[test]
fn test_hashtag_mention_complex_combinations() {
    let mut test = QueryTest::new();

    // Hashtags and mentions in complex boolean logic
    test.assert_valid("(((#MondayMotivation OR #InspirationalQuote) AND (@company OR @brand)) AND ((positive OR inspiring) NOT (spam OR promotional))) AND ((site:twitter.com OR site:instagram.com) AND language:en)");
    test.assert_no_warnings("(((#MondayMotivation OR #InspirationalQuote) AND (@company OR @brand)) AND ((positive OR inspiring) NOT (spam OR promotional))) AND ((site:twitter.com OR site:instagram.com) AND language:en)");

    // Mixed social signals
    test.assert_valid("((#technology AND #innovation) OR (@techcompany AND @startup)) AND ((breakthrough OR revolutionary) NEAR/5 (product OR service))");
    test.assert_no_warnings("((#technology AND #innovation) OR (@techcompany AND @startup)) AND ((breakthrough OR revolutionary) NEAR/5 (product OR service))");
}

#[test]
fn test_performance_warnings_in_complex_queries() {
    let mut test = QueryTest::new();

    // Complex query with multiple performance issues should generate warnings
    test.assert_valid("((ab* OR bc*) AND (cd* OR de*)) AND ((e NEAR/200 f) OR (g NEAR/150 h))");
    test.assert_warning_code(
        "((ab* OR bc*) AND (cd* OR de*)) AND ((e NEAR/200 f) OR (g NEAR/150 h))",
        "W003",
    );

    // Complex query without performance issues should have no warnings
    test.assert_valid("((a OR b) AND (c OR d)) AND ((e NEAR/5 f) OR (g AND h))");
    test.assert_no_warnings("((a OR b) AND (c OR d)) AND ((e NEAR/5 f) OR (g AND h))");
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
    let mut test = QueryTest::new();

    // Valid tilde usage - standard cases
    test.assert_valid("\"apple juice\"~5");
    test.assert_no_warnings("\"apple juice\"~5");

    test.assert_valid("\"organic fruit\"~10");
    test.assert_no_warnings("\"organic fruit\"~10");

    test.assert_valid("((apple OR orange) AND phone)~5");
    test.assert_no_warnings("((apple OR orange) AND phone)~5");

    test.assert_valid("(brand OR company)~3");
    test.assert_no_warnings("(brand OR company)~3");

    test.assert_valid("((tech OR technology) AND innovation)~7");
    test.assert_no_warnings("((tech OR technology) AND innovation)~7");

    // Valid tilde with boolean operations (no parentheses needed)
    test.assert_valid("\"apple juice\"~5 AND test");
    test.assert_no_warnings("\"apple juice\"~5 AND test");

    test.assert_valid_no_warnings("(apple AND juice)~2 AND test");

    // Valid but with warnings - single term usage
    test.assert_valid("apple~5");
    test.assert_warning_code("apple~5", "W001");

    // Valid but with warnings - single quoted word
    test.assert_valid("\"apple\"~5");
    test.assert_warning_code("\"apple\"~5", "W001");

    // Valid with implicit AND and warnings
    test.assert_valid("apple~5 juice");
    test.assert_warning_code("apple~5 juice", "W001");

    // Invalid: tilde without distance number
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
    let mut test = QueryTest::new();

    let valid_multiline = fs::read_to_string("tests/fixtures/valid_multiline.bwq").unwrap();
    test.assert_valid(&valid_multiline);
    test.assert_no_warnings(&valid_multiline);

    let complex_near = fs::read_to_string("tests/fixtures/complex_near.bwq").unwrap();
    test.assert_valid(&complex_near);
    test.assert_no_warnings(&complex_near);

    let field_operations = fs::read_to_string("tests/fixtures/field_operations.bwq").unwrap();
    test.assert_valid(&field_operations);
    test.assert_no_warnings(&field_operations);

    let comments_and_wildcards =
        fs::read_to_string("tests/fixtures/comments_and_wildcards.bwq").unwrap();
    test.assert_valid(&comments_and_wildcards);
    test.assert_no_warnings(&comments_and_wildcards);

    let invalid_mixed = fs::read_to_string("tests/fixtures/invalid_mixed_operators.bwq").unwrap();
    test.assert_error_code(&invalid_mixed, "E015"); // Mixed operators without parentheses
}

#[test]
fn test_bq_file_analysis() {
    let mut test = QueryTest::new();

    let complex_near = fs::read_to_string("tests/fixtures/complex_near.bwq").unwrap();
    test.assert_valid(&complex_near);
    test.assert_no_warnings(&complex_near);

    let invalid_mixed = fs::read_to_string("tests/fixtures/invalid_mixed_operators.bwq").unwrap();
    test.assert_error_code(&invalid_mixed, "E015"); // Mixed operators without parentheses
}

#[test]
fn test_comments_dont_participate_in_implicit_and() {
    let mut test = QueryTest::new();

    test.assert_valid("apple OR <<<comment>>> juice");
    test.assert_no_warnings("apple OR <<<comment>>> juice");

    test.assert_valid("<<<comment>>> apple OR juice");
    test.assert_no_warnings("<<<comment>>> apple OR juice");

    test.assert_valid("apple <<<comment>>> OR juice");
    test.assert_no_warnings("apple <<<comment>>> OR juice");

    test.assert_valid("(election*) OR <<<DE>>> (wahl OR wahle*)");
    test.assert_no_warnings("(election*) OR <<<DE>>> (wahl OR wahle*)");

    test.assert_valid("apple <<<first>>> OR <<<second>>> juice");
    test.assert_no_warnings("apple <<<first>>> OR <<<second>>> juice");
}
