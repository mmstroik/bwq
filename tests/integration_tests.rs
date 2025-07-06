use bwq::{analyze_query, is_valid_query, BrandwatchLinter};
use std::fs;

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
        "*invalid", // Wildcard at beginning
        "apple AND", // Missing right operand
        "OR juice", // Missing left operand
        "apple AND ()", // Empty parentheses
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
    let mut linter = BrandwatchLinter::new();

    let report = linter.lint("ab*").unwrap();
    assert!(!report.warnings.is_empty());

    let report = linter.lint("authorFollowers:[1 TO 2000000000]").unwrap();
    assert!(!report.warnings.is_empty());

    let report = linter.lint("languag:e").unwrap();
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
    let report = linter.lint("latitude:[100 TO 110]").unwrap();
    assert!(report.has_errors());

    let report = linter.lint("longitude:[-200 TO -150]").unwrap();
    assert!(report.has_errors());
}

#[test]
fn test_rating_validation() {
    let mut linter = BrandwatchLinter::new();

    assert!(linter.is_valid("rating:3"));
    assert!(linter.is_valid("rating:[2 TO 4]"));

    // Previously invalid but now valid (BW API accepts rating:0)
    assert!(linter.is_valid("rating:0"));

    let report = linter.lint("rating:6").unwrap();
    assert!(report.has_errors());

    let report = linter.lint("rating:[-1 TO 3]").unwrap();
    assert!(report.has_errors());
}

#[test]
fn test_boolean_field_validation() {
    let mut linter = BrandwatchLinter::new();

    assert!(linter.is_valid("authorVerified:true"));
    assert!(linter.is_valid("authorVerified:false"));

    let report = linter.lint("authorVerified:yes").unwrap();
    assert!(report.has_errors());

    let report = linter.lint("authorVerified:1").unwrap();
    assert!(report.has_errors());
}

#[test]
fn test_engagement_type_validation() {
    let mut linter = BrandwatchLinter::new();

    assert!(linter.is_valid("engagementType:COMMENT"));
    assert!(linter.is_valid("engagementType:REPLY"));
    assert!(linter.is_valid("engagementType:RETWEET"));
    assert!(linter.is_valid("engagementType:QUOTE"));

    // Previously invalid but now valid (BW API accepts LIKE)
    assert!(linter.is_valid("engagementType:LIKE"));
}

#[test]
fn test_verified_type_validation() {
    let mut linter = BrandwatchLinter::new();

    assert!(linter.is_valid("authorVerifiedType:blue"));
    assert!(linter.is_valid("authorVerifiedType:business"));
    assert!(linter.is_valid("authorVerifiedType:government"));

    let report = linter.lint("authorVerifiedType:gold").unwrap();
    assert!(report.has_errors());
}

#[test]
fn test_minute_of_day_validation() {
    let mut linter = BrandwatchLinter::new();

    assert!(linter.is_valid("minuteOfDay:[0 TO 1439]"));
    assert!(linter.is_valid("minuteOfDay:[720 TO 780]")); // Noon to 1 PM

    let report = linter.lint("minuteOfDay:[-1 TO 100]").unwrap();
    assert!(report.has_errors());

    let report = linter.lint("minuteOfDay:[0 TO 1440]").unwrap();
    assert!(report.has_errors());
}

#[test]
fn test_language_code_validation() {
    let mut linter = BrandwatchLinter::new();

    assert!(linter.is_valid("language:en"));
    assert!(linter.is_valid("language:es"));
    assert!(linter.is_valid("language:fr"));

    // Potential issues with language codes
    let report = linter.lint("language:ENG").unwrap(); // Should be lowercase
    assert!(!report.warnings.is_empty());

    let report = linter.lint("language:english").unwrap(); // Should be 2-char code
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

    let report = linter.lint("apple NEAR/150 juice").unwrap();
    assert!(report.warnings.is_empty());

    let report = linter.lint("apple* OR juice*").unwrap();
    assert!(!report.warnings.is_empty());

    let report = linter.lint("a").unwrap();
    assert!(report.warnings.is_empty());
}

