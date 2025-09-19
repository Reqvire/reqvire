use anyhow::Result;
use std::path::{Path, PathBuf};
use pathdiff::diff_paths;
use log::debug;
use walkdir::WalkDir;
use crate::error::ReqvireError;
use globset::GlobSet;
use regex::Regex;
use rustc_hash::FxHasher;
use std::hash::{Hasher};
use crate::git_commands;


/// Checks if a file should be processed
pub fn is_requirements_file_by_path(path: &Path, excluded_filename_patterns: &GlobSet) -> bool {
    let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
       
    filename.ends_with(".md") && !is_excluded_by_patterns(path, &excluded_filename_patterns)
}


/// Checks if a file is excluded based on configured patterns
pub fn is_excluded_by_patterns(path: &Path, excluded_filename_patterns: &GlobSet) -> bool {
    let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

    if excluded_filename_patterns.is_match(path) || excluded_filename_patterns.is_match(filename) {
        debug!("File '{}' is excluded due to matching a glob pattern.", filename);
        return true;
    }

    false
}


pub fn is_in_user_requirements_root(
    file_folder: &PathBuf,
    user_requirements_root_folder: &Option<PathBuf>,
) -> bool {
    if let Some(user_req_folder) = user_requirements_root_folder {
        if user_req_folder.as_os_str().is_empty() {
            return false;
        }
        
        let canonical_file_folder = file_folder.canonicalize().ok();
        let canonical_user_req_folder = user_req_folder.canonicalize().ok();

        match (canonical_file_folder, canonical_user_req_folder) {
            (Some(file), Some(user_req)) => file == user_req,
            _ => false,
        }
    } else {
        false
    }
}

/// Scans the git root folder for markdown files, excluding files based on patterns.
/// If the current working directory is a subfolder of the git root, only scans within that subfolder.
/// 
/// # Arguments
/// 
/// * `commit` - Optional commit ID to scan files from a specific commit
/// * `excluded_filename_patterns` - Glob patterns for files to exclude
/// 
/// # Returns
/// 
/// A vector of paths to the markdown files found
pub fn scan_markdown_files(
    commit: Option<&str>,
    excluded_filename_patterns: &GlobSet,
) -> Vec<PathBuf> {
    match commit {
        Some(commit_id) => {
            scan_markdown_files_from_commit(
                commit_id,
                excluded_filename_patterns
            )
        }
        None => {
            let mut files = Vec::new();
            
            // Get git root directory
            let git_root = match git_commands::get_git_root_dir() {
                Ok(dir) => dir,
                Err(_) => {
                    debug!("Not in a git repository, using current directory");
                    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                }
            };
            
            // Get current working directory
            let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            
            // Determine scan directory - if current directory is within git root but not at git root,
            // scan only within the current directory subtree
            let scan_dir = if current_dir.starts_with(&git_root) && current_dir != git_root {
                current_dir
            } else {
                git_root
            };
            
            debug!("Scanning for markdown files in: {}", scan_dir.display());
            
            // Scan all markdown files in the repository or specified subdirectory
            for entry in WalkDir::new(&scan_dir)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| e.path().is_file() && e.path().extension().map_or(false, |ext| ext == "md"))
                .filter(|e| is_requirements_file_by_path(e.path(), excluded_filename_patterns))
            {
                files.push(entry.path().to_path_buf());
            }

            debug!("Scanned {} markdown files.", files.len());
            files
        }
    }
}

/// Scans the given Git commit for markdown files,
/// excluding files based on provided patterns.
/// 
/// - `commit`: The Git commit (e.g. `"HEAD"`) where we want to look for files.
/// - `excluded_filename_patterns`: Glob patterns for files to exclude
pub fn scan_markdown_files_from_commit(
    commit: &str,
    excluded_filename_patterns: &GlobSet,
) -> Vec<PathBuf> {
    let mut files = Vec::new();

    // Get git root directory
    let git_root = match git_commands::get_git_root_dir() {
        Ok(dir) => dir,
        Err(_) => {
            debug!("Not in a git repository, using current directory");
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        }
    };

    // Run git ls-tree command to get all files in the commit
    let result = git_commands::ls_tree_commit(&commit);
    let documents_vec = match result {
        Err(e) => {
            eprintln!("Error listing files in commit: {}", e);
            Vec::new()
        },       
        Ok(v) => v
    };

    let matching_paths = documents_vec
        .into_iter() 
        .map(|p| git_root.join(p))             
        .filter(|p| p.extension().map_or(false, |ext| ext == "md"))
        .filter(|p| is_requirements_file_by_path(p, excluded_filename_patterns))
        .collect::<Vec<PathBuf>>();

    files.extend(matching_paths);

    files
}


