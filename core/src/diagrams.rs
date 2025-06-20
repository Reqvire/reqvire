use std::collections::{HashMap, HashSet};
use crate::element::Element;
use crate::element_registry::ElementRegistry;
use crate::error::ReqvireError;
use std::path::PathBuf;
use crate::utils;
use log::debug;
use crate::relation;
use crate::element::ElementType;
use crate::element::RequirementType;
use crate::git_commands;
use crate::filesystem;

/// Generates diagrams grouped by `file_path` and `section`
pub fn generate_diagrams_by_section(
    registry: &ElementRegistry,
    direction: &str,
    diagrams_with_blobs: bool,    
) -> Result<HashMap<String, String>, ReqvireError> {
    let mut diagrams: HashMap<String, String> = HashMap::new();

    // Group elements by (file_path, section)
    let mut grouped_elements: HashMap<(String, String), Vec<&Element>> = HashMap::new();
    
    let elements=registry.get_all_elements();
    
    for element in elements {
        grouped_elements
            .entry((element.file_path.clone(), element.section.clone()))
            .or_insert_with(Vec::new)
            .push(element);
    }

    // Generate diagrams for each group
    for ((file_path, section), section_elements) in grouped_elements {
        debug!("Generating diagram for file: {}, section: {}", file_path, section);

        let diagram = generate_section_diagram(registry, &section, &section_elements, &file_path, direction, diagrams_with_blobs)?;
        let diagram_key = format!("{}::{}", file_path, section);
        diagrams.insert(diagram_key, diagram);
    }

    Ok(diagrams)
}

/// Generates a diagram for a single section
fn generate_section_diagram(
    registry: &ElementRegistry,
    _section: &str,
    elements: &[&Element],
    file_path: &str,
    direction: &str,
    diagrams_with_blobs: bool
) -> Result<String, ReqvireError> {
    // Get Git repository information for creating proper links
    let repo_root = match git_commands::get_git_root_dir() {
        Ok(root) => root,
        Err(_) => PathBuf::from(""),
    };
    
    let base_url = match git_commands::get_repository_base_url() {
        Ok(url) => url,
        Err(_) => String::from(""),
    };
    
    let commit_hash = match git_commands::get_commit_hash() {
        Ok(hash) => hash,
        Err(_) => String::from("HEAD"),
    };

    // Get diagram direction from config (TD or LR)
    let mut diagram = String::from(format!("```mermaid\ngraph {};\n", direction));

    // Define Mermaid graph styles
    diagram.push_str("  %% Graph styling\n");
    diagram.push_str("  classDef requirement fill:#f9d6d6,stroke:#f55f5f,stroke-width:1px;\n");
    diagram.push_str("  classDef verification fill:#d6f9d6,stroke:#5fd75f,stroke-width:1px;\n");
    diagram.push_str("  classDef externalLink fill:#d0e0ff,stroke:#3080ff,stroke-width:1px;\n");
    diagram.push_str("  classDef default fill:#f5f5f5,stroke:#333333,stroke-width:1px;\n\n");

    let mut included_elements = HashSet::new();

    for element in elements {
        add_element_to_diagram(
            registry, 
            &mut diagram, 
            element, 
            &mut included_elements, 
            file_path,
            diagrams_with_blobs,           
            &repo_root,
            &base_url,
            &commit_hash,
        )?;
    }

    diagram.push_str("```");

    Ok(diagram)
}

