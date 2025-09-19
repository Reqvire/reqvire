#!/bin/bash

# Test: Format Command Requirements Verification
# This test verifies the format command requirements from SystemRequirements and UserRequirements

set -e

# Use the test directory provided by run_tests.sh
cd "$TEST_DIR"

# Copy original files to backup, then work on originals
# This ensures clean state for each test run
cp UserStories.md UserStories.md.original
cp -r SystemRequirements SystemRequirements.original
cp -r Verifications Verifications.original
cp MOEs.md MOEs.md.original

# Use the reqvire binary provided by test runner
REQVIRE="$REQVIRE_BIN"

# Test 1: Requirement - System shall provide format command and dry-run functionality
"$REQVIRE" format --dry-run > dry_run_output.txt 2>&1
DRY_RUN_EXIT_CODE=$?

if [ $DRY_RUN_EXIT_CODE -ne 0 ]; then
    echo "FAIL: Format command failed with exit code $DRY_RUN_EXIT_CODE"
    exit 1
fi

# Test 1.1: Verify dry-run shows expected changes
# Remove ANSI color codes for easier text matching
sed 's/\x1b\[[0-9;]*m//g' dry_run_output.txt > dry_run_clean.txt

EXPECTED_LINK_CONVERSION="\[MOE_UA\](MOEs.md#moe_ua)"
EXPECTED_SIMPLE_ID_CONVERSION="\[Requirements Processing\](SystemRequirements/Requirements.md#requirements-processing)"
EXPECTED_SEPARATOR_ADDITION="\+.*---"

if ! grep -q "$EXPECTED_LINK_CONVERSION" dry_run_clean.txt; then
    echo "FAIL: Dry run does not show absolute link conversion to relative"
    exit 1
fi

if ! grep -q "$EXPECTED_SIMPLE_ID_CONVERSION" dry_run_clean.txt; then
    echo "FAIL: Dry run does not show simple identifier conversion to markdown link"
    exit 1
fi

if ! grep -q "$EXPECTED_SEPARATOR_ADDITION" dry_run_clean.txt; then
    echo "FAIL: Dry run does not show separator addition"
    exit 1
fi

# Test 2: Apply formatting changes
"$REQVIRE" format > format_output.txt 2>&1
FORMAT_EXIT_CODE=$?

if [ $FORMAT_EXIT_CODE -ne 0 ]; then
    echo "FAIL: Format command execution failed with exit code $FORMAT_EXIT_CODE"
    exit 1
fi

# Test 3: Verify expected changes were applied with exact content matching

# Test 3.1: Check that absolute links were converted to relative
EXPECTED_RELATIVE_LINK="\[MOE_UA\](MOEs.md#moe_ua)"
if ! grep -q "$EXPECTED_RELATIVE_LINK" UserStories.md; then
    echo "FAIL: Absolute links not converted to relative"
    exit 1
fi

# Test 3.2: Check that simple identifiers were converted to markdown links with readable names
EXPECTED_MARKDOWN_LINK="\[Requirements Processing\](SystemRequirements/Requirements.md#requirements-processing)"
if ! grep -q "$EXPECTED_MARKDOWN_LINK" UserStories.md; then
    echo "FAIL: Simple identifiers not converted to markdown links with readable names"
    exit 1
fi

# Test 3.3: Check that separators were added where missing
if ! grep -A 4 "derive: \[Requirements Processing\]" UserStories.md | grep -q "^---$"; then
    echo "FAIL: Element separators not added"
    exit 1
fi

# Test 3.4: Check that personas section is preserved exactly
EXPECTED_PERSONA="System Engineer"
if ! grep -A 5 "## Personas" UserStories.md | grep -q "$EXPECTED_PERSONA"; then
    echo "FAIL: Personas section not preserved"
    exit 1
fi

# Test 3.5: Check that user-requirement metadata is preserved/added
EXPECTED_USER_REQ_METADATA="type: user-requirement"
if ! grep -q "$EXPECTED_USER_REQ_METADATA" UserStories.md; then
    echo "FAIL: user-requirement metadata not preserved/added"
    exit 1
fi

# Test 3.6: Check that consecutive separators are normalized to single separator
# Look for any case where a separator is immediately followed by another separator
if grep -A 1 "^---$" UserStories.md | grep -A 1 "^$" | grep -q "^---$"; then
    echo "FAIL: Consecutive separators not normalized to single separator"
    exit 1
fi

# Test 3.7: Check that relation indentation is normalized to 2 spaces
if grep -q "^\\* " UserStories.md; then
    echo "FAIL: Relations not properly indented with 2 spaces"
    exit 1
fi

# Test 4: Verify no additional changes needed (idempotent behavior)
"$REQVIRE" format --dry-run > no_changes_output.txt 2>&1
NO_CHANGES_EXIT_CODE=$?

if [ $NO_CHANGES_EXIT_CODE -ne 0 ]; then
    echo "FAIL: Second dry-run failed with exit code $NO_CHANGES_EXIT_CODE"
    exit 1
fi

EXPECTED_NO_CHANGES_MSG="No formatting changes needed"
if ! grep -q "$EXPECTED_NO_CHANGES_MSG" no_changes_output.txt; then
    echo "FAIL: Format command should report no changes needed after formatting"
    exit 1
fi

# Test 5: Verify diff output format contains expected elements
if ! grep -q "UserStories.md" dry_run_clean.txt; then
    echo "FAIL: Dry run output missing UserStories.md filename"
    exit 1
fi

if ! grep -q "+" dry_run_clean.txt; then
    echo "FAIL: Dry run output missing + addition markers"
    exit 1
fi

if ! grep -q "\-" dry_run_clean.txt; then
    echo "FAIL: Dry run output missing - removal markers"
    exit 1
fi

# Test 6: Verify line numbering is sequential and consistent
# Extract line numbers from diff output and check for proper sequencing
DIFF_LINE_NUMBERS=$(grep -E "^  [0-9]+" dry_run_clean.txt | sed 's/^  *//' | sed 's/ .*//' | tr '\n' ' ')

# Check that we have some line numbers
if [ -z "$DIFF_LINE_NUMBERS" ]; then
    echo "FAIL: No line numbers found in diff output"
    exit 1
fi

# For each file section, verify line numbers make sense
# Look for patterns like: X + Y + Z where Z > Y > X (for additions)
# or where removed lines don't break the sequence of final file positions
if grep -A 20 "UserStories.md" dry_run_clean.txt | grep -E "^  [0-9]+ \+" | head -2 | tail -1 | grep -q "019"; then
    # After adding lines 17,18,19, the next context line should be ~19 or higher
    NEXT_CONTEXT=$(grep -A 25 "UserStories.md" dry_run_clean.txt | grep -E "^  [0-9]+     " | head -1 | sed 's/^  *//' | sed 's/ .*//')
    if [ "$NEXT_CONTEXT" -lt 19 ]; then
        echo "FAIL: Line numbering not sequential - expected line $NEXT_CONTEXT to be >= 19 after additions"
        exit 1
    fi
fi

# Clean up: Restore original files for next run
mv UserStories.md.original UserStories.md
mv SystemRequirements.original SystemRequirements
mv Verifications.original Verifications
mv MOEs.md.original MOEs.md
