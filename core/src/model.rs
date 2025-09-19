use anyhow::Result;
use std::path::PathBuf;

use log::debug;
use crate::graph_registry::GraphRegistry;
use crate::error::ReqvireError;
use crate::filesystem;

use crate::utils;
use crate::parser;
use globset::GlobSet;

#[derive(Debug)]
pub struct ModelManager {
    /// In-memory graph registry of elements and relations
    pub graph_registry: GraphRegistry,

}

impl ModelManager {
    /// Creates a new ModelManager
    pub fn new() -> Self {
        // Initialize empty graph registry
        let graph_registry = GraphRegistry::new();

        Self {
            graph_registry
        }
    }


    pub fn parse_and_validate(
        &mut self,
        git_commit_hash: Option<&str>,
        user_requirements_root_folder: &Option<PathBuf>,
        excluded_filename_patterns: &GlobSet
    ) -> Result<Vec<ReqvireError>, ReqvireError> {
        debug!("Starting two-pass validation architecture");

        // Pass 1: Element collection with local validation
        let pass1_errors = self.pass1_collect_elements(
            git_commit_hash,
            user_requirements_root_folder,
            excluded_filename_patterns
        )?;

        // If Pass 1 has errors, return them as an error
        if !pass1_errors.is_empty() {
            debug!("Pass 1 validation failed with {} errors", pass1_errors.len());
            return Err(ReqvireError::ValidationError(pass1_errors));
        }

        debug!("Pass 1 completed successfully, proceeding to Pass 2");

        // Pass 2: Graph construction and relation validation
        let pass2_errors = self.pass2_build_relations(excluded_filename_patterns)?;

        // If Pass 2 has errors, return them as an error
        if !pass2_errors.is_empty() {
            debug!("Pass 2 validation failed with {} errors", pass2_errors.len());
            return Err(ReqvireError::ValidationError(pass2_errors));
        }

        debug!("Both passes completed successfully");
        Ok(Vec::new())
    }

    /// Pass 1: Parse documents and collect elements with local validation
    fn pass1_collect_elements(
        &mut self,
        git_commit_hash: Option<&str>,
        user_requirements_root_folder: &Option<PathBuf>,
        excluded_filename_patterns: &GlobSet
    ) -> Result<Vec<ReqvireError>, ReqvireError> {
        let mut errors = Vec::new();

        let files = utils::scan_markdown_files(git_commit_hash, excluded_filename_patterns);
        debug!("Pass 1: Found {} markdown files.", files.len());

        let file_iterator = filesystem::FileReaderIterator::new(git_commit_hash, files);
        for file_result in file_iterator {
            match file_result {
                Err(e) => return Err(e),
                Ok((path, file_name, file_content)) => {
                    debug!("Pass 1: Processing file: {}", file_name);

                    let relative_path_str = utils::get_relative_path(&path)?.to_string_lossy().to_string();

                    // Parse Elements, page content, and section content
                    let (elements, parse_errors, page_content, sections) = parser::parse_elements(
                        &file_name,
                        &file_content,
                        &path,
                        user_requirements_root_folder,
                    );

                    // Collect parse-time errors
                    errors.extend(parse_errors);

                    // Register page content
                    self.graph_registry.register_page(relative_path_str.clone(), page_content);

                    // Register section content
                    for (section_name, section_content, section_order) in sections {
                        self.graph_registry.register_section_with_order(
                            relative_path_str.clone(),
                            section_name,
                            section_content,
                            section_order
                        );
                    }

                    // Register parsed elements with local validation
                    for element in elements {
                        if let Err(e) = self.graph_registry.register_element(element, &relative_path_str) {
                            errors.push(e);
                        }
                    }
                }
            }
        }

        Ok(errors)
    }

    /// Pass 2: Build relations and validate graph structure
    fn pass2_build_relations(
        &mut self,
        excluded_filename_patterns: &GlobSet
    ) -> Result<Vec<ReqvireError>, ReqvireError> {
        debug!("Pass 2: Delegating to GraphRegistry for relation building and validation");
        self.graph_registry.build_relations(excluded_filename_patterns)
    }
}


