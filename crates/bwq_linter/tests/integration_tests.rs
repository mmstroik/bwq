use test_case::test_case;

use bwq_linter::BrandwatchLinter;
use bwq_linter::error::LintReport;

/// Test context for consistent query validation testing
pub struct QueryTest {
    linter: BrandwatchLinter,
    last_report: Option<LintReport>,
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
            last_report: None,
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
                // Store the report and return a reference to it
                self.last_report = Some(report);
                self.last_report.as_ref().unwrap()
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
                // Store the report and return a reference to it
                self.last_report = Some(report);
                self.last_report.as_ref().unwrap()
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

    /// Assert file is valid with no errors or warnings (tests full file pipeline)
    pub fn assert_file_valid_no_warnings(&mut self, file_path: &str) {
        let content = std::fs::read_to_string(file_path)
            .unwrap_or_else(|e| panic!("Failed to read file {file_path}: {e}"));
        self.assert_valid_no_warnings(&content);
    }

    /// Assert file has error with specific code (tests full file pipeline)
    pub fn assert_file_error_code(&mut self, file_path: &str, expected_code: &str) {
        let content = std::fs::read_to_string(file_path)
            .unwrap_or_else(|e| panic!("Failed to read file {file_path}: {e}"));
        self.assert_error_code(&content, expected_code);
    }

    /// Assert file has warning with specific code (tests full file pipeline)
    pub fn assert_file_warning_code(&mut self, file_path: &str, expected_code: &str) {
        let content = std::fs::read_to_string(file_path)
            .unwrap_or_else(|e| panic!("Failed to read file {file_path}: {e}"));
        self.assert_warning_code(&content, expected_code);
    }
}

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

