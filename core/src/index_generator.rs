use std::path::{PathBuf, Path};
use std::collections::HashMap;
use crate::graph_registry::GraphRegistry;
use crate::element::Element;
use crate::filesystem::write_file; 
use crate::error::ReqvireError;
use crate::git_commands;


/// Generates a SpecificationsIndex.md index from the existing element registry
pub fn generate_readme_index(
    registry: &GraphRegistry, 
    _output_folder: &PathBuf
) -> Result<String, ReqvireError> {
    let mut index_content = String::from("# Specification Index\n\n");

    // Group elements by file and section
    let mut grouped_elements: HashMap<String, HashMap<String, Vec<&Element>>> = HashMap::new();

    for element in registry.get_all_elements() {
        grouped_elements
            .entry(element.file_path.clone()) // Group by file
            .or_insert_with(HashMap::new)
            .entry(element.section.clone()) // Group by section
            .or_insert_with(Vec::new)
            .push(element);
    }

    // Generate the README index
    let mut sorted_files: Vec<_> = grouped_elements.keys().collect();
    sorted_files.sort(); // Sort files alphabetically

    for file in sorted_files {
        let sections = grouped_elements.get(file).unwrap();

        // Compute relative path
        let relative_path = get_relative_path(file);


        index_content.push_str(&format!("## [{}]({})\n", relative_path, relative_path));

        let mut sorted_sections: Vec<_> = sections.keys().collect();
        sorted_sections.sort(); // Sort sections alphabetically

        for section in sorted_sections {
            let elements = sections.get(section).unwrap();
            let section_id = generate_section_slug(section);


            index_content.push_str(&format!("- [{}]({}#{})\n", section, relative_path, section_id));


            for element in elements {
                let element_id = generate_section_slug(&element.identifier);
                index_content.push_str(&format!("  - [{}]({}#{})\n", element.name, relative_path, element_id));
            }

            index_content.push_str("\n"); // Add spacing between sections
        }

        index_content.push_str("\n"); // Add spacing between files
    }
    
    let total_files = grouped_elements.len();
    let total_sections: usize = grouped_elements.values().map(|s| s.len()).sum();
    let total_elements: usize = registry.get_all_elements().len();

    index_content.push_str(&format!(
        "\n---\n📊 **Summary:**\n- {} Files\n- {} Sections\n- {} Elements\n",
        total_files, total_sections, total_elements
    ));

    // Determine where to write the index file
    let git_root = git_commands::get_git_root_dir().unwrap_or_else(|_| PathBuf::from("."));
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    
    // If running from a subdirectory, write index in current directory
    // Otherwise, write to git root (original behavior)
    let output_path = if current_dir.starts_with(&git_root) && current_dir != git_root {
        current_dir.join("SpecificationIndex.md")
    } else {
        git_root.join("SpecificationIndex.md")
    };
    
    // If writing fails, return an error immediately
    if let Err(e) = write_file(&output_path, index_content.as_bytes()) {
        return Err(e);
    }

    println!("✅ SpecificationIndex.md generated at: {}", output_path.display());

    // Return the generated content only if writing was successful
    Ok(index_content)
}

fn generate_section_slug(section: &str) -> String {
    section
        .to_lowercase()
        .replace(" ", "-")
        .replace(|c: char| !c.is_alphanumeric() && c != '-', "") // Remove special characters
}

/// Computes a relative path to a file from the git repository root
fn get_relative_path(file: &str) -> String {
    match git_commands::get_git_root_dir() {
        Ok(git_root) => {
            let file_path = Path::new(file);
            match file_path.strip_prefix(&git_root) {
                Ok(relative) => relative.to_string_lossy().into_owned(),
                Err(_) => file.to_string(), // Fallback to absolute if stripping fails
            }
        },
        Err(_) => file.to_string() // Fall back to the file name if git root not found
    }
}