/// Gets the relative path of a file from the git repository root
pub fn get_relative_path(path: &PathBuf) -> Result<PathBuf, ReqvireError> {
    let git_root = match git_commands::get_git_root_dir() {
        Ok(dir) => dir,
        Err(_) => {
            debug!("Not in a git repository, using current directory");
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        }
    };
    
    if let Ok(relative) = path.strip_prefix(&git_root) {
        return Ok(relative.to_path_buf());
    }
    
    Err(ReqvireError::PathError(format!(
        "Failed to determine relative path: {}",
        path.display()
    )))
}



/// Splits an identifier into (file_part, Option(fragment)) following these rules:
/// - If the identifier starts with '#' then it is treated as a fragment-only reference 
///   (file_part is empty, and the fragment is the whole identifier without the leading '#').
/// - Otherwise, if the identifier contains a '/' or a '.' (indicating a file extension), 
///   it is treated as a file reference. In that case, if a '#' is present, split at the first '#' 
///   into file_part and fragment; otherwise, fragment is None.
/// - Otherwise (if there is neither '/' nor '.'), treat the entire identifier as a fragment-only reference.
pub fn extract_path_and_fragment(identifier: &str) -> (&str, Option<&str>) {
    if identifier.is_empty(){
        return ("",None);
    }
    if identifier.starts_with('#') {
        let frag = &identifier[1..];
        return ("", Some(frag));
    }
    // If identifier contains a '/' or a '.', assume it's a file reference.
    if identifier.contains('/') || identifier.contains('.') {
        if let Some(idx) = identifier.find('#') {
            let file_part = &identifier[..idx];
            let frag = &identifier[idx + 1..];
            (file_part, Some(frag))
        } else {
            (identifier, None)
        }
    } else {
        // Otherwise, treat as fragment-only.
        ("", Some(identifier))
    }
}
pub fn normalize_fragment(fragment: &str) -> String{
    fragment
        .trim()
        .to_lowercase()
        .replace(' ', "-")     // Replace spaces with hyphens
        .replace(['(', ')', ',', ':'], "") // Remove disallowed characters
}


/// List of known external URL schemes
pub const EXTERNAL_SCHEMES: &[&str] = &[
    "http://", "https://", "ftp://", "file://", "mailto:", "ssh://", "git://", "data:",
];

pub fn normalize_identifier(
    identifier: &str,
    base_path: &PathBuf,
) -> Result<String, ReqvireError> {
    // 0) Extract the path and any trailing fragment
    let (path_part, fragment_opt) = extract_path_and_fragment(identifier);

    // 1) Passthrough external URIs
    const EXTERNAL_SCHEMES: [&str; 3] = ["http://", "https://", "file://"];
    if EXTERNAL_SCHEMES.iter().any(|scheme| path_part.starts_with(scheme)) {
        return Ok(identifier.to_string());
    }

    // 2) Build the absolute path to work with
    let full_path = {
        let p = Path::new(path_part);
        if p.is_absolute() {
            // For absolute paths, use them directly without canonicalization
            // This allows cross-references to work even when files don't exist
            p.to_path_buf()
        } else {
            let joined_path = base_path.join(p);
            // Try to canonicalize relative path, fall back to logical path resolution if it fails
            match joined_path.canonicalize() {
                Ok(canonical) => canonical,
                Err(_) => {
                    // If canonicalization fails (e.g., file doesn't exist or is outside subdirectory),
                    // resolve the path logically by normalizing . and .. components
                    let mut resolved_path = base_path.clone();
                    
                    // Process the relative path components
                    for component in p.components() {
                        match component {
                            std::path::Component::Normal(_) => {
                                resolved_path.push(component);
                            }
                            std::path::Component::ParentDir => {
                                resolved_path.pop();
                            }
                            std::path::Component::CurDir => {
                                // Skip current directory references
                            }
                            _ => {
                                // Handle other component types (like RootDir, Prefix)
                                resolved_path.push(component);
                            }
                        }
                    }
                    
                    resolved_path
                }
            }
        }
    };

    // 3) Find & canonicalize the Git root
    let git_root = crate::git_commands::get_git_root_dir()
        .map_err(|e| ReqvireError::PathError(format!("Failed to get git root: {}", e)))?
        .canonicalize()
        .map_err(|e| ReqvireError::PathError(format!(
            "Failed to canonicalize git root: {}",
            e
        )))?;

    // 4) Strip the Git‐root prefix
    let rel = full_path
        .strip_prefix(&git_root)
        .map_err(|_| ReqvireError::PathError(format!(
            "`{}` is not inside git root `{}`",
            full_path.display(),
            git_root.display()
        )))?
        .to_string_lossy()
        .into_owned();

    // 5) Re-attach the fragment, if present    
    let full = match fragment_opt {
        Some(frag) => {
            let fragment=normalize_fragment(&frag);
            format!("{}#{}", rel, fragment)
        }
        None => rel,
    };
    
    Ok(full)
}


