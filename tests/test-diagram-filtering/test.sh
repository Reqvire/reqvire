#!/bin/bash

# Test: Diagram Relation Filtering
# ---------------------------------
# Acceptance Criteria:
# - System should render only forward relations to prevent duplicate arrows
# - System should include parent elements in diagrams even when they belong to different sections
# - System should apply direction-based rendering according to relation type registry
# - Generated diagrams should not contain both forward and backward relations for the same logical relationship
#
# Test Criteria:
# - Command exits with success (0) return code
# - Diagrams contain only forward relations (e.g., `contain` but not `containedBy`)
# - Bidirectional relationships appear as single arrows in their forward direction
# - Parent elements are included when child elements are in the section
# - No duplicate arrows exist for the same logical relationship
# - Arrow directions follow the semantic direction defined in relation type registry

# Create a reqvire.yaml configuration
cat > "$TEST_DIR/reqvire.yaml" << EOF
paths:
  output_folder: "output"
  excluded_filename_patterns: []
style:
  theme: "default"
  max_width: 1200
  diagram_direction: "LR"
EOF

# Make backup copies of original files for comparison
mkdir -p "$TEST_DIR/backup"
cp -r "$TEST_DIR/specifications" "$TEST_DIR/backup/"

# Run reqvire to generate diagrams
OUTPUT=$(cd "$TEST_DIR" && "$REQVIRE_BIN" --config "$TEST_DIR/reqvire.yaml" generate-diagrams 2>&1)
EXIT_CODE=$?

# Save output to log
printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"

# Check for basic success
if [ $EXIT_CODE -ne 0 ]; then
  echo "❌ FAILED: Diagram generation command returned error: $EXIT_CODE"
  cat "${TEST_DIR}/test_results.log"
  exit 1
fi

# Check that the test file was modified and contains mermaid diagrams
TEST_FILE="$TEST_DIR/specifications/RelationFilteringTest.md"
BACKUP_FILE="$TEST_DIR/backup/specifications/RelationFilteringTest.md"

if cmp -s "$TEST_FILE" "$BACKUP_FILE"; then
  echo "❌ NOT MODIFIED: Expected test file to be modified with diagrams"
  exit 1
fi

if ! grep -q '```mermaid' "$TEST_FILE"; then
  echo "❌ NO DIAGRAM: Expected test file to contain mermaid diagrams"
  exit 1
fi

# Test 1: Forward relations should be present
FAILED_CHECKS=0

# Check that forward relations are rendered
if ! grep -q -- "--o|contains" "$TEST_FILE"; then
  echo "❌ MISSING FORWARD: Expected 'contain' forward relation arrow"
  FAILED_CHECKS=$((FAILED_CHECKS + 1))
fi

if ! grep -q -- "-.->|derive" "$TEST_FILE"; then
  echo "❌ MISSING FORWARD: Expected 'derive' forward relation arrow"
  FAILED_CHECKS=$((FAILED_CHECKS + 1))
fi

if ! grep -q -- "-->|satisfiedBy" "$TEST_FILE"; then
  echo "❌ MISSING FORWARD: Expected 'satisfiedBy' forward relation arrow"
  FAILED_CHECKS=$((FAILED_CHECKS + 1))
fi

if ! grep -q -- "-.->|verifiedBy" "$TEST_FILE"; then
  echo "❌ MISSING FORWARD: Expected 'verifiedBy' forward relation arrow"
  FAILED_CHECKS=$((FAILED_CHECKS + 1))
fi

# Test 2: Backward relation names should NOT be present in Mermaid diagrams
# Extract only the Mermaid diagram sections
MERMAID_DIAGRAMS=$(sed -n '/```mermaid/,/```/p' "$TEST_FILE")

if echo "$MERMAID_DIAGRAMS" | grep -q -- "containedBy"; then
  echo "❌ BACKWARD PRESENT: Found 'containedBy' backward relation in Mermaid diagrams - should be filtered"
  FAILED_CHECKS=$((FAILED_CHECKS + 1))
fi

if echo "$MERMAID_DIAGRAMS" | grep -q -- "derivedFrom"; then
  echo "❌ BACKWARD PRESENT: Found 'derivedFrom' backward relation in Mermaid diagrams - should be filtered"
  FAILED_CHECKS=$((FAILED_CHECKS + 1))
fi

if echo "$MERMAID_DIAGRAMS" | grep -q -- "verify"; then
  echo "❌ BACKWARD PRESENT: Found 'verify' backward relation in Mermaid diagrams - should be filtered"
  FAILED_CHECKS=$((FAILED_CHECKS + 1))
fi

# Note: "refines" label is correct in diagrams - it's the semantic label used by both
# refine (Backward) and refinedBy (Forward). Since we only render Forward relations,
# seeing "refines" means we're showing the refinedBy relation correctly.

# Test 3: Parent elements should be included in child section diagrams
# The "Parent Element" should appear in the "Child Section" diagram
# even though it's defined in the "Parent Section"

# Extract the Child Section diagram
CHILD_SECTION_DIAGRAM=$(sed -n '/## Child Section/,/## /p' "$TEST_FILE" | sed -n '/```mermaid/,/```/p')

if ! echo "$CHILD_SECTION_DIAGRAM" | grep -q '"Parent Element"'; then
  echo "❌ MISSING HIERARCHY: Parent Element should be included in Child Section diagram"
  FAILED_CHECKS=$((FAILED_CHECKS + 1))
fi

# Test 4: Check that the contain relation is present in Child Section
# The contain relation should flow from Parent Element to Child Element
if ! echo "$CHILD_SECTION_DIAGRAM" | grep -q -- "--o|contains|"; then
  echo "❌ MISSING CONTAIN: 'contain' relation should be present in Child Section diagram"
  FAILED_CHECKS=$((FAILED_CHECKS + 1))
fi

# Test 5: Check that the derive relation is present in Child Section
# The derive relation should flow from Parent Element to Derived Child
if ! echo "$CHILD_SECTION_DIAGRAM" | grep -q -- "-.->|deriveReqT|"; then
  echo "❌ MISSING DERIVE: 'derive' relation should be present in Child Section diagram"
  FAILED_CHECKS=$((FAILED_CHECKS + 1))
fi

if [ $FAILED_CHECKS -gt 0 ]; then
  echo "❌ FAILED: $FAILED_CHECKS relation filtering checks failed"
  echo "Generated diagram content:"
  cat "$TEST_FILE"
  exit 1
fi

# Clean up backup directory
rm -rf "$TEST_DIR/backup"

exit 0