#!/bin/bash

# Test: Two-Pass Validation of Invalid Relations
# -----------------------------------------------
# This test validates the two-pass validation architecture:
# - Pass 1: Element collection and local validation (parsing/format errors)
# - Pass 2: Graph construction and relation validation (relation errors)
#
# The test runs two separate scenarios to validate each pass independently.

# Test 1: Pass 1 Errors (Parsing/Format Issues)

OUTPUT_PASS1=$(cd "${TEST_DIR}/pass1-errors" && "$REQVIRE_BIN" model-summary 2>&1)
EXIT_CODE_PASS1=$?

printf "%s\n" "$OUTPUT_PASS1" > "${TEST_DIR}/test_results_pass1.log"

# Verify exit code indicates validation failure (non-zero)
if [ $EXIT_CODE_PASS1 -eq 0 ]; then
  echo "❌ FAILED: Pass 1 validation should have failed but returned success (0)"
  exit 1
fi

# Define Pass 1 errors we expect to see
EXPECTED_PASS1_ERRORS=(
  "Duplicate element:"
  "Invalid metadata format:"
  "Invalid relation format:"
  "Unsupported relation type:"
  "Duplicate subsection:"
)

MISSING_PASS1_ERRORS=()

for expected in "${EXPECTED_PASS1_ERRORS[@]}"; do
  if ! echo "$OUTPUT_PASS1" | grep -q "$expected"; then
    MISSING_PASS1_ERRORS+=("❌ MISSING Pass 1: $expected")
  fi
done

if [ ${#MISSING_PASS1_ERRORS[@]} -gt 0 ]; then
  echo "❌ FAILED: Missing expected Pass 1 validation errors!"
  for missing in "${MISSING_PASS1_ERRORS[@]}"; do
    echo "$missing"
  done
  echo ""
  echo "ACTUAL Pass 1 ERRORS:"
  echo "$OUTPUT_PASS1"
  exit 1
fi

# Test 2: Pass 2 Errors (Relation Validation Issues)

OUTPUT_PASS2=$(cd "${TEST_DIR}/pass2-errors" && "$REQVIRE_BIN" model-summary 2>&1)
EXIT_CODE_PASS2=$?

printf "%s\n" "$OUTPUT_PASS2" > "${TEST_DIR}/test_results_pass2.log"

# Verify exit code indicates validation failure (non-zero)
if [ $EXIT_CODE_PASS2 -eq 0 ]; then
  echo "❌ FAILED: Pass 2 validation should have failed but returned success (0)"
  exit 1
fi

# Define Pass 2 errors we expect to see
EXPECTED_PASS2_ERRORS=(
  "Missing relation target:"
  "Incompatible element types for relation:"
  "Circular dependency error:"
)

MISSING_PASS2_ERRORS=()

for expected in "${EXPECTED_PASS2_ERRORS[@]}"; do
  if ! echo "$OUTPUT_PASS2" | grep -q "$expected"; then
    MISSING_PASS2_ERRORS+=("❌ MISSING Pass 2: $expected")
  fi
done

if [ ${#MISSING_PASS2_ERRORS[@]} -gt 0 ]; then
  echo "❌ FAILED: Missing expected Pass 2 validation errors!"
  for missing in "${MISSING_PASS2_ERRORS[@]}"; do
    echo "$missing"
  done
  echo ""
  echo "ACTUAL Pass 2 ERRORS:"
  echo "$OUTPUT_PASS2"
  exit 1
fi


exit 0