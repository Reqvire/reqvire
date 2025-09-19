#!/bin/bash

# Test: HTML Export
# ----------------------------------------------------
# Acceptance Criteria:
# - System should export specifications to HTML format
# - HTML files should be saved in the designated output location
# - HTML output should maintain the structure and content of the original specifications
# - SpecificationIndex.md should be renamed to index.html in output
# - Links in diagrams and text must be converted to use .html instead of .md
# - Paths in HTML files should maintain the original relative structure
# - System should work in environments without Git repositories
#
# Test Criteria:
# - Command exits with success (0) return code
# - HTML files are generated at the expected location with .html extension
# - SpecificationIndex.md is converted to index.html
# - HTML content preserves the structure and information from the source files
# - Links in HTML files use .html extension instead of .md
# - Mermaid click links are properly converted from .md to .html
# - Both GitHub-style URLs and direct file paths in mermaid click links are handled correctly
# - Paths should not have duplicated folder names (e.g., specifications/specifications)
#

# Generate HTML
OUTPUT=$(cd "${TEST_DIR}" && "$REQVIRE_BIN" --config "${TEST_DIR}/reqvire.yaml" html --output output 2>&1)
EXIT_CODE=$?

# Save output for inspection
printf "%s\n" "$OUTPUT" > "${TEST_DIR}/test_results.log"

# Verify exit code indicates success
if [ $EXIT_CODE -ne 0 ]; then
  echo "❌ FAILED: HTML export command failed with exit code $EXIT_CODE"
  exit 1
fi

# Check that output message indicates HTML files were generated
if ! echo "$OUTPUT" | grep -q "Total Markdown files exported: [0-9]"; then
  echo "❌ FAILED: Output message doesn't indicate successful HTML conversion"
  exit 1
fi

# Count the number of Markdown files in the specifications folder
MD_FILE_COUNT=$(find "${TEST_DIR}/specifications" -name "*.md" | wc -l)

# Count the number of HTML files in the output folder
HTML_FILE_COUNT=$(find "${TEST_DIR}/output" -name "*.html" | wc -l)

# Verify that at least one HTML file was generated
if [ $HTML_FILE_COUNT -eq 0 ]; then
  echo "❌ FAILED: No HTML files were generated"
  exit 1
fi

# Verify HTML files match the number of Markdown files
if [ $HTML_FILE_COUNT -lt $MD_FILE_COUNT ]; then
  echo "❌ FAILED: Number of HTML files ($HTML_FILE_COUNT) is less than Markdown files ($MD_FILE_COUNT)"
  exit 1
fi

# Check if README.md was converted to index.html
if [ -f "${TEST_DIR}/SpecificationIndex.md" ] && [ ! -f "${TEST_DIR}/output/index.html" ]; then
  echo "❌ FAILED: SpecificationIndex.md was not converted to index.html"
  exit 1
fi

