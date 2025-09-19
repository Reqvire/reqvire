#!/bin/bash

# Test: Validation of Valid Relation Targets
# -----------------------------------------
# Acceptance Criteria:
# - System should correctly validate relations to existing targets in both standard and markdown formats
# - System should not report errors for valid relations
#
# Test Criteria:
# - Command exits with success (zero) return code
# - No error output about missing relation targets
#

# Create a unique temporary directory
TMP_DIR=$(mktemp -d -t reqvire-change-impact-XXXXXX)
cp -a "${TEST_DIR}/." "${TMP_DIR}/"
mkdir -p "${TMP_DIR}/output"

# Create simple git repository to test changes
cd "${TMP_DIR}"
git init > /dev/null 2>&1
git config --local user.email "test@example.com" > /dev/null 2>&1 
git config --local user.name "Test User" > /dev/null 2>&1
git remote add origin 'https://dummy.example.com/dummy-repo.git'  > /dev/null 2>&1
git add Requirements.md > /dev/null 2>&1
git commit -m "Initial commit" > /dev/null 2>&1



OUTPUT=$(cd "$TMP_DIR" && "$REQVIRE_BIN" --config "${TMP_DIR}/reqvire.yaml" model-summary --json 2>&1)
EXIT_CODE=$?

printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"


# Verify exit code indicates success (0)
if [ $EXIT_CODE -ne 0 ]; then
  echo "FAILED: Validation should have succeeded but returned error ($EXIT_CODE)"
  echo "Output: $OUTPUT"
  exit 1
fi


# Verify that JSON output is valid (indicates successful validation)
if ! echo "$OUTPUT" | jq . > /dev/null 2>&1; then
  echo "FAILED: Invalid JSON output, validation likely failed"
  echo "Output: $OUTPUT"
  exit 1
fi

exit 0