/// File-based test expectation for fixture testing
#[derive(Debug, Clone)]
pub enum FileTestExpectation {
    /// File should be valid with no errors or warnings
    ValidNoWarnings,
    /// File should have an error with specific code
    ErrorCode(&'static str),
    /// File should have a warning with specific code
    WarningCode(&'static str),
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

impl FileTestExpectation {
    /// Apply this expectation to a file using the test context
    pub fn assert(&self, test: &mut QueryTest, file_path: &str) {
        match self {
            FileTestExpectation::ValidNoWarnings => {
                test.assert_file_valid_no_warnings(file_path);
            }
            FileTestExpectation::ErrorCode(error_code) => {
                test.assert_file_error_code(file_path, error_code);
            }
            FileTestExpectation::WarningCode(warning_code) => {
                test.assert_file_warning_code(file_path, warning_code);
            }
        }
    }
}

// ============================================================================
// BASIC SYNTAX VALIDATION
// Tests for basic syntax validation
// ============================================================================

#[test_case("apple AND juice", TestExpectation::ValidNoWarnings; "basic AND operation")]
#[test_case("apple OR orange", TestExpectation::ValidNoWarnings; "basic OR operation")]
#[test_case("apple NOT bitter NOT sour", TestExpectation::ValidNoWarnings; "basic NOT operation")]
#[test_case("(apple OR orange) AND juice", TestExpectation::ValidNoWarnings; "parenthesized boolean operations")]
#[test_case("NOT bitter", TestExpectation::ErrorCode("E013"); "pure negative query error")]
#[test_case("NOT term1 AND term2", TestExpectation::ValidNoWarnings; "leading NOT with AND")]
#[test_case("NOT term1 NOT term2", TestExpectation::ErrorCode("E013"); "double NOT pure negative query error")]
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

#[test_case("appl*", TestExpectation::ValidNoWarnings; "asterisk wildcard at end")]
#[test_case("tes*t", TestExpectation::ValidNoWarnings; "asterisk wildcard in middle")]
#[test_case("customi?e", TestExpectation::ValidNoWarnings; "question mark wildcard in middle")]
#[test_case("ab*", TestExpectation::ValidNoWarnings; "two character wildcard no warning")]
#[test_case("#*test", TestExpectation::ValidWithWarning("W002"); "wildcard after hashtag prefix performance warning")]
#[test_case("@*test", TestExpectation::ValidWithWarning("W002"); "wildcard after @ prefix performance warning")]
#[test_case("*invalid", TestExpectation::ErrorCode("E004"); "invalid wildcard at beginning")]
#[test_case("a*", TestExpectation::ErrorCode("E004"); "short wildcard matches too many unique terms")]
#[test_case("t*est", TestExpectation::ValidNoWarnings; "wildcard in middle with characters after")]
fn test_wildcard_syntax(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("apple NEAR/3 juice", TestExpectation::ValidNoWarnings; "basic NEAR operator")]
#[test_case("apple NEAR/2f juice*", TestExpectation::ValidNoWarnings; "NEAR forward")]
#[test_case("((apple OR orange) NEAR/5 (smartphone OR phone))", TestExpectation::ValidNoWarnings; "NEAR with grouped terms")]
#[test_case("(apple NEAR/5 juice) AND orange", TestExpectation::ValidNoWarnings; "NEAR with boolean AND")]
#[test_case("continent:europe AND (sustainability NEAR/10 climate)", TestExpectation::ValidNoWarnings; "field with NEAR operation")]
fn test_near_proximity_operator_syntax(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("\"apple juice\"~5", TestExpectation::ValidNoWarnings; "basic quoted phrase with tilde")]
#[test_case("(brand OR company)~3", TestExpectation::ValidNoWarnings; "simple group with tilde")]
#[test_case("((tech OR technology) AND innovation)~7", TestExpectation::ValidNoWarnings; "nested boolean group with tilde")]
#[test_case("\"apple juice\"~5 AND test", TestExpectation::ValidNoWarnings; "quoted tilde with boolean AND")]
#[test_case("(apple AND juice)~2 AND test", TestExpectation::ValidNoWarnings; "grouped terms with tilde and AND")]
#[test_case("apple~5", TestExpectation::ValidWithWarning("W001"); "single term with tilde warning")]
#[test_case("\"apple\"~5", TestExpectation::ValidWithWarning("W001"); "single quoted word with tilde warning")]
#[test_case("apple~5 juice", TestExpectation::ValidWithWarning("W001"); "tilde with implicit AND warning")]
#[test_case("\"apple juice\"~", TestExpectation::ErrorCode("E002"); "tilde without distance number on phrase")]
#[test_case("apple~", TestExpectation::ErrorCode("E002"); "tilde without distance number on term")]
#[test_case("apple ~ juice", TestExpectation::ErrorCode("E002"); "tilde with spaces")]
#[test_case("apple~ 5", TestExpectation::ErrorCode("E002"); "space between tilde and number")]
#[test_case("apple ~5", TestExpectation::ErrorCode("E002"); "space before tilde")]
#[test_case("apple~5t", TestExpectation::ErrorCode("E001"); "invalid characters after number")]
fn test_tilde_proximity_syntax(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test]
fn test_case_sensitive_matching() {
    let mut test = QueryTest::new();

    test.assert_valid_no_warnings("{BrandWatch}");

    test.assert_valid_no_warnings("apple AND {BT}");

    // with spaces
    test.assert_valid_no_warnings("{Brand Watch}");
}

#[test]
fn test_comments() {
    let mut test = QueryTest::new();

    test.assert_valid_no_warnings("apple <<<This is a comment>>> AND juice");
    test.assert_valid_no_warnings("<<<Brand monitoring>>> \"brand name\"");
    test.assert_valid_no_warnings("(election*) OR <<<DE>>> (wahl OR wahle*)");
    test.assert_valid_no_warnings("apple <<<first>>> OR <<<second>>> juice");
}

#[test_case("#MondayMotivation", TestExpectation::ValidNoWarnings; "hashtag syntax")]
#[test_case("@brandwatch", TestExpectation::ValidNoWarnings; "mention syntax")]
#[test_case("#hashtag AND @mention", TestExpectation::ValidNoWarnings; "hashtag and mention combined")]
#[test_case("test;test;test", TestExpectation::ValidNoWarnings; "semicolons in term")]
fn test_special_character_syntax(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

// Common invalid query patterns
#[test_case("apple AND", TestExpectation::ErrorCode("E007"); "missing right operand")]
#[test_case("OR juice", TestExpectation::ErrorCode("E007"); "missing left operand")]
#[test_case("apple AND ()", TestExpectation::ErrorCode("E007"); "empty parentheses")]
#[test_case("NOT bitter", TestExpectation::ErrorCode("E013"); "pure negative query")]
fn test_invalid_query_patterns(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test]
fn test_basic_field_operators() {
    let mut test = QueryTest::new();

    test.assert_valid_no_warnings("title:\"apple juice\"");

    test.assert_valid_no_warnings("site:twitter.com");

    test.assert_valid_no_warnings("author:  brandwatch"); // whitespace after colon is allowed

    test.assert_valid_no_warnings("blogName:comedycentral");

    test.assert_valid_no_warnings("tags:photography");

    test.assert_valid_no_warnings("subreddit:nba");

    // Should fail - space before colon
    test.assert_error_code("subreddit : nba", "E001");
    test.assert_error_code("subreddit :nba", "E001");
    test.assert_error_code("randomword : randomword2", "E001");
}

#[test_case("दुष्प्रचार OR \"नकली खबर\" OR नकलीखबर ", TestExpectation::ValidNoWarnings; "hindi text")]
#[test_case("नमस्कार AND goodbye", TestExpectation::ValidNoWarnings; "hindi with english")]
#[test_case("🇪🇺 AND europe", TestExpectation::ValidNoWarnings; "flag emoji")]
#[test_case("€100 OR $50", TestExpectation::ValidNoWarnings; "currency symbols")]
#[test_case("café AND नमस्ते", TestExpectation::ValidNoWarnings; "mixed unicode")]
#[test_case("O'Reilly OR McDonald's", TestExpectation::ValidNoWarnings; "names with apostrophes")]
#[test_case("🎉 celebration", TestExpectation::ValidWithWarning("W001"); "emoji with implicit AND")]
fn test_unicode_and_special_characters(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("https:www.youtube.com/", TestExpectation::ValidNoWarnings; "single slash after colon")]
#[test_case("https:w/ww.youtube.com/", TestExpectation::ValidNoWarnings; "slash in middle")]
#[test_case("https:/www.youtube.com/", TestExpectation::ValidNoWarnings; "double slash missing one")]
#[test_case("https://www.youtube.com/", TestExpectation::ValidNoWarnings; "full URL format")]
#[test_case("site:reddit.com/r/programming", TestExpectation::ValidNoWarnings; "site operator with path")]
#[test_case("url:example.com/path/to/page", TestExpectation::ValidNoWarnings; "url with path")]
fn test_url_like_strings(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

// ============================================================================
// FIELD OPERATOR VALIDATION
// Tests for field operator validation
// ============================================================================

#[test_case("rating:3", TestExpectation::ValidNoWarnings; "valid rating 3")]
#[test_case("rating:0", TestExpectation::ValidNoWarnings; "valid rating 0")]
#[test_case("rating:[2 TO 4]", TestExpectation::ValidNoWarnings; "valid rating range")]
#[test_case("rating:6", TestExpectation::ErrorCode("E009"); "rating too high")]
#[test_case("rating:[-1 TO 3]", TestExpectation::ErrorCode("E009"); "rating range with negative")]
#[test_case("rating:[x TO y]", TestExpectation::ErrorCode("E009"); "invalid rating with literal letters")]
#[test_case("rating:[1 to 5]", TestExpectation::ErrorCode("E008"); "invalid rating with lowercase to")]
fn test_rating_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("latitude:[40 TO 42]", TestExpectation::ValidNoWarnings; "valid latitude range")]
#[test_case("longitude:[-73 TO -69]", TestExpectation::ValidNoWarnings; "valid longitude range")]
#[test_case("continent:europe", TestExpectation::ValidNoWarnings; "valid continent")]
#[test_case("latitude:[100 TO 110]", TestExpectation::ErrorCode("E009"); "latitude out of range")]
#[test_case("longitude:[-200 TO -150]", TestExpectation::ErrorCode("E009"); "longitude out of range")]
#[test_case("latitude:[x TO y]", TestExpectation::ErrorCode("E009"); "invalid latitude with literal letters")]
fn test_coordinate_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("authorVerified:true", TestExpectation::ValidNoWarnings; "valid boolean true")]
#[test_case("authorVerified:false", TestExpectation::ValidNoWarnings; "valid boolean false")]
#[test_case("authorVerified:yes", TestExpectation::ErrorCode("E009"); "invalid boolean yes")]
#[test_case("authorVerified:1", TestExpectation::ErrorCode("E009"); "invalid boolean number")]
fn test_boolean_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("language:en", TestExpectation::ValidNoWarnings; "valid 2-char language code")]
#[test_case("language:fr", TestExpectation::ValidNoWarnings; "valid french language code")]
#[test_case("language:ENG", TestExpectation::ValidWithWarning("W001"); "uppercase language code warning")]
#[test_case("language:english", TestExpectation::ValidWithWarning("W001"); "full language name warning")]
#[test_case("languag:e", TestExpectation::ValidNoWarnings; "invalid field operator is valid")]
fn test_language_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("engagementType:COMMENT", TestExpectation::ValidNoWarnings; "valid engagement comment")]
#[test_case("engagementType:REPLY", TestExpectation::ValidNoWarnings; "valid engagement reply")]
#[test_case("engagementType:RETWEET", TestExpectation::ValidNoWarnings; "valid engagement retweet")]
#[test_case("engagementType:QUOTE", TestExpectation::ValidNoWarnings; "valid engagement quote")]
#[test_case("engagementType:LIKE", TestExpectation::ErrorCode("E009"); "invalid engagement like")]
fn test_engagement_type_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("authorVerifiedType:blue", TestExpectation::ValidNoWarnings; "valid verified type blue")]
#[test_case("authorVerifiedType:business", TestExpectation::ValidNoWarnings; "valid verified type business")]
#[test_case("authorVerifiedType:government", TestExpectation::ValidNoWarnings; "valid verified type government")]
#[test_case("authorVerifiedType:gold", TestExpectation::ErrorCode("E009"); "invalid verified type gold")]
fn test_verified_type_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("minuteOfDay:[0 TO 1439]", TestExpectation::ValidNoWarnings; "valid minute of day full range")]
#[test_case("minuteOfDay:[720 TO 780]", TestExpectation::ValidNoWarnings; "valid minute of day noon to 1pm")]
#[test_case("minuteOfDay:[-1 TO 100]", TestExpectation::ErrorCode("E009"); "minute of day with negative")]
#[test_case("minuteOfDay:[0 TO 1440]", TestExpectation::ErrorCode("E009"); "minute of day over max")]
fn test_minute_of_day_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("authorFollowers:[0 TO 5000]", TestExpectation::ValidNoWarnings; "valid author followers full range")]
#[test_case("authorFollowers:[-100 TO 10000]", TestExpectation::ErrorCode("E011"); "invalid author followers negative")]
#[test_case("authorFollowers:[100000 TO 1000]", TestExpectation::ErrorCode("E011"); "invalid author followers start greater than end")]
#[test_case("authorFollowers:[0 TO 10000000000]", TestExpectation::ErrorCode("E011"); "invalid author followers over max digits")]
#[test_case("authorFollowers:[x TO y]", TestExpectation::ErrorCode("E009"); "invalid author followers with literal letters")]
#[test_case("authorFollowers:term", TestExpectation::ErrorCode("E009"); "authorFollowers requires range not term")]
fn test_author_followers_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("country:gbr", TestExpectation::ValidNoWarnings; "valid country code")]
#[test_case("region:usa.fl", TestExpectation::ValidNoWarnings; "valid region code")]
#[test_case("city:\"deu.berlin.berlin\"", TestExpectation::ValidNoWarnings; "valid city code")]
fn test_location_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("guid:123456789", TestExpectation::ValidNoWarnings; "valid guid digits only")]
#[test_case("guid:123_456_789", TestExpectation::ValidNoWarnings; "valid guid with underscores")]
#[test_case("guid:term", TestExpectation::ErrorCode("E009"); "guid should be digits or digits with underscores")]
#[test_case("guid:123abc", TestExpectation::ErrorCode("E009"); "guid should not contain letters")]
#[test_case("guid:123-456", TestExpectation::ErrorCode("E009"); "guid should not contain dashes")]
fn test_guid_field_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