pub fn to_relative_identifier(
    identifier: &str,
    base_path: &PathBuf,
    should_normalize_fragment: bool,
) -> Result<String, ReqvireError> {
    let (path, fragment_opt) = extract_path_and_fragment(identifier);

    if EXTERNAL_SCHEMES.iter().any(|&scheme| identifier.starts_with(scheme)) {
        return Ok(identifier.to_string());
    }


    let git_root = crate::git_commands::get_git_root_dir()
        .map_err(|e| ReqvireError::PathError(format!("Failed to get git root: {}", e)))?;


    let stripped = if path.starts_with('/') {
        &path[1..]
    } else {
        path
    };
    
    let resolved_path=git_root.join(stripped);
    
    let canonical_path = resolved_path.canonicalize().ok();
    let canonical_base = base_path.canonicalize().ok();
   
    let relative = if let (Some(normalized), Some(base)) = (canonical_path, canonical_base) {
        diff_paths(&normalized, &base).map(|p| p.to_string_lossy().into_owned())
    } else {

        None
    };

    let base = relative.unwrap_or_else(|| path.to_string());

    let full = match fragment_opt {
        Some(frag) => {
            let fragment = if should_normalize_fragment {
                normalize_fragment(&frag)
            } else {
                frag.to_string()
            };
            format!("{}#{}", base, fragment)
        }
        None => base,
    };

    Ok(full)
}


/// Parses a metadata line and extracts a (key, value) pair if valid.
/// Expected format: `* key: value` or `- key: value`
pub fn parse_metadata_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();

    // Ensure it starts with `* ` or `- `
    if !trimmed.starts_with("* ") && !trimmed.starts_with("- ") {
        return None;
    }

    // Split at the first colon `:` to extract key and value
    if let Some((key, value)) = trimmed[2..].split_once(':') {
        let key = key.trim().to_string(); // Normalize key
        let value = value.trim().to_string(); // Normalize value

        if !key.is_empty() && !value.is_empty() {
            return Some((key, value));
        }
    }

    None
}




pub fn parse_relation_line(line: &str) -> Result<(String, (String, String)), ReqvireError> {
    let parts: Vec<&str> = line.splitn(2, ':').map(|s| s.trim()).collect();
    if parts.len() == 2 {
        let relation_type = parts[0].trim_start_matches("* ").trim().to_string(); // Remove unwanted prefix
        let target = parse_target(parts[1]); // Parse target
        Ok((relation_type, target))
    } else {
        Err(ReqvireError::InvalidRelationFormat(format!("Invalid relation format: '{}'", line)))
    }
}


/// Parses a given string and creates a RelationTarget based on rules.
/// Applies automatic normalization during parsing:
/// - Converts non-link identifiers to proper markdown links with display text
/// - Normalizes absolute paths to relative paths
fn parse_target(input: &str) -> (String, String) {
    // 1. Check if the input is a Markdown-style link: `[text](link)`
    if let Some((text, link)) = extract_markdown_link(input) {
        // Apply path normalization if the link is an absolute path
        let normalized_link = normalize_link_path(&link);
        return (text, normalized_link)
    }

    // 2. Convert non-link identifier to proper markdown link
    let (display_text, normalized_link) = normalize_nonlink_identifier(input);
    (display_text, normalized_link)
}

