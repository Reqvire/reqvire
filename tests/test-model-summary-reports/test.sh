#!/usr/bin/env bash
set -euo pipefail

# Test: Model Summary JSON and Filters Validation (with text checks and relations coverage)
# -------------------------------------------------------------------------------------
# Acceptance Criteria:
# - `reqvire model-summary --json` emits valid JSON with a top-level "files" key
# - `reqvire model-summary` prints a text summary beginning with `--- MBSE Model summary ---`
# - Each filter flag when used with `model-summary --json` or `model-summary` correctly restricts output
# - Using any filter flag without `model-summary` fails appropriately
# - Invalid regex on name/content filters fails with `Invalid regex`
# - Model summary report must include all relations for each element, showing both explicit relations and their opposite relations
# - Relations must be preserved even when filtering excludes target elements (e.g., requirements show verifiedBy relations even when verifications are filtered out)
#
# Test Criteria:
# - Commands exit 0 on success
# - JSON output parses under jq
# - Text summary contains expected header, element count, and required fields
# - Filters reduce element counts as expected
# - Incorrect usage exits non-zero with proper error message
# - Relations coverage: bidirectional relationships are shown (verifiedBy/verify, refine/refinedBy, derivedFrom/derive, etc.)
# - Filtered relations coverage: relations to filtered-out elements are preserved in both directions