# Check basic HTML structure generation
FIRST_HTML=$(find "${TEST_DIR}/output" -name "*.html" -not -name "index.html" | head -n 1)
if [ -n "$FIRST_HTML" ]; then
  # Check for presence of HTML structure
  if ! grep -q "<html" "$FIRST_HTML"; then
    echo "❌ FAILED: Generated file doesn't contain basic HTML structure"
    exit 1
  fi
  
  # Check for proper HTML conversion of Markdown headers
  if ! grep -q "<h1\|<h2\|<h3" "$FIRST_HTML"; then
    echo "❌ FAILED: HTML doesn't contain heading tags"
    exit 1
  fi
  
  # Check for presence of README.md to index.html conversion in links
  if grep -q "SpecificationIndex.md" "$FIRST_HTML"; then
    echo "❌ FAILED: HTML file contains unconverted SpecificationIndex.md references"
    exit 1
  fi
  
  # Check for presence of mermaid diagrams in HTML files
  if grep -q "mermaid" "$FIRST_HTML"; then
    
    # Verify that no raw .md references exist in the HTML
    if grep -q "\.md\"" "$FIRST_HTML"; then
      echo "❌ FAILED: HTML file contains unconverted .md links"
      exit 1
    fi
    
    # Check if the file has click links in mermaid diagrams
    if grep -q "click" "$FIRST_HTML"; then
      # Verify that regular click links to md files are converted to html (excluding GitHub URLs)
      if grep "click.*\.md" "$FIRST_HTML" | grep -vq "github.com"; then
        echo "❌ FAILED: HTML file contains unconverted .md links in mermaid click commands (excluding GitHub URLs)"
        exit 1
      fi
      
      # Verify that click links include proper HTML extension
      if ! grep -q "click.*\.html" "$FIRST_HTML"; then
        echo "❌ FAILED: HTML file does not contain properly converted .html links in mermaid click commands"
        exit 1
      fi
      
    fi
    
  fi
  
  # Specifically check the MixedLinkTypes.html file for proper conversion of different link types
  MIXED_LINKS_HTML=$(find "${TEST_DIR}/output" -name "MixedLinkTypes.html" | head -n 1)
  if [ -n "$MIXED_LINKS_HTML" ]; then
    
    # Extract the exact content of the mermaid diagram for analysis
    MERMAID_CONTENT=$(cat "$MIXED_LINKS_HTML" | grep -A 50 '<div class="mermaid">' | grep -B 50 '</div>' | grep -v '<div class="mermaid">' | grep -v '</div>')
    
    # Create a temp file with the extracted content
    TEMP_MERMAID_FILE="${TEST_DIR}/mermaid_content.tmp"
    echo "$MERMAID_CONTENT" > "$TEMP_MERMAID_FILE"
    
    # Check for direct path conversion - exact match with HTML-encoded quotes
    if ! grep -q 'click req1 &quot;TestRequirements.html#test-requirement-1&quot;' "$TEMP_MERMAID_FILE"; then
      echo "❌ FAILED: Direct path link not properly converted in mermaid diagram"
      echo "Expected: click req1 &quot;TestRequirements.html#test-requirement-1&quot;"
      grep "click req1" "$TEMP_MERMAID_FILE"
      exit 1
    fi
    
    # Check for GitHub URL preservation - the GitHub URL should remain as-is with .md extension
    if ! grep -q 'click req2 &quot;https://github.com/user/repo/blob/main/specifications/TestRequirements.md#test-requirement-1&quot;' "$TEMP_MERMAID_FILE"; then
      echo "❌ FAILED: GitHub URL link not preserved in mermaid diagram"
      echo "Expected: click req2 &quot;https://github.com/user/repo/blob/main/specifications/TestRequirements.md#test-requirement-1&quot;"
      grep "click req2" "$TEMP_MERMAID_FILE"
      exit 1
    fi
    
    # Check that non-markdown links are preserved - exact match with HTML-encoded quotes
    if ! grep -q 'click req3 &quot;https://github.com/user/repo/blob/main/src/main.rs&quot;' "$TEMP_MERMAID_FILE"; then
      echo "❌ FAILED: Non-markdown link not preserved in mermaid diagram"
      echo "Expected: click req3 &quot;https://github.com/user/repo/blob/main/src/main.rs&quot;"
      grep "click req3" "$TEMP_MERMAID_FILE"
      exit 1
    fi
    
    # Check for parent directory path conversion - exact match with HTML-encoded quotes
    if ! grep -q 'click req4 &quot;../TestRequirements.html#test-requirement-2&quot;' "$TEMP_MERMAID_FILE"; then
      echo "❌ FAILED: Parent directory path not properly converted in mermaid diagram"
      echo "Expected: click req4 &quot;../TestRequirements.html#test-requirement-2&quot;"
      grep "click req4" "$TEMP_MERMAID_FILE"
      exit 1
    fi
    
    # Check for MD file without fragment conversion - exact match with HTML-encoded quotes
    if ! grep -q 'click req5 &quot;TestRequirements.html&quot;' "$TEMP_MERMAID_FILE"; then
      echo "❌ FAILED: MD file without fragment not properly converted in mermaid diagram"
      echo "Expected: click req5 &quot;TestRequirements.html&quot;"
      grep "click req5" "$TEMP_MERMAID_FILE"
      exit 1
    fi
    
    # Check for GitHub URL without fragment - should remain as-is with .md extension
    if ! grep -q 'click req6 &quot;https://github.com/user/repo/blob/main/specifications/TestRequirements.md&quot;' "$TEMP_MERMAID_FILE"; then
      echo "❌ FAILED: GitHub URL without fragment not preserved in mermaid diagram"
      echo "Expected: click req6 &quot;https://github.com/user/repo/blob/main/specifications/TestRequirements.md&quot;"
      grep "click req6" "$TEMP_MERMAID_FILE"
      exit 1
    fi
    
    # Clean up temp file
    rm -f "$TEMP_MERMAID_FILE"
        
    # Check for duplicated folder names in paths
    if grep -q "specifications/specifications" "$MIXED_LINKS_HTML"; then
      echo "❌ FAILED: Found duplicated folder names (specifications/specifications) in paths"
      exit 1
    fi
  else
    echo "⚠️ WARNING: MixedLinkTypes.html not found for detailed link conversion testing"
  fi
fi

exit 0
