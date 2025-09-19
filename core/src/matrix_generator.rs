// traceability_matrix.rs

use std::collections::{HashMap, HashSet};
use serde_json::{json};
use crate::graph_registry::GraphRegistry;
use crate::element::{Element, ElementType};
use crate::relation::{LinkType, RELATION_TYPES};
use crate::git_commands;
use crate::element;

/// Enum to specify the matrix format
pub enum MatrixFormat {
    Markdown,
    Json,
    Svg,
}

/// Configuration for the traceability matrix
pub struct MatrixConfig {
    /// Source element type (e.g., "requirement")
    pub source_types: Vec<ElementType>,
    /// Target element type (e.g., "verification")
    pub target_types: Vec<ElementType>,
    /// Relation types to trace (e.g., ["verifiedBy"])
    pub relation_types: Vec<&'static str>,
}

impl Default for MatrixConfig {
    fn default() -> Self {
        MatrixConfig {
            source_types: vec![
                 ElementType::Requirement(element::RequirementType::System),
                 ElementType::Requirement(element::RequirementType::User),
            ],
            target_types: vec![
                ElementType::Verification(element::VerificationType::Test),
                ElementType::Verification(element::VerificationType::Analysis),
                ElementType::Verification(element::VerificationType::Inspection),
                ElementType::Verification(element::VerificationType::Demonstration)
            ],
            relation_types: vec!["verifiedBy"]
        }
    }
}

/// Generates a traceability matrix based on the provided configuration
pub fn generate_matrix(
    registry: &GraphRegistry,
    config: &MatrixConfig,
    format: MatrixFormat,    
) -> String {
    // Retrieve Git repository information
    let base_url = git_commands::get_repository_base_url().unwrap_or_default();
    let commit_hash = git_commands::get_commit_hash().unwrap_or_else(|_| String::from("HEAD"));

    // Collect source and target elements based on their types
    let source_elements: Vec<&Element> = registry.get_all_elements()
        .iter()
        .filter(|elem| config.source_types.contains(&elem.element_type))
        .cloned()
        .collect();

    let target_elements: Vec<&Element> = registry.get_all_elements()
        .iter()
        .filter(|elem| config.target_types.contains(&elem.element_type))
        .cloned()
        .collect();

    // Build the mapping of source to target elements
    let mut matrix_data: HashMap<String, HashSet<String>> = HashMap::new();

    for source in &source_elements {
        let mut targets = HashSet::new();

        for relation in &source.relations {
            if config.relation_types.contains(&relation.relation_type.name) {
                if let LinkType::Identifier(target_id) = &relation.target.link {
                    if let Some(target) = registry.get_element(target_id) {
                        if config.target_types.contains(&target.element_type) {
                            targets.insert(target_id.clone());
                        }
                    }
                }
            }
        }

        if !targets.is_empty() {
            matrix_data.insert(source.identifier.clone(), targets);
        }
    }

    // Generate the matrix in the specified format
    match format {
        MatrixFormat::Markdown => generate_markdown_matrix(
            &matrix_data,
            &source_elements,
            &target_elements,
            &config.relation_types,
            &registry,
            &base_url,
            &commit_hash,
        ),
        MatrixFormat::Json => generate_json_matrix(
            &matrix_data,
            &source_elements,
            &target_elements,
            &config.relation_types,
            &base_url,
            &commit_hash,
        ),
        MatrixFormat::Svg => generate_svg_matrix(
            &matrix_data,
            &source_elements,
            &target_elements,
        ),
    }
}