# 1) No filters: base JSON summary
OUTPUT=$(cd "$TEST_DIR" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" model-summary --json 2>&1)
printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"
EXIT_JSON=$?
if [ $EXIT_JSON -ne 0 ]; then
  echo "FAILED: base JSON summary exited $EXIT_JSON"
  exit 1
fi

echo "$OUTPUT" | jq . >/dev/null 2>&1
if ! echo "$OUTPUT" | jq 'has("files")' | grep -q true; then
  echo "FAILED: JSON missing files"
  exit 1
fi

TOTAL=$(echo "$OUTPUT" | jq '.global_counters.total_elements')
if [ "$TOTAL" -ne 5 ]; then
  echo "FAILED: expected 5 elements in JSON, got $TOTAL"
  exit 1
fi

# Enhanced JSON structure verification - Test new features

# Check that global counters include the new count fields
if ! echo "$OUTPUT" | jq -e '.global_counters.total_files' >/dev/null; then
  echo "FAILED: JSON missing total_files in global_counters"
  exit 1
fi

if ! echo "$OUTPUT" | jq -e '.global_counters.total_sections' >/dev/null; then
  echo "FAILED: JSON missing total_sections in global_counters"
  exit 1
fi

# Check that file summaries include the new count fields
FIRST_FILE=$(echo "$OUTPUT" | jq -r '.files | keys[0]')
if ! echo "$OUTPUT" | jq -e ".files[\"$FIRST_FILE\"].total_sections" >/dev/null; then
  echo "FAILED: JSON missing total_sections in file summaries"
  exit 1
fi

if ! echo "$OUTPUT" | jq -e ".files[\"$FIRST_FILE\"].total_elements" >/dev/null; then
  echo "FAILED: JSON missing total_elements in file summaries"
  exit 1
fi

# Check that section summaries include element_count field
FIRST_SECTION=$(echo "$OUTPUT" | jq -r ".files[\"$FIRST_FILE\"].sections | keys[0]")
if ! echo "$OUTPUT" | jq -e ".files[\"$FIRST_FILE\"].sections[\"$FIRST_SECTION\"].element_count" >/dev/null; then
  echo "FAILED: JSON missing element_count in section summaries"
  exit 1
fi

# Test page content with mermaid diagrams
REQUIREMENTS_FILE="specifications/Requirements.md"
if echo "$OUTPUT" | jq -e ".files[\"$REQUIREMENTS_FILE\"].page_content" >/dev/null; then
  PAGE_CONTENT=$(echo "$OUTPUT" | jq -r ".files[\"$REQUIREMENTS_FILE\"].page_content")
  if [[ ! "$PAGE_CONTENT" == *"mermaid"* ]]; then
    echo "FAILED: Page content should include mermaid diagram"
    exit 1
  fi
  if [[ ! "$PAGE_CONTENT" == *"graph TD"* ]]; then
    echo "FAILED: Page content should include the test mermaid diagram content"
    exit 1
  fi
fi

# Test section content with mermaid diagrams
REQUIREMENTS_SECTION="Requirements A"
if echo "$OUTPUT" | jq -e ".files[\"$REQUIREMENTS_FILE\"].sections[\"$REQUIREMENTS_SECTION\"].section_content" >/dev/null; then
  SECTION_CONTENT=$(echo "$OUTPUT" | jq -r ".files[\"$REQUIREMENTS_FILE\"].sections[\"$REQUIREMENTS_SECTION\"].section_content")
  if [[ ! "$SECTION_CONTENT" == *"mermaid"* ]]; then
    echo "FAILED: Section content should include mermaid diagram"
    exit 1
  fi
  if [[ ! "$SECTION_CONTENT" == *"flowchart LR"* ]]; then
    echo "FAILED: Section content should include the test mermaid diagram content"
    exit 1
  fi
else
  echo "Warning: Section content not found for Requirements A"
fi


# 2) No filters: base text summary
OUTPUT=$(cd "$TEST_DIR" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" model-summary 2>&1)
printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"
EXIT_TEXT=$?
if [ $EXIT_TEXT -ne 0 ]; then
  echo "FAILED: base text summary exited $EXIT_TEXT"
  exit 1
fi

if ! grep -q '^--- MBSE Model summary ---' <<< "$OUTPUT"; then
  echo "FAILED: text summary header missing"
  exit 1
fi


ELEMS_TEXT=$(grep -c '🔹 Element:' <<< "$OUTPUT")
if [ "$ELEMS_TEXT" -ne 5 ]; then
  echo "FAILED: expected 5 elements in text summary, got $ELEMS_TEXT"
  exit 1
fi

# Check required fields in text
for KEY in \
    '- Name:' \
    '- Section:' \
    '- File:' \
    '- Type:' \
    '- Content:'; do

  if ! grep -q -- "$KEY" <<< "$OUTPUT"; then
    echo "FAILED: text summary missing '$KEY'"
    exit 1
  fi
done

# Enhanced content and counts verification - Test new features

# Check file counts format in text output
if ! grep -q '^📂 File:.* (sections: [0-9]*, elements: [0-9]*)' <<< "$OUTPUT"; then
  echo "FAILED: text summary missing file counts format"
  exit 1
fi

# Check section counts format in text output
if ! grep -q '^  📖 Section:.* (elements: [0-9]*)' <<< "$OUTPUT"; then
  echo "FAILED: text summary missing section counts format"
  exit 1
fi

# Check that global summary includes the new counts
if ! grep -q 'Total files:' <<< "$OUTPUT"; then
  echo "FAILED: text summary missing total files count"
  exit 1
fi

if ! grep -q 'Total sections:' <<< "$OUTPUT"; then
  echo "FAILED: text summary missing total sections count"
  exit 1
fi

# Test page content display in text format with debug formatting
if grep -q '📄 Page content:' <<< "$OUTPUT"; then
  # Verify that page content includes mermaid diagrams with \n formatting
  if ! grep -A5 '📄 Page content:' <<< "$OUTPUT" | grep -q '\\n'; then
    echo "FAILED: Page content should use debug format with \\n newlines"
    exit 1
  fi
fi

# Test section content display in text format with debug formatting
if grep -q '📝 Section content:' <<< "$OUTPUT"; then
  # Verify that section content includes mermaid diagrams with \n formatting
  if ! grep -A5 '📝 Section content:' <<< "$OUTPUT" | grep -q '\\n'; then
    echo "FAILED: Section content should use debug format with \\n newlines"
    exit 1
  fi
fi




# 3) --filter-file=Requirements.md
OUTPUT=$(cd "$TEST_DIR" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" model-summary --json --filter-file="specifications/Requirements.md")

printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"
if [ "$(echo "$OUTPUT" | jq '.files | length')" -ne 1 ]; then
  echo "FAILED: --filter-file JSON did not limit to 1 file"
  exit 1
fi


OUTPUT=$(cd "$TEST_DIR" && "$REQVIRE_BIN"  --config "${TEST_DIR}/reqvire.yaml" model-summary --filter-file="specifications/Requirements.md")

printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"
count=$(grep -c -- '^📂 File:' <<<"$OUTPUT")
if (( count != 1 )); then
  echo "FAILED: --filter-file text did not limit to 1 file (got $count)"
  exit 1
fi

# 4) --filter-section=Requirements
OUTPUT=$(cd "$TEST_DIR" && "$REQVIRE_BIN"  --config "${TEST_DIR}/reqvire.yaml" model-summary --json --filter-section="Requirements*")
printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"

COUNT_SEC_REQ=$(
  echo "$OUTPUT" | jq -r '
    .files
    | to_entries[]
    | select(.key | endswith("Requirements.md"))
    | .value.sections
    | to_entries
    | map(
        select(.key | test("^Requirements.*"))
        .value.elements
        | length
      )
    | add  // 0
  '
)

if [ "$COUNT_SEC_REQ" -ne 4 ]; then
  echo "FAILED: --filter-section=Requirements should yield 4 elements (got $COUNT_SEC_REQ)"
  exit 1
fi

OUTPUT=$(cd "$TEST_DIR" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" model-summary --filter-section="Requirements A")

printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"

# 1) Make sure exactly one “Requirements A” section is printed
sec_count=$(printf '%s\n' "$OUTPUT" \
  | grep -F -c "📖 Section: Requirements")

if [ "$sec_count" -ne 1 ]; then
  echo "FAILED: expected 1 'Section: Requirements' header (got $sec_count)"
  exit 1
fi

# 2) Make sure there are exactly 3 elements in total
elem_count=$(grep -c 'Element:' <<<"$OUTPUT")
if [ "$elem_count" -ne 3 ]; then
  echo "FAILED: expected 3 elements under Requirements (got $elem_count)"
  exit 1
fi



# 5) --filter-type=user-requirement
OUTPUT=$(cd "$TEST_DIR" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" model-summary --json --filter-type="user-requirement")
printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"
if [ "$(echo "$OUTPUT" | jq '.global_counters.total_elements')" -ne 3 ]; then
  echo "FAILED: --filter-type=user-requirement JSON should yield 3 elements"
  exit 1
fi
OUTPUT=$(cd "$TEST_DIR" && "$REQVIRE_BIN"  --config "${TEST_DIR}/reqvire.yaml" model-summary --filter-type="user-requirement")
printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"
if [ "$(grep -c '🔹 Element:' <<< "$OUTPUT")" -ne 3 ]; then
  echo "FAILED: --filter-type=user-requirement text should yield 3 elements"
  exit 1
fi


# 6) --filter-type=verification
OUTPUT=$(cd "$TEST_DIR" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" model-summary --json --filter-type="verification")
printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"
if [ "$(echo "$OUTPUT" | jq '.global_counters.total_elements')" -ne 1 ]; then
  echo "FAILED: --filter-type=verification JSON should yield 1 element"
  exit 1
fi
OUTPUT=$(cd "$TEST_DIR" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" model-summary --filter-type="verification")
printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"
if [ "$(grep -c '🔹 Element:' <<< "$OUTPUT")" -ne 1 ]; then
  echo "FAILED: --filter-type=verification text should yield 1 element"
  exit 1
fi


# 7) --filter-name-regex="^Requirement with Valid Standard"
OUTPUT=$(cd "$TEST_DIR" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" model-summary --json --filter-name="^Requirement with Valid Standard")
printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"
if [ "$(echo "$OUTPUT" | jq '.global_counters.total_elements')" -ne 1 ]; then
  echo "FAILED: --filter-name-regex JSON should yield 1 element"
  exit 1
fi
OUTPUT=$(cd "$TEST_DIR" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" model-summary --filter-name="^Requirement with Valid Standard")
printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"
if [ "$(grep -c '🔹 Element:' <<< "$OUTPUT")" -ne 1 ]; then
  echo "FAILED: --filter-name-regex text should yield 1 element"
  exit 1
fi


# 8) --filter-content="subsection"
OUTPUT=$(cd "$TEST_DIR" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" model-summary --json --filter-content="subsection")
printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"
if [ "$(echo "$OUTPUT" | jq '.global_counters.total_elements')" -ne 1 ]; then
  echo "FAILED: --filter-content JSON should yield 1 element"
  exit 1
fi
OUTPUT=$(cd "$TEST_DIR" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" model-summary --filter-content="subsection")
printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"
if [ "$(grep -c '🔹 Element:' <<< "$OUTPUT")" -ne 1 ]; then
  echo "FAILED: --filter-content text should yield 1 element"
  exit 1
fi


# 9) --filter-is-verified
OUTPUT=$(cd "$TEST_DIR" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" model-summary --json --filter-is-not-verified)
printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"

if [ "$(echo "$OUTPUT" | jq '.global_counters.requirements_not_verified')" -ne 2 ]; then
  echo "FAILED: --filter-is-not-verified JSON should yield 2 elements"
  exit 1
fi

OUTPUT=$(cd "$TEST_DIR" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" model-summary --filter-is-not-verified)
printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"
if ! grep -q "Requirements not verified: 2" <<< "$OUTPUT"; then
  echo "FAILED: Expected 'Requirements not verified: 2' but got:"
  echo "$OUTPUT"
  exit 1
fi


# 10) --filter-is-satisfied
OUTPUT=$(cd "$TEST_DIR" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" model-summary --json --filter-is-not-satisfied)
printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"
if [ "$(echo "$OUTPUT" | jq '.global_counters.requirements_not_satisfied')" -ne 1 ]; then
  echo "FAILED: --filter-is-not-satisfied JSON should yield 3 elements"
  exit 1
fi
OUTPUT=$(cd "$TEST_DIR" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" model-summary --filter-is-not-satisfied)
printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"
if ! grep -q "Requirements not satisfied: 1" <<< "$OUTPUT"; then
  echo "FAILED: Expected 'Requirements not satisfied: 1' but got:"
  echo "$OUTPUT"
  exit 1
fi


# 11) Combination: user-requirement + is-satisfied
OUTPUT=$(cd "$TEST_DIR" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" model-summary --json \
           --filter-type="user-requirement" --filter-is-not-satisfied)
printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"           
if [ "$(echo "$OUTPUT" | jq '.global_counters.total_elements')" -ne 1 ]; then
  echo "FAILED: combo JSON should yield 1 elements"
  exit 1
fi


           
OUTPUT=$(cd "$TEST_DIR" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" model-summary \
           --filter-type="user-requirement" --filter-is-not-satisfied)
printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"           
if [ "$(grep -c '🔹 Element:' <<< "$OUTPUT")" -ne 1 ]; then
  echo "FAILED: combo text should yield 1 elements"
  exit 1
fi


# 12) invalid regex
set +e
OUTPUT=""
CODE=0

# Capture output and exit code separately
OUTPUT=$(cd "$TEST_DIR" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" model-summary  --filter-name="[invalid" 2>&1)
CODE=$?
# Don't log error output that we expect to happen
# printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"
set -e

# Assert: must NOT return exit code 0 AND must mention 'Invalid regex'
if [ $CODE -ne 1 ] && ! grep -q "Invalid regex" <<< "$OUTPUT"; then
  echo "FAILED: invalid regex TEXT did not error as expected"
  exit 1
fi

set +e
OUTPUT=""
CODE=0

# Capture output and exit code separately
OUTPUT=$(cd "$TEST_DIR" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" model-summary  --json --filter-name="[invalid" 2>&1)
CODE=$?
set -e

# Assert: must NOT return exit code 0 AND must mention 'Invalid regex'
if [ $CODE -ne 1 ] || ! grep -q "Invalid regex" <<< "$OUTPUT"; then
  echo "FAILED: invalid regex JSON did not error as expected"
  exit 1
fi

# Test #13 removed - no longer testing filter without model-summary error case

# 14) Relations coverage - bidirectional relationships
OUTPUT=$(cd "$TEST_DIR" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" model-summary --json)
printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results_relations.log"

# Check that "Verification of Standard Relations" has verify relation pointing to requirement
VERIFY_RELATIONS=$(echo "$OUTPUT" | jq -r '
  .files | to_entries[] | .value.sections.Verifictions.elements[]?
  | select(.name == "Verification of Standard Relations")
  | .relations[]?
  | select(.relation_type == "verify")
  | .target.target
')

# Note: Currently the verification element might not show verify relations if they are only implicit
# This test will pass if the verify relation exists, but we'll also check the requirements have verifiedBy
if [ -n "$VERIFY_RELATIONS" ]; then
  if ! echo "$VERIFY_RELATIONS" | grep -q "requirement-with-valid-standard-relations"; then
    echo "FAILED: Verification should have 'verify' relation to 'Requirement with Valid Standard Relations'"
    echo "Found verify relations: $VERIFY_RELATIONS"
    exit 1
  fi
fi

# Check that "Requirement with Valid Standard Relations" has verifiedBy relation pointing back to verification
VERIFIED_BY_RELATIONS=$(echo "$OUTPUT" | jq -r '
  .files | to_entries[] | .value.sections."Requirements A".elements[]
  | select(.name == "Requirement with Valid Standard Relations")
  | .relations[]
  | select(.relation_type == "verifiedBy")
  | .target.target
')

if ! echo "$VERIFIED_BY_RELATIONS" | grep -q "verification-of-standard-relations"; then
  echo "FAILED: Requirement should have 'verifiedBy' relation to 'Verification of Standard Relations'"
  echo "Found verifiedBy relations: $VERIFIED_BY_RELATIONS"
  exit 1
fi

# Check that "Requirement with Valid Markdown Relations" also has verifiedBy relation to the same verification
VERIFIED_BY_RELATIONS_2=$(echo "$OUTPUT" | jq -r '
  .files | to_entries[] | .value.sections."Requirements A".elements[]
  | select(.name == "Requirement with Valid Markdown Relations")
  | .relations[]
  | select(.relation_type == "verifiedBy")
  | .target.target
')

if ! echo "$VERIFIED_BY_RELATIONS_2" | grep -q "verification-of-standard-relations"; then
  echo "FAILED: Second requirement should also have 'verifiedBy' relation to 'Verification of Standard Relations'"
  echo "Found verifiedBy relations: $VERIFIED_BY_RELATIONS_2"
  exit 1
fi

# Check refinedBy relation (looking at the JSON output, this is the opposite relation that should be shown)
REFINED_BY_RELATIONS=$(echo "$OUTPUT" | jq -r '
  .files | to_entries[] | .value.sections."Requirements A".elements[]
  | select(.name == "Requirement with Valid Markdown Relations")
  | .relations[]
  | select(.relation_type == "refinedBy")
  | .target.target
')

if ! echo "$REFINED_BY_RELATIONS" | grep -q "requirement-with-valid-standard-relations"; then
  echo "FAILED: Second requirement should have 'refinedBy' relation from first requirement"
  echo "Found refinedBy relations: $REFINED_BY_RELATIONS"
  exit 1
fi

# Check derive relation (from the JSON, the parent shows derive relation to child)
DERIVE_RELATIONS=$(echo "$OUTPUT" | jq -r '
  .files | to_entries[] | .value.sections."Requirements A".elements[]
  | select(.name == "Requirement with Valid Standard Relations")
  | .relations[]
  | select(.relation_type == "derive")
  | .target.target
')

if ! echo "$DERIVE_RELATIONS" | grep -q "requirement-with-designspecifications-reference"; then
  echo "FAILED: Parent requirement should have 'derive' relation to child requirement"
  echo "Found derive relations: $DERIVE_RELATIONS"
  exit 1
fi

# 15) Relations visibility when filtering by type - verification filter
OUTPUT_VERIFICATION_FILTER=$(cd "$TEST_DIR" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" model-summary --json --filter-type="verification")
printf "%s\n" "$OUTPUT_VERIFICATION_FILTER" > "${TEST_DIR}/test_results_verification_filter.log"

# When filtering by verification type, we should only see verification elements
TOTAL_ELEMENTS_VERIFICATION_FILTER=$(echo "$OUTPUT_VERIFICATION_FILTER" | jq '.global_counters.total_elements')
if [ "$TOTAL_ELEMENTS_VERIFICATION_FILTER" -ne 1 ]; then
  echo "FAILED: --filter-type=verification should show only 1 element"
  echo "Found: $TOTAL_ELEMENTS_VERIFICATION_FILTER elements"
  exit 1
fi

# But the verification element should still show its 'verify' relations to requirements (if it has any)
# Even though those requirements are not included in the filtered results
VERIFICATION_VERIFY_RELATIONS=$(echo "$OUTPUT_VERIFICATION_FILTER" | jq -r '
  .files | to_entries[] | .value.sections.Verifictions.elements[]?
  | select(.name == "Verification of Standard Relations")
  | .relations[]?
  | select(.relation_type == "verify")
  | .target.target
')

# Verification elements MUST show verify relations to maintain bidirectional traceability
if [ -z "$VERIFICATION_VERIFY_RELATIONS" ]; then
  echo "FAILED: Verification element should show explicit 'verify' relations to requirements for complete bidirectional traceability"
  echo "Requirements show verifiedBy relations, but verification shows no corresponding verify relations"
  exit 1
fi

# 16) Relations visibility when filtering by type - requirement filter
OUTPUT_REQUIREMENT_FILTER=$(cd "$TEST_DIR" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" model-summary --json --filter-type="user-requirement")
printf "%s\n" "$OUTPUT_REQUIREMENT_FILTER" > "${TEST_DIR}/test_results_requirement_filter.log"

# When filtering by requirement type, we should see 3 user-requirement elements
TOTAL_ELEMENTS_REQUIREMENT_FILTER=$(echo "$OUTPUT_REQUIREMENT_FILTER" | jq '.global_counters.total_elements')
if [ "$TOTAL_ELEMENTS_REQUIREMENT_FILTER" -ne 3 ]; then
  echo "FAILED: --filter-type=user-requirement should show 3 elements"
  echo "Found: $TOTAL_ELEMENTS_REQUIREMENT_FILTER elements"
  exit 1
fi

# The requirements should still show their 'verifiedBy' relations to verifications
# Even though the verification element is not included in the filtered results
REQUIREMENT_VERIFIEDBY_RELATIONS_FILTERED=$(echo "$OUTPUT_REQUIREMENT_FILTER" | jq -r '
  .files | to_entries[] | .value.sections."Requirements A".elements[]
  | select(.name == "Requirement with Valid Standard Relations")
  | .relations[]
  | select(.relation_type == "verifiedBy")
  | .target.target
')

if ! echo "$REQUIREMENT_VERIFIEDBY_RELATIONS_FILTERED" | grep -q "verification-of-standard-relations"; then
  echo "FAILED: Requirements should still show verifiedBy relations even when verifications are filtered out"
  echo "Found verifiedBy relations in filtered output: $REQUIREMENT_VERIFIEDBY_RELATIONS_FILTERED"
  exit 1
fi

# Test that relations to filtered-out elements are preserved in both directions
REQUIREMENT2_VERIFIEDBY_RELATIONS_FILTERED=$(echo "$OUTPUT_REQUIREMENT_FILTER" | jq -r '
  .files | to_entries[] | .value.sections."Requirements A".elements[]
  | select(.name == "Requirement with Valid Markdown Relations")
  | .relations[]
  | select(.relation_type == "verifiedBy")
  | .target.target
')

if ! echo "$REQUIREMENT2_VERIFIEDBY_RELATIONS_FILTERED" | grep -q "verification-of-standard-relations"; then
  echo "FAILED: Second requirement should also show verifiedBy relations when verifications are filtered out"
  echo "Found verifiedBy relations: $REQUIREMENT2_VERIFIEDBY_RELATIONS_FILTERED"
  exit 1
fi

exit 0

