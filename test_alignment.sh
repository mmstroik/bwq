#!/bin/bash

# Test alignment between our linter and Brandwatch API
# Usage: ./test_alignment.sh

echo "=== Brandwatch Query Linter vs API Comparison ==="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

test_count=0
mismatches=0

test_query() {
    local expected="$1"
    local query="$2"
    local description="$3"
    
    test_count=$((test_count + 1))
    
    echo "Test $test_count: $description"
    echo "Query: $query"
    
    # Test our linter
    if cargo run --quiet -- validate "$query" >/dev/null 2>&1; then
        our_result="valid"
    else
        our_result="invalid"
    fi
    
    # Test Brandwatch API (escape quotes properly)
    escaped_query=$(echo "$query" | sed 's/"/\\"/g')
    api_response=$(curl -s -X POST https://api.brandwatch.com/query-validation \
        -H "authorization: bearer $BW_API_KEY" \
        -H 'Content-Type: application/json' \
        -d "{\"booleanQuery\": \"$escaped_query\",\"languages\": []}")
    
    if echo "$api_response" | grep -q '"errors":\[\]'; then
        api_result="valid"
    else
        api_result="invalid"
    fi
    
    echo "Our linter: $our_result"
    echo "BW API: $api_result"
    
    if [ "$our_result" = "$api_result" ]; then
        echo -e "${GREEN}✓ MATCH${NC}"
    else
        echo -e "${RED}✗ MISMATCH${NC}"
        mismatches=$((mismatches + 1))
        
        # Show API error details for mismatches
        if [ "$api_result" = "invalid" ]; then
            echo "API errors:"
            echo "$api_response" | grep -o '"errors":\[[^]]*\]' | sed 's/"errors":\[//; s/\]$//' | tr ',' '\n'
        fi
    fi
    
    echo "---"
}

# Run specific edge case tests
echo "Testing critical edge cases..."

test_query "invalid" "NOT bitter" "Pure negative query"
test_query "invalid" "apple AND" "Incomplete AND"
test_query "invalid" "OR apple" "Missing left operand"
test_query "valid" "apple" "Simple term"
test_query "valid" "apple AND juice" "Basic AND"
test_query "valid" "apple NEAR/5 juice" "NEAR operator"
test_query "invalid" "apple AND juice NEAR/5 orange" "Mixed AND with NEAR"
test_query "valid" "(apple NEAR/5 juice) AND orange" "Parenthesized NEAR with AND"
test_query "valid" "appl*" "Valid wildcard"
test_query "valid" "\"apple juice\"" "Quoted phrase"
test_query "valid" "title:\"apple juice\"" "Field operation"
test_query "valid" "{BrandWatch}" "Case sensitive"

# Test the user's real-world query
test_query "valid" "competition OR competitive OR competitor* OR saturat* OR ((China OR chinese OR cheap OR \"low cost\" OR \"low-cost\") NEAR/3 (brand* OR product* OR seller))" "User's real-world query"

echo ""
echo "=== Summary ==="
echo "Total tests: $test_count"
echo "Matches: $((test_count - mismatches))"
echo "Mismatches: $mismatches"

if [ $mismatches -eq 0 ]; then
    echo -e "${GREEN}All tests passed! Perfect alignment with Brandwatch API.${NC}"
else
    echo -e "${YELLOW}$mismatches mismatches found. Review validation logic.${NC}"
fi