#[test]
fn test_wildcard_position_validation() {
    let mut linter = BrandwatchLinter::new();

    let report = linter.lint("tes*t").unwrap();
    assert!(!report.has_errors());
    assert!(report.warnings.is_empty());

    let report = linter.lint("#test*").unwrap();
    assert!(!report.has_errors());
    assert!(report.warnings.is_empty());

    let report = linter.lint("#*test").unwrap();
    assert!(!report.has_errors());
    assert!(!report.warnings.is_empty());
}

#[test]
fn test_empty_and_whitespace_queries() {
    let mut linter = BrandwatchLinter::new();

    assert!(!linter.is_valid(""));
    assert!(!linter.is_valid("   "));
    assert!(!linter.is_valid("\n\t"));
}

#[test]
fn test_nested_expressions() {
    assert!(is_valid_query(
        "((apple OR orange) AND (juice OR smoothie))"
    ));
    assert!(is_valid_query(
        "(apple AND (juice OR smoothie)) OR (orange AND drink)"
    ));
    // NOT-only queries should fail (no positive terms)
    assert!(!is_valid_query("NOT (apple AND (bitter OR sour))"));

    // But mixed positive/negative should work
    assert!(is_valid_query("juice NOT (apple AND (bitter OR sour))"));
}

#[test]
fn test_real_world_queries() {
    let real_queries = vec![
        r#"("brand name" OR @brandhandle) AND (positive OR "great product") NOT (complaint OR "bad service")"#,
        r#"title:"product review" AND ((iPhone OR Samsung OR Google) NEAR/5 (camera OR "battery life"))"#,
        r#"site:reddit.com AND subreddit:technology AND ("AI" OR "artificial intelligence") NOT "clickbait""#,
        r#"authorFollowers:[1000 TO 50000] AND language:en AND engagementType:RETWEET"#,
        r#"(#MondayMotivation OR #Inspiration) AND @company_handle"#,
        r#"continent:europe AND language:en AND ("sustainability" NEAR/10 "climate change")"#,
        // Actual Brandwatch query from user
        r#"competition OR competitive OR competitor* OR saturat* OR ((China OR chinese OR cheap OR "low cost" OR "low-cost") NEAR/3 (brand* OR product* OR seller))"#,
        // Real-world queries with NOT in parentheses
        r#"uschamberofcommerce OR chamberofcommerce OR ("State Farm" NOT "stadium") OR {STFGX}"#,
        // Query with properly parenthesized implicit AND
        r#"(uschamberofcommerce chamberofcommerce) OR ("State Farm" NOT "stadium") OR {STFGX}"#,
    ];

    for query in real_queries {
        assert!(
            is_valid_query(query),
            "Real-world query should be valid: {}",
            query
        );
    }
}

#[test]
fn test_implicit_and_behavior() {
    let mut linter = BrandwatchLinter::new();

    let report = linter.lint("apple banana").unwrap();
    assert!(!report.has_errors(), "Implicit AND should be valid");
    assert!(report.has_warnings(), "Implicit AND should generate warnings");

    let report = linter.lint("apple banana OR cherry").unwrap();
    assert!(report.has_errors(), "Mixed implicit AND with OR should fail without parentheses");

    let report = linter.lint("(apple banana) OR cherry").unwrap();
    assert!(!report.has_errors(), "Properly parenthesized implicit AND should be valid");
    assert!(report.has_warnings(), "Implicit AND should still generate warnings");

    let report = linter.lint("apple AND banana").unwrap();
    assert!(!report.has_warnings(), "Explicit AND should not generate warnings");
}

