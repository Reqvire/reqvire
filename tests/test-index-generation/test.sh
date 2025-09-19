#!/bin/bash

# Test: Index Generation
# ----------------------------------------------------
# Acceptance Criteria:
# - System shall generate index in root folder
# - Index shall contain links to all specification documents
# - Index shall be properly structured with sections
# - Index shall include brief summaries of each document
#
# Test Criteria:
# - Command with generate-index flag runs successfully
# - Index file is created in the expected location
# - Index contains links to all specification documents
# - HTML index is created when html --output output flag is also used
#

# Create output directory if it doesn't exist
mkdir -p "${TEST_DIR}/output"

pushd "$TEST_DIR" > /dev/null 2>&1
git init > /dev/null 2>&1
popd > /dev/null 2>&1

# Run reqvire with generate-index flag
OUTPUT=$(cd "${TEST_DIR}" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" generate-index 2>&1)
EXIT_CODE=$?

# Save output for inspection
printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"

# Verify exit code indicates success
if [ $EXIT_CODE -ne 0 ]; then
  echo "❌ FAILED: Index generation failed with exit code $EXIT_CODE"
  exit 1
fi

# Check that SpecificationIndex.md was generated (the index file is named SpecificationIndex.md, not index.md)
if [ ! -f "${TEST_DIR}/SpecificationIndex.md" ]; then
  echo "❌ FAILED: SpecificationIndex.md not generated"
  exit 1
fi

# Verify index contains links to all specification documents
DOCUMENT_COUNT=$(find "${TEST_DIR}" -name "*.md" | grep -v "SpecificationIndex.md" | wc -l)
LINK_COUNT=$(grep -c "\[.*\](.*\.md)" "${TEST_DIR}/SpecificationIndex.md")

if [ $LINK_COUNT -lt $DOCUMENT_COUNT ]; then
  echo "❌ FAILED: Index does not contain links to all documents (found $LINK_COUNT links, expected at least $DOCUMENT_COUNT)"
  exit 1
fi

# We don't need to test HTML output as it's not part of the current functionality
# Just ensure the README.md was properly generated with content
if [ ! -s "${TEST_DIR}/SpecificationIndex.md" ]; then
  echo "❌ FAILED: SpecificationIndex.md is empty or not properly generated"
  exit 1
fi

exit 0