#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::linting::LintFix;

    #[test]
    fn test_extract_path_and_fragment() {
        // Test file reference with fragment.
        let input = "/user/repo#readme";
        let (file, frag) = crate::utils::extract_path_and_fragment(input);
        assert_eq!(file, "/user/repo");
        assert_eq!(frag, Some("readme"));

        // Test fragment-only with leading '#'.
        let input = "#intro";
        let (file, frag) = crate::utils::extract_path_and_fragment(input);
        assert_eq!(file, "");
        assert_eq!(frag, Some("intro"));

        // Test file only.
        let input = "document.md";
        let (file, frag) = crate::utils::extract_path_and_fragment(input);
        assert_eq!(file, "document.md");
        assert_eq!(frag, None);

        // Test fragment-only without '#' (treated as fragment-only)
        let input = "onlyfragment";
        let (file, frag) = crate::utils::extract_path_and_fragment(input);
        assert_eq!(file, "");
        assert_eq!(frag, Some("onlyfragment"));
    }

    #[test]
    fn test_find_nonlink_identifiers_plain_file_md() {
        // This test verifies that non-link plain file references are detected.
        let content = "Check out file.md please.";
        let file_path = PathBuf::from("test.md");
        let suggestions = crate::linting::nonlink_identifiers::find_nonlink_identifiers(content, &file_path);
        // Our regex for relation lines only matches relation bullet lines.
        // So this should produce 0 suggestions.
        assert_eq!(suggestions.len(), 0);
    }

    #[test]
    fn test_find_nonlink_identifiers_file_md_with_fragment() {
        let content = " * derivedFrom: file.md#Element Name with spaces";
        let file_path = PathBuf::from("test.md");
        let suggestions = crate::linting::nonlink_identifiers::find_nonlink_identifiers(content, &file_path);
        assert_eq!(suggestions.len(), 1, "Expected one suggestion");

        let suggestion = &suggestions[0];
        if let LintFix::ReplacePattern { pattern, replacement } = &suggestion.fix {
            // Pattern should contain the original raw identifier.
            assert!(pattern.contains("file.md#Element Name with spaces"), "pattern: {:?}", pattern);
            // Normalized: "file.md#element-name-with-spaces", link text remains as "file.md#Element Name with spaces"
            let expected_link = "[file.md#Element Name with spaces](file.md#element-name-with-spaces)";
            assert!(replacement.contains(expected_link), "replacement: {:?}", replacement);
        } else {
            panic!("Expected ReplacePattern fix");
        }
    }

    #[test]
    fn test_find_nonlink_identifiers_hash_only_fragment() {
        let content = " * derivedFrom: #Some Fragment";
        let file_path = PathBuf::from("test.md");
        let suggestions = crate::linting::nonlink_identifiers::find_nonlink_identifiers(content, &file_path);
        assert_eq!(suggestions.len(), 1, "Expected one suggestion");

        let suggestion = &suggestions[0];
        if let LintFix::ReplacePattern { pattern, replacement } = &suggestion.fix {
            // For a hash-only fragment, the file part is empty.
            assert!(pattern.contains("#Some Fragment"), "pattern: {:?}", pattern);
            // Link text should be "Some Fragment" (without '#') and link target should be "#some-fragment"
            let expected_link = "[Some Fragment](#some-fragment)";
            assert!(replacement.contains(expected_link), "replacement: {:?}", replacement);
        } else {
            panic!("Expected ReplacePattern fix");
        }
    }

    #[test]
    fn test_find_nonlink_identifiers_already_bracketed_link_ignored() {
        let content = "Check out [file.md](file.md) for details.";
        let file_path = PathBuf::from("test.md");
        let suggestions = crate::linting::nonlink_identifiers::find_nonlink_identifiers(content, &file_path);
        // Should ignore already bracketed links.
        assert_eq!(suggestions.len(), 0, "Expected no suggestions for already bracketed links");
    }
}