/// Normalizes absolute paths in links to relative paths
fn normalize_link_path(link: &str) -> String {
    // If it's an absolute path starting with '/', try to convert to relative
    if link.starts_with('/') {
        // For now, just remove the leading '/' as a simple normalization
        // TODO: Implement proper relative path conversion when file context is available
        link.trim_start_matches('/').to_string()
    } else {
        link.to_string()
    }
}

/// Converts a non-link identifier to a proper markdown link with display text
fn normalize_nonlink_identifier(input: &str) -> (String, String) {
    let input = input.trim();

    // Split the identifier into file part and optional fragment
    let (file_part, fragment_opt) = extract_path_and_fragment(input);

    // Normalize the fragment if present
    let normalized_link = if let Some(frag) = fragment_opt {
        let norm_frag = normalize_fragment(&frag);
        if file_part.is_empty() {
            // For fragment-only references, always include a leading '#' in the target
            format!("#{}", norm_frag)
        } else {
            format!("{}#{}", file_part, norm_frag)
        }
    } else {
        file_part.to_string()
    };

    // For display text: if it's a fragment-only reference, display it without the leading '#'
    let display_text = if file_part.is_empty() {
        if input.starts_with('#') {
            input.trim_start_matches('#').to_string()
        } else {
            input.to_string()
        }
    } else {
        input.to_string()
    };

    (display_text, normalized_link)
}

/// Extracts text and link from a Markdown-style link if present.
fn extract_markdown_link(input: &str) -> Option<(String, String)> {
    let input = input.trim();
    let markdown_regex = Regex::new(r"^\[(.+?)\]\((.+?)\)$").unwrap();
    if let Some(captures) = markdown_regex.captures(input) {
        let text = captures.get(1)?.as_str().to_string();
        let link = captures.get(2)?.as_str().to_string();
        Some((text, link))
    } else {
        None
    }
}


/// Generates a fast and lightweight hash ID
pub fn hash_identifier(identifier: &str) -> String {
    let mut hasher = FxHasher::default();
    hasher.write(identifier.as_bytes());
    format!("{:x}", hasher.finish()).to_string() 
}
pub fn hash_content(content: &str) -> String {
    let mut hasher = FxHasher::default();
    hasher.write(content.as_bytes());
    format!("{:x}", hasher.finish()).to_string() 
}