// ============================================================================
// MISC TESTS
// Tests for misc validation
// ============================================================================

#[test_case("apple NEAR/150 juice", TestExpectation::ValidNoWarnings; "NEAR with large distance should not generate warnings")]
#[test_case("apple* OR juice*", TestExpectation::ValidNoWarnings; "multiple wildcards in OR")]
#[test_case("a", TestExpectation::ValidNoWarnings; "single character should not generate warnings")]
#[test_case("42 OR 24*", TestExpectation::ValidNoWarnings; "mixing pure numbers and numeric wildcards")]
fn test_performance_edge_cases(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test_case("", TestExpectation::ErrorCode("E007"); "empty query")]
#[test_case("   ", TestExpectation::ErrorCode("E007"); "whitespace only query")]
#[test_case("\n\t", TestExpectation::ErrorCode("E007"); "newline and tab only")]
fn test_empty_and_whitespace_queries(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

// ============================================================================
// INTERACTION TESTS
// Tests for interactions between operators and tokens
// ============================================================================

#[test]
fn test_implicit_and_behavior() {
    let mut test = QueryTest::new();

    // Implicit AND should be valid but generate warnings
    test.assert_valid("apple banana");
    test.assert_warning_code("apple banana", "W001");

    // Mixed implicit AND with OR should fail without parentheses
    test.assert_error_code("apple banana OR cherry", "E012");

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

    test.assert_valid_no_warnings("((smartphone OR phone) NEAR/5 (review OR rating)) AND ((camera OR battery) NEAR/3 (excellent OR amazing))");

    test.assert_valid_no_warnings("\"apple juice\"~5 AND (organic OR natural)");

    test.assert_valid_no_warnings("juice NOT (apple AND (bitter OR sour))");

    test.assert_valid_no_warnings(
        "((brand OR company) NEAR/2f (announcement OR news)) AND (exciting OR important)",
    );

    test.assert_valid_no_warnings(
        "((complain* NEAR/5 product*) NOT (resolve* NEAR/3 solution*)) AND site:twitter.com",
    );
}

#[test_case("apple OR banana AND juice", TestExpectation::ErrorCode("E012"); "mixed OR AND without parentheses")]
#[test_case("apple AND banana OR juice AND smoothie", TestExpectation::ErrorCode("E012"); "mixed AND OR without parentheses")]
#[test_case("apple NOT bitter AND sweet OR sour", TestExpectation::ErrorCode("E012"); "mixed NOT AND OR without parentheses")]
#[test_case("(apple OR banana) AND juice", TestExpectation::ValidNoWarnings; "properly parenthesized OR AND")]
#[test_case("(apple AND banana) OR (juice AND smoothie)", TestExpectation::ValidNoWarnings; "properly parenthesized AND OR")]
#[test_case("apple NOT (bitter AND sweet) OR sour", TestExpectation::ValidNoWarnings; "properly parenthesized NOT AND OR")]
fn test_operator_precedence_validation(query: &str, expected: TestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, query);
}