fn generate_matrix_table(
    all_targets: &[&Element],
    source_elements: &[&Element],
    matrix_data: &HashMap<String, HashSet<String>>,
    output: &mut String,
    base_url: &str,
    commit_hash: &str,
) {
    // If no source elements, return early
    if source_elements.is_empty() {
        output.push_str("No requirements found for this group.\n");
        return;
    }

    // Filter relevant target elements
    let relevant_targets: Vec<&Element> = all_targets.iter()
        .filter(|target| {
            source_elements.iter().any(|source| {
                matrix_data.get(&source.identifier)
                    .map_or(false, |targets| targets.contains(&target.identifier))
            })
        })
        .cloned()
        .collect();

    // Start the table
    output.push_str("| Requirement | Verified |");
    for target in &relevant_targets {
        let short_name = get_short_element_name(target);
        let target_url = format!("{}/blob/{}/{}", base_url, commit_hash, &target.identifier);
        output.push_str(&format!(" [{}]({}) |", short_name, target_url));
    }
    output.push_str("\n|");
    for _ in 0..relevant_targets.len() + 2 {
        output.push_str(" --- |");
    }
    output.push_str("\n");

    // Parent-child hierarchy
    let mut parent_to_children: HashMap<String, Vec<&Element>> = HashMap::new();
    let parent_relation_types = crate::relation::get_parent_relation_types();
    let mut hierarchical_elements = Vec::new();

    for source in source_elements {
        let mut has_parent = false;
        for relation in &source.relations {
            if parent_relation_types.contains(&relation.relation_type.name) {
                if let LinkType::Identifier(parent_id) = &relation.target.link {
                    parent_to_children.entry(parent_id.clone()).or_default().push(source);
                    has_parent = true;
                    break;
                }
            }
        }
        if !has_parent {
            hierarchical_elements.push((source, 0)); // No parent = top-level
        }
    }

    fn add_children<'a>(
        element: &'a Element,
        level: usize,
        result: &mut Vec<(&'a Element, usize)>,
        parent_to_children: &HashMap<String, Vec<&'a Element>>,
    ) {
        result.push((element, level));
        if let Some(children) = parent_to_children.get(&element.identifier) {
            let mut sorted_children = children.clone();
            sorted_children.sort_by(|a, b| a.name.cmp(&b.name));
            for child in sorted_children {
                add_children(child, level + 1, result, parent_to_children);
            }
        }
    }

    let mut sorted_hierarchy = Vec::new();
    let mut sorted_roots = hierarchical_elements.clone();
    sorted_roots.sort_by(|(a, _), (b, _)| a.name.cmp(&b.name));
    for (element, level) in sorted_roots {
        add_children(element, level, &mut sorted_hierarchy, &parent_to_children);
    }

    // Rows with hierarchy and indentation
    for (source, level) in sorted_hierarchy {
        let source_id = &source.identifier;
        let short_name = get_short_element_name(source);
        let indentation = match level {
            0 => "",
            1 => "↳ ",
            2 => "__↳ ",
            3 => "____↳ ",
            _ => "______↳ ",
        };
        let source_url = format!("{}/blob/{}/{}", base_url, commit_hash, source_id);
        output.push_str(&format!("| [{}{}]({}) |", indentation, short_name, source_url));

        let is_verified = matrix_data.get(source_id).map_or(false, |targets| !targets.is_empty());
        output.push_str(if is_verified { " ✅ |" } else { " ❌ |" });

        for target in &relevant_targets {
            let target_id = &target.identifier;
            if matrix_data.get(source_id).map_or(false, |targets| targets.contains(target_id)) {
                output.push_str(" ✔️ |");
            } else {
                output.push_str("   |");
            }
        }
        output.push_str("\n");
    }
}


