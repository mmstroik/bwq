use bw_bool::{analyze_query, is_valid_query, BrandwatchLinter};

#[test]
fn test_basic_boolean_operators() {
    assert!(is_valid_query("apple AND juice"));
    assert!(is_valid_query("apple OR orange"));
    assert!(is_valid_query("apple NOT bitter"));
    assert!(is_valid_query("(apple OR orange) AND juice"));
    
    // This should be invalid - pure negative query
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
    assert!(is_valid_query("((apple OR orange) NEAR/5 (smartphone OR phone))"));
    
    // Valid NEAR with proper parentheses
    assert!(is_valid_query("(apple NEAR/5 juice) AND orange"));
    assert!(is_valid_query("continent:europe AND (sustainability NEAR/10 climate)"));
}

#[test]
fn test_wildcards_and_replacement() {
    assert!(is_valid_query("appl*"));
    assert!(is_valid_query("customi?e"));
    assert!(is_valid_query("complain*"));
    
    // Invalid wildcard at beginning
    let mut linter = BrandwatchLinter::new();
    let report = linter.lint("*invalid").unwrap();
    assert!(report.has_errors());
}

#[test]
fn test_field_operators() {
    assert!(is_valid_query("title:\"apple juice\""));
    assert!(is_valid_query("site:twitter.com"));
    assert!(is_valid_query("author:brandwatch"));
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
    
    assert!(is_valid_query(
        r#"title:"smartphone review" AND (iPhone OR Samsung) NEAR/5 (camera OR battery)"#
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
        "*invalid",  // Wildcard at beginning
        "apple AND",  // Missing right operand
        "OR juice",   // Missing left operand
        "apple ()",   // Empty parentheses
        "rating:6",   // Invalid rating (should be 1-5) - NOTE: BW API may be more permissive
        "NEAR/0 apple juice",  // Zero distance
        "[3 TO 1]",  // Invalid range (start > end)
        "NOT bitter",  // Pure negative query (now disabled since NOT is binary)
        // NOTE: Some field validations may be too strict compared to BW API:
        // "authorGender:X" - BW API accepts this
        // "engagementType:LIKE" - BW API accepts this
        // "rating:0" - BW API accepts this
        // These discrepancies suggest our validation is stricter than BW API
    ];
    
    for query in invalid_queries {
        let analysis = analyze_query(query);
        assert!(!analysis.is_valid, "Query should be invalid: {}", query);
    }
}

#[test]
fn test_validation_warnings() {
    let mut linter = BrandwatchLinter::new();
    
    // Test performance warnings
    let report = linter.lint("a*").unwrap();  // Short wildcard
    assert!(!report.warnings.is_empty());
    
    let report = linter.lint("authorFollowers:[1 TO 2000000000]").unwrap();  // Very large range
    assert!(!report.warnings.is_empty());
    
    let report = linter.lint("languag:e").unwrap();  // Potential typo in field name
    // This should generate an error for unknown field
    assert!(report.has_errors());
}

#[test]
fn test_location_validation() {
    assert!(is_valid_query("continent:europe"));
    assert!(is_valid_query("country:gbr"));
    assert!(is_valid_query("region:usa.fl"));
    assert!(is_valid_query("city:\"deu.berlin.berlin\""));
    
    // Test coordinate validation
    let mut linter = BrandwatchLinter::new();
    let report = linter.lint("latitude:[100 TO 110]").unwrap();  // Invalid lat range
    assert!(report.has_errors());
    
    let report = linter.lint("longitude:[-200 TO -150]").unwrap();  // Invalid long range
    assert!(report.has_errors());
}

#[test]
fn test_rating_validation() {
    let mut linter = BrandwatchLinter::new();
    
    // Valid ratings
    assert!(linter.is_valid("rating:3"));
    assert!(linter.is_valid("rating:[2 TO 4]"));
    
    // Invalid ratings
    let report = linter.lint("rating:0").unwrap();
    assert!(report.has_errors());
    
    let report = linter.lint("rating:6").unwrap();
    assert!(report.has_errors());
    
    let report = linter.lint("rating:[-1 TO 3]").unwrap();
    assert!(report.has_errors());
}

#[test]
fn test_boolean_field_validation() {
    let mut linter = BrandwatchLinter::new();
    
    // Valid boolean values
    assert!(linter.is_valid("authorVerified:true"));
    assert!(linter.is_valid("authorVerified:false"));
    
    // Invalid boolean values
    let report = linter.lint("authorVerified:yes").unwrap();
    assert!(report.has_errors());
    
    let report = linter.lint("authorVerified:1").unwrap();
    assert!(report.has_errors());
}

#[test]
fn test_engagement_type_validation() {
    let mut linter = BrandwatchLinter::new();
    
    // Valid engagement types
    assert!(linter.is_valid("engagementType:COMMENT"));
    assert!(linter.is_valid("engagementType:REPLY"));
    assert!(linter.is_valid("engagementType:RETWEET"));
    assert!(linter.is_valid("engagementType:QUOTE"));
    
    // Invalid engagement type
    let report = linter.lint("engagementType:LIKE").unwrap();
    assert!(report.has_errors());
}

#[test]
fn test_verified_type_validation() {
    let mut linter = BrandwatchLinter::new();
    
    // Valid verified types
    assert!(linter.is_valid("authorVerifiedType:blue"));
    assert!(linter.is_valid("authorVerifiedType:business"));
    assert!(linter.is_valid("authorVerifiedType:government"));
    
    // Invalid verified type
    let report = linter.lint("authorVerifiedType:gold").unwrap();
    assert!(report.has_errors());
}

#[test]
fn test_minute_of_day_validation() {
    let mut linter = BrandwatchLinter::new();
    
    // Valid minute ranges
    assert!(linter.is_valid("minuteOfDay:[0 TO 1439]"));
    assert!(linter.is_valid("minuteOfDay:[720 TO 780]"));  // Noon to 1 PM
    
    // Invalid minute ranges
    let report = linter.lint("minuteOfDay:[-1 TO 100]").unwrap();
    assert!(report.has_errors());
    
    let report = linter.lint("minuteOfDay:[0 TO 1440]").unwrap();
    assert!(report.has_errors());
}

#[test]
fn test_language_code_validation() {
    let mut linter = BrandwatchLinter::new();
    
    // Valid language codes
    assert!(linter.is_valid("language:en"));
    assert!(linter.is_valid("language:es"));
    assert!(linter.is_valid("language:fr"));
    
    // Potential issues with language codes
    let report = linter.lint("language:ENG").unwrap();  // Should be lowercase
    assert!(!report.warnings.is_empty());
    
    let report = linter.lint("language:english").unwrap();  // Should be 2-char code
    assert!(!report.warnings.is_empty());
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
    
    // Very long proximity distances should generate warnings
    let report = linter.lint("apple NEAR/150 juice").unwrap();
    assert!(!report.warnings.is_empty());
    
    // Multiple wildcards in OR should generate performance warning
    let report = linter.lint("apple* OR juice*").unwrap();
    assert!(!report.warnings.is_empty());
    
    // Single character terms should generate performance warning
    let report = linter.lint("a").unwrap();
    assert!(!report.warnings.is_empty());
}

#[test]
fn test_empty_and_whitespace_queries() {
    let mut linter = BrandwatchLinter::new();
    
    // These should fail at the parsing level
    assert!(!linter.is_valid(""));
    assert!(!linter.is_valid("   "));
    assert!(!linter.is_valid("\n\t"));
}

#[test]
fn test_nested_expressions() {
    assert!(is_valid_query("((apple OR orange) AND (juice OR smoothie))"));
    assert!(is_valid_query("(apple AND (juice OR smoothie)) OR (orange AND drink)"));
    assert!(is_valid_query("NOT (apple AND (bitter OR sour))"));
}

#[test]
fn test_real_world_queries() {
    // Real-world style queries that should be valid
    let real_queries = vec![
        r#"("brand name" OR @brandhandle) AND (positive OR "great product") NOT (complaint OR "bad service")"#,
        r#"title:"product review" AND (iPhone OR Samsung OR Google) NEAR/5 (camera OR "battery life")"#,
        r#"site:reddit.com AND subreddit:technology AND ("AI" OR "artificial intelligence") NOT "clickbait""#,
        r#"authorFollowers:[1000 TO 50000] AND language:en AND engagementType:RETWEET"#,
        r#"(#MondayMotivation OR #Inspiration) AND @company_handle"#,
        r#"continent:europe AND language:en AND ("sustainability" NEAR/10 "climate change")"#,
        // Actual Brandwatch query from user
        r#"competition OR competitive OR competitor* OR saturat* OR ((China OR chinese OR cheap OR "low cost" OR "low-cost") NEAR/3 (brand* OR product* OR seller))"#,
        // Real-world queries with NOT in parentheses
        r#"uschamberofcommerce OR chamberofcommerce OR ("State Farm" NOT "stadium") OR {STFGX}"#,
        // Query with implicit AND (should be valid with warnings)
        r#"uschamberofcommerce chamberofcommerce OR ("State Farm" NOT "stadium") OR {STFGX}"#,
    ];
    
    for query in real_queries {
        assert!(is_valid_query(query), "Real-world query should be valid: {}", query);
    }
}

#[test]
fn test_implicit_and_behavior() {
    let mut linter = BrandwatchLinter::new();
    
    // Test simple implicit AND
    let report = linter.lint("apple banana").unwrap();
    assert!(!report.has_errors());  // Should be valid
    assert!(report.has_warnings()); // Should have warnings about implicit AND
    
    // Test implicit AND mixed with explicit operators
    let report = linter.lint("apple banana OR cherry").unwrap();
    assert!(!report.has_errors());  // Should be valid
    assert!(report.has_warnings()); // Should have warnings about implicit AND
    
    // Test explicit AND should not generate warnings about implicit operators
    let report = linter.lint("apple AND banana").unwrap();
    assert!(!report.has_errors());  // Should be valid
    // May have other warnings but not about implicit AND
}

#[test]
fn test_api_discrepancies_documented() {
    // These tests document known discrepancies between our linter and BW API
    // Our linter is stricter than the API in these cases
    let mut linter = BrandwatchLinter::new();
    
    // Our linter rejects these, but BW API accepts them
    let overly_strict_cases = vec![
        ("authorGender:X", "Invalid gender value"),
        ("engagementType:LIKE", "Invalid engagement type"),
        ("rating:0", "Invalid rating range"),
    ];
    
    for (query, description) in overly_strict_cases {
        let report = linter.lint(query).unwrap();
        assert!(report.has_errors(), 
            "Query '{}' should fail in our linter ({}), but BW API accepts it", 
            query, description);
    }
    
    // Test case sensitivity - lowercase operators
    let report = linter.lint("apple and juice").unwrap();
    assert!(!report.has_errors(), "Lowercase 'and' should be treated as implicit AND");
    assert!(report.has_warnings(), "Should warn about implicit AND usage");
}

#[test]
fn test_edge_case_combinations() {
    // Test complex combinations that might behave differently
    assert!(is_valid_query("apple* NEAR/5 juice*")); // Wildcards with proximity
    assert!(is_valid_query("\"apple juice\" NEAR/3 \"organic fruit\"")); // Quotes with proximity
    assert!(is_valid_query("(apple NEAR/5 juice) AND orange")); // Grouped proximity with AND
    
    // Complex field combinations
    assert!(is_valid_query("site:reddit.com AND subreddit:technology AND authorVerified:true"));
    assert!(is_valid_query("authorFollowers:[1000 TO 50000] AND engagementType:RETWEET"));
    
    // Boundary testing for coordinates
    assert!(is_valid_query("latitude:[89.9 TO 90] AND longitude:[179.9 TO 180]"));
    assert!(is_valid_query("longitude:[-180 TO -90]")); // Negative ranges
}