#[test]
fn test_near_operator_interaction_validation() {
    let mut test = QueryTest::new();

    let mixed_near_boolean_cases = vec![
        "(apple OR orange) NEAR/5 (juice OR drink) AND fresh",
        "(apple OR orange) NEAR/5 (juice OR drink) OR fresh",
    ];

    for query in mixed_near_boolean_cases {
        test.assert_error_code(query, "E010");
    }
    // proper parentheses
    let valid_near_cases = vec![
        "((apple OR orange) NEAR/5 (juice OR drink)) AND fresh",
        "(apple NEAR/3 banana) OR (juice NEAR/2 smoothie)",
        "(apple NEAR/5 juice) AND (banana NEAR/3 smoothie)",
    ];

    for query in valid_near_cases {
        test.assert_valid_no_warnings(query);
    }
}

// ============================================================================
// FIXTURE-BASED TESTS
// Tests using .bwq files from resources/test/fixtures/
// ============================================================================

#[test_case("resources/test/fixtures/valid/complex_boolean_operations.bwq", FileTestExpectation::ValidNoWarnings; "complex boolean operations")]
#[test_case("resources/test/fixtures/valid/complex_field_combinations.bwq", FileTestExpectation::ValidNoWarnings; "complex field combinations")]
#[test_case("resources/test/fixtures/valid/complex_proximity_operations.bwq", FileTestExpectation::ValidNoWarnings; "complex proximity operations")]
#[test_case("resources/test/fixtures/valid/complex_social_media.bwq", FileTestExpectation::ValidNoWarnings; "complex social media query")]
#[test_case("resources/test/fixtures/valid/valid_real_world_multiline.bwq", FileTestExpectation::ValidNoWarnings; "real world multiline query")]
#[test_case("resources/test/fixtures/valid/complex_near.bwq", FileTestExpectation::ValidNoWarnings; "complex NEAR operations")]
#[test_case("resources/test/fixtures/valid/field_operations.bwq", FileTestExpectation::ValidNoWarnings; "field operations")]
#[test_case("resources/test/fixtures/valid/comments_and_wildcards.bwq", FileTestExpectation::ValidNoWarnings; "comments and wildcards")]
#[test_case("resources/test/fixtures/invalid/invalid_mixed_operators.bwq", FileTestExpectation::ErrorCode("E012"); "invalid mixed operators")]
fn test_fixture_files(file_path: &str, expected: FileTestExpectation) {
    let mut test = QueryTest::new();
    expected.assert(&mut test, file_path);
}