/// Generates a Markdown representation of the traceability matrix
fn generate_markdown_matrix(
    matrix_data: &HashMap<String, HashSet<String>>,
    source_elements: &[&Element],
    target_elements: &[&Element],
    relation_types: &[&str],
    registry: &GraphRegistry,
    base_url: &str,
    commit_hash: &str,
) -> String {
    let mut output = String::new();

    // Title and intro
    output.push_str("# Traceability Matrix\n\n");
    output.push_str("This matrix shows relationships between requirements and verification elements, grouped by root requirements.\n\n");

    // Relation types
    output.push_str("## Relation Types Used\n\n");
    for rel_type in relation_types {
        if let Some(info) = RELATION_TYPES.get(*rel_type) {
            output.push_str(&format!("- **{}**: {}\n", rel_type, info.description));
        } else {
            output.push_str(&format!("- **{}**\n", rel_type));
        }
    }
    output.push_str("\n");

    // Sort target elements
    let mut sorted_target_elements = target_elements.to_vec();
    sorted_target_elements.sort_by(|a, b| a.identifier.cmp(&b.identifier));

    // Grouping logic from old script
    let requirements_by_root = registry.get_requirements_by_root();

    if requirements_by_root.is_empty() {
        output.push_str("## All Requirements\n\n");
        generate_matrix_table(
            &sorted_target_elements,
            source_elements,
            matrix_data,
            &mut output,
            base_url,
            commit_hash,
        );
    } else {
        let mut root_ids: Vec<String> = requirements_by_root.keys().cloned().collect();
        root_ids.sort();

        for root_id in &root_ids {
            output.push_str(&format!("## {} Requirements\n\n", root_id));
            if let Some(group_elements) = requirements_by_root.get(root_id) {
                // Only include source elements (requirements) in each group
                let group_source_elements: Vec<&Element> = group_elements.iter()
                    .filter(|elem| source_elements.iter().any(|src| src.identifier == elem.identifier))
                    .cloned()
                    .collect();

                if !group_source_elements.is_empty() {
                    generate_matrix_table(
                        &sorted_target_elements,
                        &group_source_elements,
                        matrix_data,
                        &mut output,
                        base_url,
                        commit_hash,
                    );
                }
            }
            output.push_str("\n");
        }
    }

    // Legend
    output.push_str("## Legend\n\n");
    output.push_str("- ✅ (in 'Verified' column): Requirement is verified by at least one verification element\n");
    output.push_str("- ❌ (in 'Verified' column): Requirement is not verified by any verification element\n");
    output.push_str("- ✔️ (in element columns): Direct relationship exists between requirement and verification\n");

    output
}


/// Generates a JSON representation of the traceability matrix
fn generate_json_matrix(
    matrix_data: &HashMap<String, HashSet<String>>,
    source_elements: &[&Element],
    target_elements: &[&Element],
    relation_types: &[&str],
    base_url: &str,
    commit_hash: &str,
) -> String {
    let metadata = json!({
        "relation_types": relation_types,
        "source_count": source_elements.len(),
        "target_count": target_elements.len(),
    });

    // Collect sources
    let sources = source_elements.iter().map(|e| {
        let url = format!("{}/blob/{}/{}", base_url, commit_hash, &e.identifier);
        json!({"id": &e.identifier, "name": e.name, "url": url})
    }).collect::<Vec<_>>();

    // Collect targets
    let targets = target_elements.iter().map(|e| {
        let url = format!("{}/blob/{}/{}", base_url, commit_hash, &e.identifier);
        json!({"id": &e.identifier, "name": e.name, "url": url})
    }).collect::<Vec<_>>();

    // Collect matrix data
    let matrix = matrix_data.iter().map(|(source_id, targets)| {
        (source_id, targets)
    }).collect::<HashMap<_, _>>();

    // Create verification status for each requirement
    let verification_status = source_elements.iter().map(|e| {
        let is_verified = matrix_data.get(&e.identifier).map_or(false, |targets| !targets.is_empty());
        (&e.identifier, is_verified)
    }).collect::<HashMap<_, _>>();

    let output = json!({
        "metadata": metadata,
        "sources": sources,
        "targets": targets,
        "matrix": matrix,
        "verificationStatus": verification_status
    });

    serde_json::to_string_pretty(&output).unwrap()
}