#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{ PathBuf};
    use tempfile::TempDir;
    use std::process::Command;
    use git_commands;
    use serial_test::serial;
     
        
    // Mock Config structure for tests
    struct MockConfig {
        pub paths: MockPaths,
    }
    
    struct MockPaths {
        pub user_requirements_root_folder: String,
        pub excluded_filename_patterns: Vec<String>,
    }
    
    impl MockConfig {
        fn default() -> Self {
            Self {
                paths: MockPaths {
                    user_requirements_root_folder: "specifications".to_string(),
                    excluded_filename_patterns: Vec::new(),
                }
            }
        }
        

        
        fn get_excluded_filename_patterns_glob_set(&self) -> globset::GlobSet {
            let mut builder = globset::GlobSetBuilder::new();
            for pattern in &self.paths.excluded_filename_patterns {
                if let Ok(glob) = globset::Glob::new(pattern) {
                    builder.add(glob);
                }
            }
            builder.build().expect("Failed to build glob set")
        }
    }
    
    #[test]
    fn test_extract_path_and_fragment() {
        let test_cases = vec![
            ("/repo/file.md#section", "/repo/file.md", Some("section")),
            ("/repo/path/to/file.md", "/repo/path/to/file.md", None),
            ("/user/repo#readme", "/user/repo", Some("readme")),
            ("/user/repo/", "/user/repo/", None),
            ("File1.md", "File1.md", None),            
            ("onlyfragment", "", Some("onlyfragment")),
            ("#onlyfragment", "", Some("onlyfragment")),
            ("", "", None), // Empty input
        ];

        for (input, expected_path, expected_fragment) in test_cases {
            let (path, fragment) = extract_path_and_fragment(input);
            assert_eq!(path, expected_path, "Failed for input: {:?}", input);
            assert_eq!(fragment, expected_fragment, "Failed for input: {:?}", input);
        }
    }
    
    #[test]
    #[serial]    
    fn test_to_relative_identifier_with_github_fragments() {
        git_commands::clear_git_cache();
        

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let temp_path = temp_dir.path();

        std::env::set_current_dir(&temp_path)
            .expect("Failed to set current directory");

        // Initialize Git repository
        Command::new("git")
            .arg("init")
            .current_dir(&temp_path)
            .output()
            .expect("Failed to initialize git repo");

        let base_path = temp_path.join("specifications/documents/");
        fs::create_dir_all(&base_path).expect("Failed to create base path");

        let file_path = temp_path.join("File4.md");
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(&file_path, "dummy").expect("Failed to create File4.md");
        assert!(file_path.exists(), "File4.md should exist");

        // Test cases: (identifier, base_path, expected result with GitHub-style fragment)
        let test_cases = vec![
        (
            "File1.md#Some Fragment",
            &base_path,
            "File1.md#some-fragment",
        ),
        (
            "/File4.md#Title: Example",
            &base_path,
            "../../File4.md#title-example",
        ),
        (
            "/some/other/File4.md",
            &base_path,
            "../../some/other/File4.md",
        ),                
        (
            "README.md#Installation (Windows)",
            &base_path,
            "README.md#installation-windows",
        ),
        ];

        for (identifier, base_path, expected_result) in test_cases {
            // Prepare the file that the identifier points to
            let (path, _) = crate::utils::extract_path_and_fragment(identifier);
            let full_path = if Path::new(path).is_absolute() {
                temp_path.join(path.trim_start_matches('/'))
            } else {
                base_path.join(path)
            };

            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }

            fs::write(&full_path, "test").unwrap();

            let result = to_relative_identifier(identifier, base_path,true)
                .expect("Failed to to relative identifier");

            assert_eq!(
                result,
                expected_result,
                "Expected GitHub-style normalized result for '{}'",
                identifier
            );
        }
    }
   
    #[test]
    fn test_to_relative_identifier_external_links() {

        let dummy_path = std::env::temp_dir(); // Doesn't matter for external links

        let test_cases = vec![
            "https://example.com/doc.md#Section",
            "http://example.org/readme.md",
            "ftp://files.example.com/archive.zip",
            "mailto:user@example.com",
            "file:///usr/share/docs/manual.md",
        ];

        for identifier in test_cases {
            let result = to_relative_identifier(identifier, &dummy_path,true)
                .expect("Should return external URL unchanged");
            assert_eq!(
                result, identifier,
                "External link was modified unexpectedly: {}",
                identifier
            );
        }
    }   
    
    // Test the is_requirements_file_by_path function
    #[test]
    fn test_is_requirements_file_by_path() {
     
        // Configure external folders for these tests
        let mut config_with_externals = MockConfig::default();
        config_with_externals.paths.user_requirements_root_folder = "specifications".to_string();
    
        config_with_externals.paths.excluded_filename_patterns=vec![
            "**/README*.md".to_string(),
            "**/Logical*.md".to_string(),
            "**/Physical*.md".to_string(),
            "**/index.md".to_string()
        ];
               
                       
        // Test cases for requirements files
        let req_file_cases = vec![
            // Requirements files in specifications root
            "specifications/UserRequirements.md",
            "specifications/SystemRequirements.md",
            "specifications/MissionRequirements.md",
            
            // Requirements files in system requirements folder
            "specifications/SystemRequirements/Requirements.md",
            "specifications/SystemRequirements/Subsystem/Requirements.md",
        ];
        
        // Test cases for non-requirements files
        let non_req_file_cases = vec![
            // Design specifications
            "specifications/DesignSpecifications/DSD_Diagram.md",
            "specifications/DSD_Architecture.md",
            
            // Other markdown files
            "specifications/README.md",
            "README.md",
            
            // Non-markdown files
            "specifications/document.txt",
        ];
        
        // Test that requirements files are properly identified
        for path_str in req_file_cases {
            let path = PathBuf::from(path_str);
            // Skip tests if the path doesn't exist 
            // This allows the tests to be run in different environments
            if !path.exists() {
                continue;
            }
            assert!(is_requirements_file_by_path(&path, &config_with_externals.get_excluded_filename_patterns_glob_set()), 
                    "Expected {} to be identified as a requirements file", path_str);
        }
        
        // Test that non-requirements files are properly excluded
        for path_str in non_req_file_cases {
            let path = PathBuf::from(path_str);
            // Skip tests if the path doesn't exist
            if !path.exists() {
                continue;
            }
            assert!(!is_requirements_file_by_path(&path, &config_with_externals.get_excluded_filename_patterns_glob_set()), 
                    "Expected {} to NOT be identified as a requirements file", path_str);
        }
    }
    
    // Test excluded_filename_patterns
    #[test]
    fn test_excluded_filename_patterns() {
        //let _ = env_logger::builder().is_test(true).try_init();   

        // Configure external folders for these tests
        let mut config_with_externals = MockConfig::default();
        config_with_externals.paths.user_requirements_root_folder = "specifications".to_string();
    
        config_with_externals.paths.excluded_filename_patterns=vec![
            "**/README*.md".to_string(),
            "**/Logical*.md".to_string(),
            "**/Physical*.md".to_string(),
            "**/index.md".to_string()
        ];
                
        

        let test_files = [
            // README* pattern test files
            ("specifications/README.md", true), // Should be excluded
            ("specifications/READMEtest.md", true), // Should be excluded
            ("specifications/readme.md", false), // Should NOT be excluded (case sensitive)
            ("specifications/READ.md", false), // Should NOT be excluded
            ("specifications/subfolder/README.md", true), // Should be excluded
            ("specifications/deep/nested/folder/README.md", true), // Should be excluded
            
            // Logical* pattern test files
            ("specifications/LogicalArchitecture.md", true), // Should be excluded
            ("specifications/LOGICAL_view.md", false), // Should NOT be excluded (case sensitive)
            ("specifications/logical_design.md", false), // Should NOT be excluded (case sensitive)
            ("specifications/Logicless.md", false), // Should NOT be excluded
            ("specifications/subfolder/LogicalModel.md", true), // Should be excluded
            ("external_repo/specs/LogicalView.md", true), // Should be excluded
            
            // Physical* pattern test files
            ("specifications/PhysicalArchitecture.md", true), // Should be excluded
            ("specifications/subfolder/PhysicalDiagram.md", true), // Should be excluded
            ("specifications/NotPhysical.md", false), // Should NOT be excluded
            ("specifications/Physicalsomething.md", true), // Should be excluded
            
            // index.md pattern test files
            ("specifications/index.md", true), // Should be excluded
            ("specifications/subfolder/index.md", true), // Should be excluded
            ("specifications/INDEX.md", false), // Should NOT be excluded (case sensitive)
            ("specifications/indexing_guide.md", false), // Should NOT be excluded
            ("specifications/deep/nested/folder/index.md", true), // Should be excluded
            
            // Standard requirement files - should never be excluded
            ("specifications/Requirements.md", false),
            ("specifications/SystemRequirements.md", false),
            ("specifications/UserRequirements.md", false),
            ("specifications/subfolder/Requirements.md", false),
            ("external_repo/specs/Requirements.md", false),
        ];
        
        
        for (path, should_exclude) in &test_files {
            let test_path = PathBuf::from(path);
            let matches_pattern = is_excluded_by_patterns(&test_path, &config_with_externals.get_excluded_filename_patterns_glob_set());                

            if *should_exclude {
                 assert!(
                     matches_pattern,
                     "❌ File '{}' should be EXCLUDED by pattern but is NOT",
                      path
                 );
            } else {
                   assert!(
                      !matches_pattern,
                      "❌ File '{}' should NOT be excluded by pattern but IS",
                      path
                   );
           }
        }
        
    }
    

    #[test]
    fn test_parse_target_cases() {
        let test_cases = vec![
            // Markdown External URL
            ("[OpenAI](https://openai.com)", "OpenAI", "https://openai.com"),
            
            // Markdown Internal Identifier
            ("[Docs](some-identifier)", "Docs", "some-identifier"),

            // Plain External URL
            ("https://github.com/user/repo", "https://github.com/user/repo", "https://github.com/user/repo"),

            // Plain Internal Identifier
            ("some-identifier", "some-identifier", "some-identifier"),

            // File URL (External)
            ("file:///usr/local/docs.txt", "file:///usr/local/docs.txt", "file:///usr/local/docs.txt"),

            // FTP Link (External)
            ("ftp://example.com/file.zip", "ftp://example.com/file.zip", "ftp://example.com/file.zip"),

            // Mailto Link (External)
            ("mailto:user@example.com", "mailto:user@example.com", "mailto:user@example.com"),

            // Git Repository Link (External)
            ("git://github.com/user/repo.git", "git://github.com/user/repo.git", "git://github.com/user/repo.git"),

            // SSH Link (External)
            ("ssh://192.168.1.1", "ssh://192.168.1.1", "ssh://192.168.1.1"),

            // Data URI (External)
            ("data:image/png;base64,XYZ", "data:image/png;base64,XYZ", "data:image/png;base64,XYZ"),

            // Internal Path Reference
            ("/docs/reference.md", "/docs/reference.md", "/docs/reference.md"),

            // Markdown Internal Path
            ("[Reference](/docs/ref.md)", "Reference", "/docs/ref.md"),
        ];

        for (input, expected_text, expected_link) in test_cases {
            let (text, link) = parse_target(input);
            assert_eq!(text, expected_text, "Failed on input: {}", input);

            assert_eq!(link, expected_link, "Failed on input: {}", input);
        }
    }
    

    #[test]
    #[serial]    
    fn test_to_relative_identifier() {
        git_commands::clear_git_cache();
        
        let temp_spec_folder = TempDir::new().expect("Failed to create temp dir");
        let specifications_folder = temp_spec_folder.path().to_path_buf();

        // Set working directory to temp folder
        std::env::set_current_dir(&specifications_folder)
            .expect("Failed to change current directory");

        // Initialize Git in temp directory
        Command::new("git")
            .arg("init")
            .current_dir(&specifications_folder)
            .output()
            .expect("Failed to initialize git repo");

        let base_path = specifications_folder.join("in/side/");
        fs::create_dir_all(&base_path).expect("Failed to create base path");

        // Create subfolder target file
        let subfolder_path = specifications_folder.join("subfolder");
        fs::create_dir_all(&subfolder_path).expect("Failed to create subfolder");
        fs::write(subfolder_path.join("file.yaml"), "test")
            .expect("Failed to create file.yaml in subfolder");

        // Create a file in the same folder as specifications_folder
        let same_folder_file = specifications_folder.join("file.yaml");
        fs::write(&same_folder_file, "same-level")
            .expect("Failed to create file.yaml in specifications_folder");

        // 1. External URL should be returned unchanged
        let external_url = "http://example.com/path/to/spec";
        let result = to_relative_identifier(external_url, &base_path, false)
            .expect("Should return external URL as-is");
        assert_eq!(result, external_url, "Failed external URL check");

        // 2. Absolute identifier path resolved relative to Git root
        let spec_identifier = "/subfolder/file.yaml";
        let result = to_relative_identifier(&spec_identifier, &base_path, false)
            .expect("Should return relative path inside specifications folder");
        assert_eq!(
            result,
            "../../subfolder/file.yaml",
            "Failed Git-root-based absolute path check"
        );

        // 3. Same-folder file path
        let spec_identifier = "/file.yaml";
                
        let result =
            to_relative_identifier(&spec_identifier, &specifications_folder, false)
                .expect("Should return relative path inside specifications folder");
        assert_eq!(result, "file.yaml", "Failed same-folder file check");


    }
}

/// Diagram utility functions for consistent filtering across the codebase

/// Constant marker used to identify auto-generated diagrams
pub const REQVIRE_AUTOGENERATED_DIAGRAM_MARKER: &str = "REQVIRE-AUTOGENERATED-DIAGRAM";


/// Removes only auto-generated mermaid diagrams from content
/// Preserves user-created diagrams by checking for the auto-generation marker
pub fn remove_autogenerated_diagrams(content: &str) -> String {
    let mut result = String::new();
    let mut lines = content.lines();

    while let Some(line) = lines.next() {
        if line.trim().starts_with("```mermaid") {
            // Collect the entire mermaid block to check if it's auto-generated
            let mut block_lines = vec![line];
            let mut is_auto_generated = false;

            // Read until the closing ```
            while let Some(block_line) = lines.next() {
                block_lines.push(block_line);

                if block_line.contains(REQVIRE_AUTOGENERATED_DIAGRAM_MARKER) {
                    is_auto_generated = true;
                }

                if block_line.trim() == "```" {
                    break;
                }
            }

            // Only include the block if it's NOT auto-generated (preserve user diagrams)
            if !is_auto_generated {
                for block_line in block_lines {
                    result.push_str(block_line);
                    result.push('\n');
                }
            }
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