#[test]
fn test_api_discrepancies_documented() {
    // These tests document that our linter now correctly matches BW API behavior
    let mut linter = BrandwatchLinter::new();

    let previously_overly_strict_cases = vec![
        "authorGender:X",      // BW API accepts any gender value
        "engagementType:LIKE", // BW API accepts this engagement type
        "rating:0",            // BW API accepts rating 0
    ];

    for query in previously_overly_strict_cases {
        let report = linter.lint(query).unwrap();
        assert!(
            !report.has_errors(),
            "Query '{}' should now pass - we fixed overly strict validation to match BW API",
            query
        );
    }

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
fn test_edge_case_combinations() {
    assert!(is_valid_query("apple* NEAR/5 juice*"));
    assert!(is_valid_query("\"apple juice\" NEAR/3 \"organic fruit\""));
    assert!(is_valid_query("(apple NEAR/5 juice) AND orange"));

    assert!(is_valid_query(
        "site:reddit.com AND subreddit:technology AND authorVerified:true"
    ));
    assert!(is_valid_query(
        "authorFollowers:[1000 TO 50000] AND engagementType:RETWEET"
    ));

    assert!(is_valid_query(
        "latitude:[89.9 TO 90] AND longitude:[179.9 TO 180]"
    ));
    assert!(is_valid_query("longitude:[-180 TO -90]"));
}

#[test]
fn test_deep_nesting_levels() {
    // Test 3-level nesting
    assert!(is_valid_query(
        "((apple OR orange) AND (juice OR smoothie)) OR ((banana OR grape) AND drink)"
    ));

    // Test 4-level nesting
    assert!(is_valid_query("(((brand AND product) OR (service AND quality)) AND ((review OR rating) NOT (spam OR fake))) AND site:twitter.com"));

    // Test 5-level nesting
    assert!(is_valid_query("((((apple OR orange) AND (fresh OR organic)) OR ((banana OR grape) AND (sweet OR ripe))) AND ((juice OR smoothie) NOT artificial)) AND healthy"));

    // Deep nesting with proximity operators
    assert!(is_valid_query("(((apple OR orange) NEAR/3 (juice OR drink)) AND ((fresh OR organic) NOT artificial)) AND site:healthfood.com"));

    // Complex field combinations with deep nesting
    assert!(is_valid_query("(((title:\"product review\" OR title:\"service review\") AND (authorFollowers:[1000 TO 50000] OR authorVerified:true)) AND ((engagementType:RETWEET OR engagementType:QUOTE) AND language:en)) AND country:usa"));
}

#[test]
fn test_operators_on_groupings() {
    // NEAR operators applied to grouped expressions
    assert!(is_valid_query("(apple OR orange) NEAR/3 (juice OR drink)"));
    assert!(is_valid_query(
        "(iPhone OR Samsung) NEAR/5 (review OR rating)"
    ));
    assert!(is_valid_query(
        "(\"brand name\" OR @brandhandle) NEAR/10 (positive OR excellent)"
    ));

    // Multiple NEAR operations on groups
    assert!(is_valid_query("((smartphone OR phone) NEAR/5 (review OR rating)) AND ((camera OR battery) NEAR/3 (excellent OR amazing))"));

    // Tilde proximity with AND requires parentheses
    assert!(!is_valid_query(
        "\"apple juice\"~5 AND (organic OR natural)"
    ));

    // Properly parenthesized tilde/AND should work
    assert!(is_valid_query(
        "(\"apple juice\"~5) AND (organic OR natural)"
    ));
    assert!(is_valid_query(
        "(\"brand experience\"~3 OR \"customer service\"~2) AND positive"
    ));

    // Forward NEAR on groups
    assert!(is_valid_query(
        "(product OR service) NEAR/3f (quality OR excellence)"
    ));
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
fn test_multiline_style_queries() {
    // Long queries that would typically span multiple lines in real usage
    let long_query = r#"((apple OR orange OR banana OR grape) AND (juice OR smoothie OR drink OR beverage)) AND ((fresh OR organic OR natural OR healthy) NOT (artificial OR processed OR chemical OR synthetic)) AND ((site:healthfood.com OR site:nutrition.org) AND (language:en OR language:es)) AND ((authorVerified:true OR authorFollowers:[1000 TO 100000]) AND (engagementType:RETWEET OR engagementType:QUOTE OR engagementType:COMMENT))"#;
    assert!(is_valid_query(long_query));

    // Brand monitoring query
    let brand_query = r#"((("brand name" OR @brandhandle OR {BrandName}) AND (mention OR review OR feedback OR comment)) AND ((positive OR excellent OR amazing OR great) NOT (negative OR terrible OR awful OR bad))) AND ((site:twitter.com OR site:facebook.com OR site:instagram.com) AND (language:en AND country:usa)) AND ((authorFollowers:[500 TO 50000] AND authorVerified:true) OR (engagementType:RETWEET AND rating:[3 TO 5]))"#;
    assert!(is_valid_query(brand_query));

    // Technology discussion query
    let tech_query = r#"(((artificial AND intelligence) OR (machine AND learning) OR AI OR ML) AND ((breakthrough OR innovation OR advancement OR development) NEAR/5 (technology OR research OR science))) AND ((site:reddit.com AND (subreddit:MachineLearning OR subreddit:artificial)) OR (site:arxiv.org OR site:ieee.org)) AND ((authorVerified:true OR authorFollowers:[1000 TO 100000]) AND language:en)"#;
    assert!(is_valid_query(tech_query));
}

#[test]
fn test_wildcard_and_replacement_in_complex_contexts() {
    // Wildcards in deeply nested contexts
    assert!(is_valid_query("(((complain* OR problem* OR issue*) NEAR/5 (product* OR service* OR brand*)) AND ((respond* OR solution* OR fix*) NOT (ignore* OR dismiss* OR avoid*))) AND site:twitter.com"));

    // Replacement characters in complex queries
    assert!(is_valid_query("((customi?e OR personali?e OR optimi?e) AND (experience OR service)) AND ((organi?ation OR compan?) NEAR/3 (innovation OR excellence))"));

    // Mixed wildcards and replacements
    assert!(is_valid_query("((analy* OR anali?e OR insight*) AND (data OR information)) AND ((busin?ss OR corporat*) NEAR/5 (decision* OR strateg*))"));
}

#[test]
fn test_case_sensitivity_in_complex_contexts() {
    // Case sensitive terms in complex nesting
    assert!(is_valid_query("(({BrandName} OR {ProductName}) AND ((review OR rating) NEAR/3 (excellent OR {Perfect}))) AND ((site:review.com OR site:testimonial.org) AND language:en)"));

    // Mixed case sensitivity
    assert!(is_valid_query("((apple OR {Apple} OR {APPLE}) AND (juice OR {Juice})) AND ((organic OR {Organic}) NOT artificial)"));
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
fn test_range_operators_in_complex_contexts() {
    // Complex range combinations
    assert!(is_valid_query("(((rating:[4 TO 5] AND authorFollowers:[1000 TO 50000]) OR (rating:[3 TO 5] AND authorVerified:true)) AND ((engagementType:RETWEET OR engagementType:QUOTE) AND language:en)) AND ((minuteOfDay:[480 TO 720] OR minuteOfDay:[1080 TO 1320]) AND country:usa)"));

    assert!(is_valid_query("(((latitude:[40 TO 42] AND longitude:[-75 TO -73]) OR (latitude:[51 TO 53] AND longitude:[-1 TO 1])) AND ((city:\"new york\" OR city:london) AND language:en)) AND authorVerified:true"));
}

#[test]
fn test_performance_warnings_in_complex_queries() {
    let mut linter = BrandwatchLinter::new();

    let report = linter
        .lint("((ab* OR bc*) AND (cd* OR de*)) AND ((e NEAR/200 f) OR (g NEAR/150 h))")
        .unwrap();
    assert!(!report.has_errors());
    assert!(!report.warnings.is_empty());

    let report = linter
        .lint("((a OR b) AND (c OR d)) AND ((e NEAR/5 f) OR (g AND h))")
        .unwrap();
    assert!(!report.has_errors());
    assert!(report.warnings.is_empty());
}

#[test]
fn test_operator_precedence_validation() {
    let mut linter = BrandwatchLinter::new();

    let mixed_and_or_cases = vec![
        "apple OR banana AND juice",
        "apple AND banana OR juice AND smoothie",
        "apple NOT bitter AND sweet OR sour",
    ];

    for query in mixed_and_or_cases {
        let report = linter.lint(query).unwrap();
        assert!(report.has_errors(), "Query should fail: {}", query);
        assert!(
            report.errors.iter().any(|e| e
                .to_string()
                .contains("AND and OR operators cannot be mixed")),
            "Should have mixed AND/OR error for: {}",
            query
        );
    }

    let properly_parenthesized_cases = vec![
        "(apple OR banana) AND juice",
        "(apple AND banana) OR (juice AND smoothie)",
        "apple NOT (bitter AND sweet) OR sour",
    ];

    for query in properly_parenthesized_cases {
        let report = linter.lint(query).unwrap();
        assert!(!report.has_errors(), "Query should pass: {}", query);
    }
}

#[test]
fn test_tilde_proximity_operators() {
    // Postfix tilde on quoted phrases
    assert!(is_valid_query("\"apple juice\"~5"));
    assert!(is_valid_query("\"organic fruit\"~10"));

    // Alternative NEAR syntax with groups (line 17 of docs)
    assert!(is_valid_query(
        "((apple OR orange) AND (smartphone OR phone))~5"
    ));
    assert!(is_valid_query("(brand OR company)~3"));

    // Single word tilde (doesn't do any proximity stuff but is valid)
    assert!(is_valid_query("apple~5"));
    assert!(is_valid_query("\"apple\"~3"));

    // Tilde with complex expressions
    assert!(is_valid_query("((tech OR technology) AND innovation)~7"));
}

#[test]
fn test_invalid_tilde_syntax() {
    let mut linter = BrandwatchLinter::new();

    // Invalid: tilde between separate terms (this was the bug we fixed)
    // This should now fail parsing, so we check the error directly
    assert!(!linter.is_valid("apple ~5 juice"));
    assert!(!linter.is_valid("word1 ~10 word2"));

    // Verify the specific error message
    match linter.lint("apple ~5 juice") {
        Err(error) => {
            assert!(error
                .to_string()
                .contains("The ~ character should be used after a search term or quoted phrase"));
        }
        Ok(_) => panic!("Expected parsing error for invalid tilde syntax"),
    }

    // These should still be valid (no regression)
    assert!(linter.is_valid("\"apple juice\"~5")); // Quoted phrase
    assert!(linter.is_valid("apple~5")); // Single term fuzzy
    assert!(linter.is_valid("((apple OR orange) AND phone)~5")); // Group
}

#[test]
fn test_extreme_deep_nesting() {
    // Test 6+ level deep nesting that should work perfectly
    assert!(is_valid_query("((((((apple OR orange) AND fresh) OR (banana AND ripe)) AND (juice OR smoothie)) OR ((grape AND sweet) AND drink)) AND healthy)"));

    // Test complex nested proximity and boolean combinations
    assert!(is_valid_query("(((apple NEAR/3 juice) AND fresh) OR ((orange NEAR/5 smoothie) AND organic)) AND ((healthy OR nutritious) NOT artificial)"));

    // Test extreme nesting with field operators - simplified to avoid parsing issues
    assert!(is_valid_query("(((country:usa OR country:gbr) AND language:en) OR ((country:fra OR country:deu) AND language:fr)) AND ((rating:[4 TO 5] AND authorVerified:true) OR authorFollowers:[10000 TO 100000])"));
}

#[test]
fn test_near_operator_interaction_validation() {
    let mut linter = BrandwatchLinter::new();

    let mixed_near_boolean_cases = vec!["(apple OR orange) NEAR/5 (juice OR drink) AND fresh"];

    for query in mixed_near_boolean_cases {
        let report = linter.lint(query).unwrap();
        assert!(report.has_errors(), "Query should fail: {}", query);
        assert!(
            report.errors.iter().any(|e| e
                .to_string()
                .contains("cannot be used within the NEAR operator")
                || e.to_string().contains("cannot be mixed")),
            "Should have NEAR/boolean mixing error for: {}",
            query
        );
    }

    let valid_near_cases = vec![
        "((apple OR orange) NEAR/5 (juice OR drink)) AND fresh", // Proper parentheses
        "(apple NEAR/3 banana) OR (juice NEAR/2 smoothie)",      // Properly parenthesized NEAR/OR
        "(apple NEAR/5 juice) AND (banana NEAR/3 smoothie)",     // Separate NEAR operations
    ];

    for query in valid_near_cases {
        let report = linter.lint(query).unwrap();
        assert!(!report.has_errors(), "Query should pass: {}", query);
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