/// Generates a simple SVG representation of the traceability matrix
fn generate_svg_matrix(
    matrix_data: &HashMap<String, HashSet<String>>,
    source_elements: &[&Element],
    target_elements: &[&Element],
) -> String {
    // Determine column widths based on content
    // First, calculate the max width needed for source element names
    let source_max_width = source_elements.iter()
        .map(|e| e.name.len())
        .max()
        .unwrap_or(20) as i32;
    
    // Calculate width for each target element individually
    let target_column_widths: Vec<i32> = target_elements.iter()
        .map(|e| {
            // Calculate width based on text length with extra padding
            // Use 8px per character plus extra padding (40px) to ensure enough space
            // Also ensure a minimum width for very short names
            (e.name.len() as i32 * 8 + 40).max(100)
        })
        .collect();
    
    // Convert character lengths to pixel widths
    let source_column_width = (source_max_width * 8 + 30).max(180);
    let verified_column_width = 60; // Narrower column for verification status
    
    // Calculate total width needed
    let _total_columns = 2 + target_elements.len() as i32; // Requirements + Verified + All targets
    let total_width = source_column_width + verified_column_width + target_column_widths.iter().sum::<i32>() + 40;
    let total_height = 100 + (source_elements.len() as i32 * 30) + 50; // Header + rows + margin
    
    // Create SVG with calculated dimensions
    let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" width=\"{}\" height=\"{}\">\n",
        total_width, total_height, total_width, total_height);
    
    // Add explicit white background rectangle that covers the entire area
    svg.push_str(&format!("<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" fill=\"white\" />\n",
        total_width, total_height));
    
    // Add styles
    svg.push_str("<style>\n");
    svg.push_str("  .header { font-family: sans-serif; font-weight: bold; font-size: 14px; }\n");
    svg.push_str("  .cell { font-family: sans-serif; font-size: 12px; }\n");
    svg.push_str("  .verified { fill: #5fd75f; }\n");
    svg.push_str("  .unverified { fill: #f55f5f; }\n");
    svg.push_str("</style>\n");
    
    // Add title
    svg.push_str("<text x=\"20\" y=\"30\" class=\"header\" font-size=\"18\">Traceability Matrix</text>\n");
    
    // Constants
    let cell_height = 30;
    let start_x = 20;
    let start_y = 60;
    let header_offset = 40;
    
    // Draw table header - first column (Requirements)
    svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#f0f0f0\" stroke=\"#333\" />\n", 
        start_x, start_y, source_column_width, cell_height));
    svg.push_str(&format!("<text x=\"{}\" y=\"{}\" class=\"header\">Requirement</text>\n", 
        start_x + 10, start_y + 20));
    
    // Second column (Verified)
    let verified_col_x = start_x + source_column_width;
    svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#f0f0f0\" stroke=\"#333\" />\n", 
        verified_col_x, start_y, verified_column_width, cell_height));
    svg.push_str(&format!("<text x=\"{}\" y=\"{}\" class=\"header\" text-anchor=\"middle\">Verified</text>\n", 
        verified_col_x + verified_column_width/2, start_y + 20));
    
    // Verification elements header columns
    let mut current_x = verified_col_x + verified_column_width;
    for (i, target) in target_elements.iter().enumerate() {
        let column_width = target_column_widths[i];
        
        svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#f0f0f0\" stroke=\"#333\" />\n", 
            current_x, start_y, column_width, cell_height));
        
        // Display full name
        svg.push_str(&format!("<text x=\"{}\" y=\"{}\" class=\"header\" text-anchor=\"middle\">{}</text>\n", 
            current_x + column_width/2, start_y + 20, target.name));
        
        current_x += column_width;
    }
    
    // Build parent-child hierarchy for indentation
    let mut parent_to_children: HashMap<String, Vec<&Element>> = HashMap::new();
    let parent_relation_types = crate::relation::get_parent_relation_types();
    let mut hierarchical_elements = Vec::new();

    for source in source_elements {
        let mut has_parent = false;
        for relation in &source.relations {
            if parent_relation_types.contains(&relation.relation_type.name) {
                if let LinkType::Identifier(parent_id) = &relation.target.link {
                    parent_to_children.entry(parent_id.clone()).or_default().push(source);
                    has_parent = true;
                    break;
                }
            }
        }
        if !has_parent {
            hierarchical_elements.push((source, 0)); // No parent = top-level
        }
    }

    fn add_svg_children<'a>(
        element: &'a Element,
        level: usize,
        result: &mut Vec<(&'a Element, usize)>,
        parent_to_children: &HashMap<String, Vec<&'a Element>>,
    ) {
        result.push((element, level));
        if let Some(children) = parent_to_children.get(&element.identifier) {
            let mut sorted_children = children.clone();
            sorted_children.sort_by(|a, b| a.name.cmp(&b.name));
            for child in sorted_children {
                add_svg_children(child, level + 1, result, parent_to_children);
            }
        }
    }

    let mut sorted_hierarchy = Vec::new();
    let mut sorted_roots = hierarchical_elements.clone();
    sorted_roots.sort_by(|(a, _), (b, _)| a.name.cmp(&b.name));
    for (element, level) in sorted_roots {
        add_svg_children(element, level, &mut sorted_hierarchy, &parent_to_children);
    }
    
    // Draw rows for each requirement with indentation
    for (i, (source, level)) in sorted_hierarchy.iter().enumerate() {
        let row_y = start_y + header_offset + i as i32 * cell_height;
        
        // Requirement name cell
        svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"white\" stroke=\"#333\" />\n", 
            start_x, row_y, source_column_width, cell_height));
        
        // Apply indentation based on level
        let indentation = match level {
            0 => "",
            1 => "↳ ",
            2 => "__↳ ",
            3 => "____↳ ",
            _ => "______↳ ",
        };
        
        // Display full name with indentation
        svg.push_str(&format!("<text x=\"{}\" y=\"{}\" class=\"cell\">{}{}</text>\n", 
            start_x + 10, row_y + 20, indentation, source.name));
        
        // Verification status cell
        let is_verified = matrix_data.get(&source.identifier).map_or(false, |targets| !targets.is_empty());
        let status_class = if is_verified { "verified" } else { "unverified" };
        let status_symbol = if is_verified { "✅" } else { "❌" };
        
        svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"white\" stroke=\"#333\" />\n", 
            verified_col_x, row_y, verified_column_width, cell_height));
        svg.push_str(&format!("<text x=\"{}\" y=\"{}\" class=\"cell {}\" text-anchor=\"middle\">{}</text>\n", 
            verified_col_x + verified_column_width/2, row_y + 20, status_class, status_symbol));
        
        // Cells for each verification element
        current_x = verified_col_x + verified_column_width;
        for (i, target) in target_elements.iter().enumerate() {
            let column_width = target_column_widths[i];
            
            let has_relation = matrix_data.get(&(*source).identifier)
                .map_or(false, |targets| targets.contains(&target.identifier));
            
            svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"white\" stroke=\"#333\" />\n", 
                current_x, row_y, column_width, cell_height));
            
            if has_relation {
                svg.push_str(&format!("<text x=\"{}\" y=\"{}\" class=\"cell verified\" text-anchor=\"middle\">✅</text>\n", 
                    current_x + column_width/2, row_y + 20));
            }
            
            current_x += column_width;
        }
    }
    
    // Add legend
    let legend_y = start_y + header_offset + source_elements.len() as i32 * cell_height + 30;
    svg.push_str(&format!("<text x=\"{}\" y=\"{}\" class=\"header\">Legend:</text>\n", start_x, legend_y));
    svg.push_str(&format!("<text x=\"{}\" y=\"{}\" class=\"cell\">✅ Verified requirement | ❌ Unverified requirement</text>\n", 
        start_x + 80, legend_y));
    
    // Close SVG tag
    svg.push_str("</svg>");
    svg
}

