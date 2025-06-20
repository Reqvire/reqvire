#!/bin/bash

# Test: Element content extraction
# --------------------------------------
# Acceptance Criteria:
# - System should properly extract Requirement body for change impact detection
# - Requirement body is consiting of normalized main text and content from '#### Details subsection'
#
# Test Criteria:
# - Command exits with success (0) return code
# - Output shows expected content for each element
#




OUTPUT=$(cd "${TEST_DIR}" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" model-summary --json 2>&1)
EXIT_CODE=$?

printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"


GOTTEN_CONTENT=$(awk '/# [^:]+:/' "${TEST_DIR}/test_results.log")
GOTTEN_CONTENT=$(echo "$OUTPUT" | jq -r '
  [
    .files
    | .. 
    | objects 
    | select(.content != null)
    | (
        (.name + ":" + (.content | gsub("\n+"; " ")))
        | gsub("(^\\s+)|(\\s+$)"; "")
        + "\n"
      )
  ] | sort | .[]
')

GOTTEN_CONTENT=$(printf "\n%s" "$GOTTEN_CONTENT")


EXPECTED_CONTENT="
REQ 0:Root requirement for relations to work.

REQ 1:This is simple requirement with main text only.

REQ 1A:This is simple requirement with main text and details. REQ 1A details.

REQ 2:Requirement with main text and relations.

REQ 3:Requirement with main text and relations and metadata.

REQ 4:Requirement with main text and relations and metadata and details. REQ 4 Details.

REQ 5:Requirement with main text and relations and metadata and details different order (A). REQ 5 Details.

REQ 6:Requirement with main text and relations and metadata and details different order (B). REQ 6 Details.

REQ 7:Requirement with main text and relations and metadata and details different order (C). REQ 7 Details.

REQ 8:Requirement with main text and relations details with <details> element that should not break parsing and validation.    <details> ### REQ 8 Nested requirement which should not be processed as requirement. #### Relations   * derivedFrom: #req-0          </details>"



if ! diff <(echo "$EXPECTED_CONTENT") <(echo "$GOTTEN_CONTENT") > /dev/null; then
  echo "FAILED: Exctracted content not matching expected content."
  diff -u <(echo "$EXPECTED_CONTENT") <(echo "$GOTTEN_CONTENT")
  exit 1
fi

exit 0