/// Adds an element and its relations to the diagram
fn add_element_to_diagram(
    registry: &ElementRegistry,
    diagram: &mut String,
    element: &Element,
    included_elements: &mut HashSet<String>,
    file_path: &str,
    diagrams_with_blobs: bool,  
    repo_root: &PathBuf,
    base_url: &str,
    commit_hash: &str,
) -> Result<(), ReqvireError> {

    // Convert file path to its parent directory (returns PathBuf)
    let base_dir = PathBuf::from(file_path)
        .parent()
        .map(|p| p.to_path_buf()) 
        .unwrap_or_else(|| PathBuf::from("."));
 
    // Get relative ID for local navigation
    let relative_target = utils::to_relative_identifier(
        &element.identifier.clone(),
        &base_dir,
        false
    )?;


    // Create a stable GitHub link for the element if we have the git info and diagrams should be with blobs
    let has_git_info = !base_url.is_empty() && !commit_hash.is_empty() && !repo_root.as_os_str().is_empty();
       
    let click_target = if diagrams_with_blobs && has_git_info {
        // Get repository-relative path
        let relative_id = match utils::get_relative_path(&PathBuf::from(&element.identifier)) {
            Ok(rel_path) => rel_path.to_string_lossy().to_string(),
            Err(_) => element.identifier.clone(),
        };
        
        // Create a git link for the element
        format!("{}/blob/{}/{}", base_url, commit_hash, relative_id)
    } else {
        // Fall back to the relative link for local navigation
        relative_target.clone()
    };    
        
    let element_id = utils::hash_identifier(&element.identifier);   

    if !included_elements.contains(&element.identifier) {
       included_elements.insert(element.identifier.clone());
       
       let label = element.name.replace('"', "&quot;");
       
       let class=match &element.element_type {
           ElementType::Requirement(RequirementType::User)  => "requirement",                    
           ElementType::Requirement(RequirementType::System) =>"requirement",
           ElementType::Verification(_) =>"verification",           
           _ => "default"
       };
           
                  
       // Add the element node
       diagram.push_str(&format!("  {}[\"{}\"];\n", element_id, label));      
       diagram.push_str(&format!("  class {} {};\n", element_id, class));
       diagram.push_str(&format!("  click {} \"{}\";\n", element_id, click_target));       
    }



    for relation in &element.relations {
        if !relation.is_opposite {
        
        
        let label = relation.target.text.clone();
        let target_id = match &relation.target.link {
            relation::LinkType::Identifier(target) => {            
                
                let target_id = utils::hash_identifier(&target);               

                // Get relative ID for local navigation
                let relative_target = utils::to_relative_identifier(
                    &target,
                    &base_dir,
                    false
                )?;
                
           
                // Get a GitHub link if we have git info
                let click_target = if diagrams_with_blobs &&  has_git_info {
                    // Get repository-relative path
                    let relative_id = match utils::get_relative_path(&PathBuf::from(target)) {
                        Ok(rel_path) => rel_path.to_string_lossy().to_string(),
                        Err(_) => target.clone(),
                    };
                    
                    // Create a git link for the target element
                    format!("{}/blob/{}/{}", base_url, commit_hash, relative_id)
                } else {
                    // Fall back to the relative link for local navigation
                    relative_target.clone()
                };
                             
                
                if !included_elements.contains(target) {
                    included_elements.insert(target.clone());
                                 
                    let class = match registry.get_element(&target) {
                        Ok(existing_element)=>{
                            match existing_element.element_type {
                                ElementType::Requirement(RequirementType::User)  => "requirement",                    
                                ElementType::Requirement(RequirementType::System) => "requirement",
                                ElementType::Verification(_) => "verification",           
                                _ => "default"                    
                             }
                        },
                        _ => "default"
                    };
                                                               
                    diagram.push_str(&format!("  {}[\"{}\"];\n", target_id, label));
                    diagram.push_str(&format!("  class {} {};\n", target_id, class));                    
                    diagram.push_str(&format!("  click {} \"{}\";\n", target_id, click_target));                    
                }
                target_id
            },
            relation::LinkType::ExternalUrl(url) => {
                // Always add external URLs, regardless of `included_elements`
                let target_id = utils::hash_identifier(url);
                diagram.push_str(&format!("  {}[\"{}\"];\n", target_id, label));
                diagram.push_str(&format!("  class {} {};\n", target_id,"default"));
                diagram.push_str(&format!("  click {} \"{}\";\n", target_id, url));
                
                target_id               
            },
            relation::LinkType::InternalPath(path) => {
                // Get relative ID for local navigation
                let relative_target = utils::to_relative_identifier(
                    &path.to_string_lossy().into_owned(),
                    &base_dir,
                    false
                )?;
                
           
                // Get a GitHub link if we have git info
                let click_target = if diagrams_with_blobs &&  has_git_info {
                    // Get repository-relative path
                    let relative_id = match utils::get_relative_path(&path) {
                        Ok(rel_path) => rel_path.to_string_lossy().to_string(),
                        Err(_) => path.to_string_lossy().to_string()
                    };
                    
                    // Create a git link for the target element
                    format!("{}/blob/{}/{}", base_url, commit_hash, relative_id)
                } else {
                    // Fall back to the relative link for local navigation
                    relative_target.clone()
                };
            
                // Always add internal paths, regardless of `included_elements`
                let cow = path.to_string_lossy();
                let path_str = cow.as_ref();
                let target_id = utils::hash_identifier(path_str);
                diagram.push_str(&format!("  {}[\"{}\"];\n", target_id, label));
                diagram.push_str(&format!("  class {} {};\n", target_id,"default"));
                diagram.push_str(&format!("  click {} \"{}\";\n", target_id, click_target));
                
                target_id               
            }            
        };


        if let Some(info) = relation::RELATION_TYPES.get(relation.relation_type.name) {
            // pick from/to based on semantic direction
            let (from_id, to_id) = match info.direction {
                relation::RelationDirection::Forward => (target_id.clone(), element_id.clone()),
                _                           => (element_id.clone(), target_id.clone()),
            };

            diagram.push_str(&format!(
                "  {} {}|{}| {};\n",
                from_id,
                info.arrow,
                info.label,
                to_id,
            ));
        } else {
            // fallback: unknown relation
            diagram.push_str(&format!(
                "  {} -->|relates to| {};\n",
                element_id, target_id,
            ));
        }
        }
    }

    Ok(())
}


