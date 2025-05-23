#!/bin/bash

# Test: Validation of Same-File Fragment Relations
# -----------------------------------------------
# Acceptance Criteria:
# - System should correctly validate relations to fragments within the same file
# - System should not report errors for valid fragment references
#
# Test Criteria:
# - Command exits with success (zero) return code
# - No error output about missing relation targets when using #fragment references
#
# Test Implementation for Verification:
# - Tests requirements with fragment-only references like "#fragment-id"
# - Tests fragments referenced by proper element ID

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


OUTPUT=$(cd "$TMP_DIR" && "$REQVIRE_BIN"  --config "${TMP_DIR}/reqvire.yaml"  --validate --json 2>&1)
EXIT_CODE=$?


printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"

# Verify exit code indicates success (0)
if [ $EXIT_CODE -ne 0 ]; then
  echo "FAILED: Validation should have succeeded but returned error ($EXIT_CODE)"
  echo "Output: $OUTPUT"
  exit 1
fi

# Verify that successful validation message is displayed
if ! echo "$OUTPUT" | grep -q "Validation completed successfully with no errors."; then
  echo "FAILED: Missing success message for relation validation"
  exit 1
fi

exit 0