/// Helper function to get a short display name for an element
fn get_short_element_name(element: &Element) -> String {
    let name_parts: Vec<&str> = element.name.split('/').collect();
    if let Some(last_part) = name_parts.last() {
        if last_part.len() > 30 {
            format!("{}...", &last_part[0..27])
        } else {
            last_part.to_string()
        }
    } else {
        if element.name.len() > 30 {
            format!("{}...", &element.name[0..27])
        } else {
            element.name.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::{Element, ElementType, RequirementType, VerificationType};
    use crate::relation::{Relation, RelationTarget};
    use crate::graph_registry::GraphRegistry;
    use crate::relation::LinkType;
    use crate::matrix_generator::{generate_matrix, MatrixConfig, MatrixFormat};

    fn create_mock_registry() -> GraphRegistry {
        let mut registry = GraphRegistry::new();


        let relation_type = RELATION_TYPES.get("verifiedBy").unwrap();
        let _relation = Relation {
            relation_type,
            target: RelationTarget {
                text: "".to_string(),
                link: LinkType::Identifier("tests/TEST-001".to_string()),
            },
            user_created: true,
        };

       // Helper: default metadata
       let empty_metadata: HashMap<String, String> = HashMap::new();


        // Create requirement elements
        let req1 = Element {
            identifier: "reqs/REQ-001".to_string(),
            name: "System Requirement 1".to_string(),
            element_type: ElementType::Requirement(RequirementType::System),
            section: "".to_string(),
            metadata: empty_metadata.clone(),
            hash_impact_content: "".to_string(),
            changed_since_commit: false,
            content: "Requirement content 1".to_string(),
            file_path: "reqs/REQ-001".to_string(),
            relations: vec![],
        };

        let req2 = Element {
            identifier: "reqs/REQ-002".to_string(),
            name: "User Requirement 2".to_string(),
            element_type: ElementType::Requirement(RequirementType::User),
            section: "".to_string(),
            metadata: empty_metadata.clone(),
            hash_impact_content: "".to_string(),
            changed_since_commit: false,
            content: "Requirement content 2".to_string(),
            file_path: "reqs/REQ-002".to_string(),
            relations: vec![],
        };

        // Create verification element
        let ver1 = Element {
            identifier: "tests/TEST-001".to_string(),
            name: "Test Case 1".to_string(),
            element_type: ElementType::Verification(VerificationType::Test),
            section: "".to_string(),
            metadata: empty_metadata.clone(),
            hash_impact_content: "".to_string(),
            changed_since_commit: false,
            content: "Test case content".to_string(),
            file_path: "tests/TEST-001".to_string(),
            relations: vec![],
        };

        // Add relation from req1 to ver1
        let mut req1_with_rel = req1.clone();
        req1_with_rel.relations.push(Relation {
            relation_type: &relation_type,
            target: RelationTarget {
                link: LinkType::Identifier("tests/TEST-001".to_string()),
                text: "".to_string(),
            },
            user_created: true,
        });

        // Register elements with the registry
        registry.register_element(req1_with_rel, "reqs/REQ-001").unwrap();
        registry.register_element(req2, "reqs/REQ-002").unwrap();
        registry.register_element(ver1, "tests/TEST-001").unwrap();

        // Build relations for the matrix generator to work
        let _ = registry.build_relations(&globset::GlobSet::empty());

        registry
    }

    #[test]
    fn test_generate_markdown_matrix() {
        let registry = create_mock_registry();
        let config = MatrixConfig::default();
        let output = generate_matrix(&registry, &config, MatrixFormat::Markdown);


        assert!(output.contains("Traceability Matrix"));
        assert!(output.contains("REQ-001"));
        assert!(output.contains("REQ-002"));
        assert!(output.contains("Test Case 1"));
        assert!(output.contains("✔️")); // Relation mark
    }

    #[test]
    fn test_generate_json_matrix() {
        let registry = create_mock_registry();
        let config = MatrixConfig::default();
        let output = generate_matrix(&registry, &config, MatrixFormat::Json);

        let json_output: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert!(json_output["metadata"].is_object());
        assert_eq!(json_output["sources"].as_array().unwrap().len(), 2);
        assert_eq!(json_output["targets"].as_array().unwrap().len(), 1);
    }
}