/// Processes diagram generation for markdown files in place (without writing to output).
/// Used when the `--generate-diagrams` flag is set.
pub fn process_diagrams(
    registry: &ElementRegistry,
    diagram_direction: &str,
    diagrams_with_blobs: bool,    
    ) -> Result<(), ReqvireError> {

    // Generate diagrams by section
    let diagrams = generate_diagrams_by_section(&registry, diagram_direction, diagrams_with_blobs)?;

    // Group diagrams by file path
    let mut files_to_update: HashMap<String, Vec<(&str, &String)>> = HashMap::new();

    for (file_section_key, new_diagram) in &diagrams {
        let parts: Vec<&str> = file_section_key.split("::").collect();
        if parts.len() != 2 {
            continue; // Skip invalid entries
        }
        let file_path = parts[0];
        let section = parts[1];
    
        files_to_update
            .entry(file_path.to_string())
            .or_insert_with(Vec::new)
            .push((section, new_diagram));
    }

    // Get git root for resolving relative paths
    let git_root = match git_commands::get_git_root_dir() {
        Ok(root) => root,
        Err(_) => {
            log::error!("Not in a git repository, using current directory");
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        }
    };

    // Process each file
    for (file_path, section_diagrams) in files_to_update {
        // Resolve file path relative to git root, not current directory
        let absolute_file_path = git_root.join(&file_path);

        // Read file content
        let mut file_content = match filesystem::read_file(&absolute_file_path) {
            Ok(content) => content,
            Err(e) => {
                log::error!("Failed to read file '{}': {}", absolute_file_path.display(), e);
                continue;
            }
        };

        // Replace diagrams for all sections in this file
        for (section, new_diagram) in section_diagrams {              
            file_content = replace_section_diagram(&file_content, section, new_diagram);
        }

        // Write updated content back if modified
        if let Err(e) = filesystem::write_file(&absolute_file_path, &file_content) {
            log::error!("Failed to write updated diagrams to '{}': {}", absolute_file_path.display(), e);
        } else {
            println!("Updated diagrams in '{}'", file_path);
        }
    }

    Ok(())
}

/// Replaces the old diagram in a specific section of a markdown file.
///
/// - `content`: The original file content.
/// - `section`: The section name where the diagram should be replaced.
/// - `new_diagram`: The newly generated Mermaid diagram.
///
/// Returns the modified file content as a `String`.
fn replace_section_diagram(content: &str, section: &str, new_diagram: &str) -> String {
    let section_header = format!("## {}", section);
    let mermaid_block_start = "```mermaid";
    let mermaid_block_end = "```";

    let mut new_lines = Vec::new();
    let mut lines = content.lines().peekable();
    while let Some(line) = lines.next() {
        if line.trim() == section_header {
            // Found the target section header.
            new_lines.push(line.to_string());
            // Insert the new diagram immediately after the header.
            new_lines.push(new_diagram.to_string());
            // Skip any blank lines immediately after the header.
            while let Some(&next_line) = lines.peek() {
                if next_line.trim().is_empty() {
                    lines.next();
                } else {
                    break;
                }
            }
            // If the next non-empty line starts a Mermaid block, skip it.
            if let Some(&next_line) = lines.peek() {
                if next_line.trim().starts_with(mermaid_block_start) {
                    // Skip the mermaid block: first skip the start marker.
                    lines.next();
                    // Then skip lines until the end marker is found.
                    while let Some(l) = lines.next() {
                        if l.trim().starts_with(mermaid_block_end) {
                            break;
                        }
                    }
                }
            }
            // Continue with the rest of the file.
            } else {
                new_lines.push(line.to_string());
        }
    }
    new_lines.join("\n")
}
