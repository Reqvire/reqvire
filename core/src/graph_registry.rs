use std::collections::{HashMap, BTreeSet, HashSet, BTreeMap};
use std::fs;
use std::path::{Path, PathBuf};
use log::{debug, warn};
use serde::Serialize;

use crate::relation::{self, LinkType, get_parent_relation_types};
use crate::element::{Element, ElementType, RequirementType};
use crate::error::ReqvireError;
use crate::git_commands;
use globset::GlobSet;
use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct SectionKey {
    pub file_path: String,
    pub section_name: String,
}

impl SectionKey {
    pub fn new(file_path: String, section_name: String) -> Self {
        Self {
            file_path,
            section_name,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Page {
    pub frontmatter_content: String,
}

impl Page {
    pub fn new(frontmatter_content: String) -> Self {
        Self {
            frontmatter_content,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Section {
    pub content: String,
    pub section_order: usize,
}

impl Section {
    pub fn new(content: String) -> Self {
        Self {
            content,
            section_order: 0, // Will be set when section is added
        }
    }

    pub fn new_with_order(content: String, section_order: usize) -> Self {
        Self {
            content,
            section_order,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RelationNode {
    pub relation_trigger: String,
    pub element_node: ElementNode,
}

#[derive(Debug, Clone, Serialize)]
pub struct ElementNode {
    pub element: Element,
    pub relations: Vec<RelationNode>,
}

#[derive(Debug, Clone)]
pub struct FormatResult {
    pub files_changed: usize,
    pub diffs: Vec<FileDiff>,
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub file_path: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub prefix: String,
    pub content: String,
    pub color: String,
}

#[derive(Debug)]
pub struct GraphRegistry {
    pub nodes: HashMap<String, ElementNode>,
    pub pages: HashMap<String, Page>,
    pub sections: HashMap<SectionKey, Section>,
}

impl GraphRegistry {
    /// Creates a new empty GraphRegistry
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            pages: HashMap::new(),
            sections: HashMap::new(),
        }
    }

    /// Registers a page with content
    pub fn register_page(&mut self, file_path: String, page_content: String) {
        self.pages.insert(file_path, Page::new(page_content));
    }

    /// Registers a section with content
    pub fn register_section(&mut self, file_path: String, section_name: String, section_content: String) {
        let section_key = SectionKey::new(file_path, section_name);
        self.sections.insert(section_key, Section::new(section_content));
    }

    /// Registers a section with content and order
    pub fn register_section_with_order(&mut self, file_path: String, section_name: String, section_content: String, section_order: usize) {
        let section_key = SectionKey::new(file_path, section_name);
        self.sections.insert(section_key, Section::new_with_order(section_content, section_order));
    }

    /// Registers an element with local validation
    pub fn register_element(&mut self, element: Element, _file_path: &str) -> Result<(), ReqvireError> {
        let element_id = element.identifier.clone();

        if self.nodes.contains_key(&element_id) {
            return Err(ReqvireError::DuplicateElement(element_id));
        }

        self.nodes.insert(element_id, ElementNode {
            element,
            relations: Vec::new(),
        });

        Ok(())
    }

    /// Build relations and validate graph structure
    pub fn build_relations(&mut self, excluded_filename_patterns: &GlobSet) -> Result<Vec<ReqvireError>, ReqvireError> {
        debug!("GraphRegistry: Building relations and validating graph structure");

        // First build the relation graph
        self.build_relation_graph();

        // Add missing opposites
        self.propagate_missing_opposites(excluded_filename_patterns);

        // Validate relations
        let mut errors = self.validate_relations(excluded_filename_patterns)?;

        // Validate cross-component dependencies
        errors.extend(self.validate_cross_component_dependencies()?);

        Ok(errors)
    }

    /// Build the relation graph structure
    fn build_relation_graph(&mut self) {
        let element_ids: Vec<String> = self.nodes.keys().cloned().collect();

        for source_id in &element_ids {
            let mut relation_nodes = Vec::new();

            if let Some(source_node) = self.nodes.get(source_id) {
                for relation in &source_node.element.relations {
                    if let LinkType::Identifier(ref target_id) = relation.target.link {
                        // Only handle relations that propagate impact
                        if relation::IMPACT_PROPAGATION_RELATIONS.contains(&relation.relation_type.name) {
                            if let Some(target_node) = self.nodes.get(target_id) {
                                relation_nodes.push(RelationNode {
                                    relation_trigger: relation.relation_type.name.to_string(),
                                    element_node: target_node.clone(),
                                });
                            }
                        }
                    }
                }
            }

            if let Some(entry) = self.nodes.get_mut(source_id) {
                entry.relations = relation_nodes;
            }
        }
    }

    /// Adds missing opposite relations into the registry (does not return errors).
    fn propagate_missing_opposites(&mut self, excluded_filename_patterns: &GlobSet) {
        log::debug!("Propagating missing opposite relations...");
        let mut to_add: Vec<(String, crate::relation::Relation)> = Vec::new();
        let element_ids: Vec<String> = self.nodes.keys().cloned().collect();
        let md_regex = Regex::new(r"\.md(?:#|$)").unwrap();

        for source_id in &element_ids {
            if let Some(source_node) = self.nodes.get(source_id) {
                for relation in &source_node.element.relations {
                    if let crate::relation::LinkType::Identifier(ref target_id) = relation.target.link {
                        if !md_regex.is_match(target_id) || excluded_filename_patterns.is_match(target_id) {
                            continue;
                        }

                        if let Some(opposite_name) = relation.relation_type.opposite {
                            if let Some(target_node) = self.nodes.get(target_id) {
                                let already_present = target_node.element.relations.iter().any(|r| {
                                    matches!(&r.target.link, crate::relation::LinkType::Identifier(id) if id == source_id)
                                        && r.relation_type.name.eq_ignore_ascii_case(opposite_name)
                                });

                                if !already_present {
                                    if let Some(opposite_relation) =
                                        relation.to_opposite(&source_node.element.name, &source_node.element.identifier)
                                    {
                                        to_add.push((target_id.clone(), opposite_relation));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Apply mutations
        for (target_id, relation) in to_add {
            if let Some(target_node) = self.nodes.get_mut(&target_id) {
                target_node.element.relations.push(relation);
                log::debug!("Added opposite relation to '{}'", target_id);
            }
        }
    }

    /// Validates relations for target existence and element type compatibility.
    fn validate_relations(&self, excluded_filename_patterns: &GlobSet) -> Result<Vec<ReqvireError>, ReqvireError> {
        log::debug!("Running relation validation...");
        let mut errors = Vec::new();
        let element_ids: Vec<String> = self.nodes.keys().cloned().collect();

        for source_id in &element_ids {
            if let Some(source_node) = self.nodes.get(source_id) {
                for relation in &source_node.element.relations {
                    // Only process user-created relations to avoid infinite loops
                    if !relation.user_created {
                        continue;
                    }

                    match &relation.target.link {
                        crate::relation::LinkType::Identifier(ref target_id) => {
                            if excluded_filename_patterns.is_match(target_id) {
                                log::debug!("Skipping excluded target: {}", target_id);
                                continue;
                            }

                            match self.get_element(target_id) {
                                None => {
                                    errors.push(ReqvireError::MissingRelationTarget(
                                        format!("Element '{}' references missing target '{}'", source_node.element.identifier, target_id),
                                    ));
                                }
                                Some(target_element) => {
                                    if let Some(error) = self.validate_element_types(
                                        relation.relation_type.name,
                                        &source_node.element,
                                        target_element,
                                    ) {
                                        errors.push(error);
                                    }
                                }
                            }
                        }
                        crate::relation::LinkType::InternalPath(ref file_path) => {
                            // Validate file existence for InternalPath targets
                            if !file_path.exists() {
                                errors.push(ReqvireError::MissingRelationTarget(
                                    format!("Element '{}' references missing target '{}'",
                                        source_node.element.identifier,
                                        file_path.to_string_lossy()),
                                ));
                            }
                        }
                        crate::relation::LinkType::ExternalUrl(_) => {
                            // Skip validation for external URLs as per specification
                            log::debug!("Skipping external URL validation");
                        }
                    }
                }
            }
        }

        Ok(errors)
    }

    /// Validates element types for a relation and returns an error if validation fails
    fn validate_element_types(
        &self,
        relation_type: &str,
        source_element: &Element,
        target_element: &Element
    ) -> Option<ReqvireError> {
        // Only validate relation types with element type restrictions
        if let Some(expected_types) = crate::relation::get_relation_element_type_description(relation_type) {
            // Check if the element types are compatible
            let is_valid = crate::relation::validate_relation_element_types(
                relation_type,
                &source_element.element_type,
                &target_element.element_type
            );

            if !is_valid {
                return Some(ReqvireError::IncompatibleElementTypes(
                    format!("Relation '{}' from '{}' ({:?}) to '{}' ({:?}) has incompatible element types. {}",
                        relation_type,
                        source_element.identifier,
                        source_element.element_type,
                        target_element.identifier,
                        target_element.element_type,
                        expected_types
                    )
                ));
            }
        }

        None
    }

    /// Validates cross-component dependencies for circular dependencies and missing links.
    fn validate_cross_component_dependencies(&self) -> Result<Vec<ReqvireError>, ReqvireError> {
        debug!("Validating cross-component dependencies...");
        let mut errors = Vec::new();
        let mut visited = HashSet::new();

        // Check for circular dependencies - but be less strict about what constitutes a cycle
        for element_node in self.nodes.values() {
            let mut path = Vec::new();
            self.check_circular_dependencies(&element_node.element, &mut visited, &mut path, &mut errors);
        }

        // Check for missing parent relations
        let valid_parent_relations = get_parent_relation_types();
        for element_node in self.nodes.values() {
            let element = &element_node.element;
            let element_file = &element.file_path;

            // Important: Only system requirements needs parent
            if let ElementType::Requirement(req_type) = &element.element_type {
                match req_type {
                    RequirementType::User => continue,
                    RequirementType::System => {
                        let has_parent_relation = element.relations.iter()
                            .any(|r| valid_parent_relations.contains(&r.relation_type.name));

                        if !has_parent_relation {
                            errors.push(ReqvireError::MissingParentRelation(
                                format!("File {}: Element '{}' has no parent relation (needs one of: {:?})", element_file, element.name, valid_parent_relations),
                            ));
                        }
                    }
                }
            }
        }

        if errors.is_empty() {
            debug!("No cross-component dependency validation errors found.");
        } else {
            debug!("{} cross-component validation errors found.", errors.len());
        }

        Ok(errors)
    }

    /// Check for circular dependencies using relation metadata to traverse in canonical direction
    fn check_circular_dependencies(
        &self,
        element: &Element,
        visited: &mut HashSet<String>,
        path: &mut Vec<String>,
        errors: &mut Vec<ReqvireError>,
    ) {
        let element_id = element.identifier.clone();

        // If we've already fully processed this element, no need to check again.
        if visited.contains(&element_id) {
            return;
        }

        // If the current path already contains this element, we've found a cycle.
        if let Some(pos) = path.iter().position(|id| id == &element_id) {
            let cycle = path[pos..].join(" -> ");
            let full_cycle = format!("{} -> {}", cycle, element_id);
            errors.push(ReqvireError::CircularDependencyError(
                format!("Circular dependency error: {}", full_cycle),
            ));
            return;
        }

        // Add this element to the current traversal path.
        path.push(element_id.clone());

        // Traverse relations using their metadata to determine canonical direction
        for relation in &element.relations {
            if let crate::relation::LinkType::Identifier(ref target_id) = relation.target.link {
                // Use relation metadata to traverse in canonical direction only
                let should_traverse = if let Some(_opposite) = relation.relation_type.opposite {
                    // For bidirectional relations, only traverse in one canonical direction
                    // to avoid detecting the same logical cycle twice
                    // Traverse if this relation type is "lexicographically smaller" than its opposite
                    // or if this is the primary direction for this relation type
                    relation.relation_type.name < relation.relation_type.opposite.unwrap_or("")
                } else {
                    // For unidirectional relations, always traverse
                    true
                };

                if should_traverse {
                    if let Some(target_element) = self.get_element(target_id) {
                        self.check_circular_dependencies(target_element, visited, path, errors);
                    }
                }
            }
        }

        // Mark the current element as completely processed and remove it from the current path.
        visited.insert(element_id);
        path.pop();
    }

    /// Updates an element's identifier and rewires all incoming relations
    pub fn update_identifier(&mut self, old_id: &str, new_id: &str) {
        if let Some(mut node) = self.nodes.remove(old_id) {
            node.element.identifier = new_id.to_string();

            // Update relations within this element (if any self-refs)
            for relation in &mut node.element.relations {
                if let LinkType::Identifier(ref mut link_id) = relation.target.link {
                    if link_id == old_id {
                        *link_id = new_id.to_string();
                    }
                }
            }

            // Reinsert with new ID
            self.nodes.insert(new_id.to_string(), node);

            // Update all relations pointing to this identifier
            for (_id, other_node) in self.nodes.iter_mut() {
                for relation in &mut other_node.element.relations {
                    if let LinkType::Identifier(ref mut link_id) = relation.target.link {
                        if link_id == old_id {
                            *link_id = new_id.to_string();
                        }
                    }
                }

                for relation_node in &mut other_node.relations {
                    if relation_node.element_node.element.identifier == old_id {
                        relation_node.element_node.element.identifier = new_id.to_string();
                    }
                }
            }
        }
    }

    /// Moves an element to an existing location in the graph (file and section must already exist)
    pub fn move_element_to_location(&mut self, element_id: &str, new_file_path: &str, new_section: &str) -> Result<(), ReqvireError> {
        // Verify the target location exists in the graph
        let target_exists = self.nodes.values().any(|node| {
            node.element.file_path == new_file_path && node.element.section == new_section
        });

        if !target_exists {
            return Err(ReqvireError::LocationNotFound(format!(
                "Target location '{}:{}' does not exist in the graph",
                new_file_path, new_section
            )));
        }

        if let Some(node) = self.nodes.get_mut(element_id) {
            let old_file_path = node.element.file_path.clone();
            let old_section = node.element.section.clone();

            node.element.file_path = new_file_path.to_string();
            node.element.section = new_section.to_string();

            // Update the element in all relation nodes that reference it
            for (_id, other_node) in self.nodes.iter_mut() {
                for relation_node in &mut other_node.relations {
                    if relation_node.element_node.element.identifier == element_id {
                        relation_node.element_node.element.file_path = new_file_path.to_string();
                        relation_node.element_node.element.section = new_section.to_string();
                    }
                }
            }

            log::debug!(
                "Moved element '{}' from '{}:{}' to '{}:{}'",
                element_id, old_file_path, old_section, new_file_path, new_section
            );

            Ok(())
        } else {
            Err(ReqvireError::MissingElement(format!("Element '{}' not found in graph", element_id)))
        }
    }

    /// Adds a new section to an existing file in the graph (virtual - no filesystem changes)
    pub fn add_section_to_file(&mut self, file_path: &str, new_section: &str) -> Result<(), ReqvireError> {
        // Verify the file exists in the graph
        let file_exists = self.nodes.values().any(|node| node.element.file_path == file_path);

        if !file_exists {
            return Err(ReqvireError::LocationNotFound(format!("File '{}' does not exist in the graph", file_path)));
        }

        // Check if the section already exists
        let section_exists = self.nodes.values().any(|node| {
            node.element.file_path == file_path && node.element.section == new_section
        });

        if section_exists {
            return Err(ReqvireError::LocationAlreadyExists(format!("Section '{}' already exists in file '{}'", new_section, file_path)));
        }

        // Create a virtual placeholder element to track this location
        let virtual_id = format!("__virtual__{}#{}", file_path, new_section);
        let virtual_element = Element::new(
            &format!("Virtual placeholder for {}", new_section),
            &virtual_id,
            file_path,
            new_section,
            None,
        );

        self.nodes.insert(virtual_id, ElementNode {
            element: virtual_element,
            relations: Vec::new(),
        });

        log::debug!("Added virtual section '{}' to file '{}'", new_section, file_path);
        Ok(())
    }

    /// Adds a new file location to the graph (virtual - no filesystem changes)
    pub fn add_file_location(&mut self, new_file_path: &str, initial_section: &str) -> Result<(), ReqvireError> {
        // Check if the file already exists
        let file_exists = self.nodes.values().any(|node| node.element.file_path == new_file_path);

        if file_exists {
            return Err(ReqvireError::LocationAlreadyExists(format!("File '{}' already exists in the graph", new_file_path)));
        }

        // Create a virtual placeholder element to track this file location
        let virtual_id = format!("__virtual__{}#{}", new_file_path, initial_section);
        let virtual_element = Element::new(
            &format!("Virtual placeholder for {}", new_file_path),
            &virtual_id,
            new_file_path,
            initial_section,
            None,
        );

        self.nodes.insert(virtual_id, ElementNode {
            element: virtual_element,
            relations: Vec::new(),
        });

        log::debug!("Added virtual file location '{}' with initial section '{}'", new_file_path, initial_section);
        Ok(())
    }

    /// Moves element to a new section in an existing file (creates section if needed)
    pub fn move_element_to_new_section(&mut self, element_id: &str, file_path: &str, new_section: &str) -> Result<(), ReqvireError> {
        // Verify the file exists in the graph
        let file_exists = self.nodes.values().any(|node| node.element.file_path == file_path);

        if !file_exists {
            return Err(ReqvireError::LocationNotFound(format!("File '{}' does not exist in the graph", file_path)));
        }

        // Check if section exists, if not, create it virtually
        let section_exists = self.nodes.values().any(|node| {
            node.element.file_path == file_path && node.element.section == new_section
        });

        if !section_exists {
            self.add_section_to_file(file_path, new_section)?;
        }

        if let Some(node) = self.nodes.get_mut(element_id) {
            let old_file_path = node.element.file_path.clone();
            let old_section = node.element.section.clone();

            node.element.file_path = file_path.to_string();
            node.element.section = new_section.to_string();

            // Update the element in all relation nodes that reference it
            for (_id, other_node) in self.nodes.iter_mut() {
                for relation_node in &mut other_node.relations {
                    if relation_node.element_node.element.identifier == element_id {
                        relation_node.element_node.element.file_path = file_path.to_string();
                        relation_node.element_node.element.section = new_section.to_string();
                    }
                }
            }

            log::debug!(
                "Moved element '{}' from '{}:{}' to new section '{}:{}'",
                element_id, old_file_path, old_section, file_path, new_section
            );

            Ok(())
        } else {
            Err(ReqvireError::MissingElement(format!("Element '{}' not found in graph", element_id)))
        }
    }

    /// Moves element to a new file location (creates file location if needed)
    pub fn move_element_to_new_file(&mut self, element_id: &str, new_file_path: &str, new_section: &str) -> Result<(), ReqvireError> {
        // Check if file exists, if not, create it virtually
        let file_exists = self.nodes.values().any(|node| node.element.file_path == new_file_path);

        if !file_exists {
            self.add_file_location(new_file_path, new_section)?;
        }

        if let Some(node) = self.nodes.get_mut(element_id) {
            let old_file_path = node.element.file_path.clone();
            let old_section = node.element.section.clone();

            node.element.file_path = new_file_path.to_string();
            node.element.section = new_section.to_string();

            // Update the element in all relation nodes that reference it
            for (_id, other_node) in self.nodes.iter_mut() {
                for relation_node in &mut other_node.relations {
                    if relation_node.element_node.element.identifier == element_id {
                        relation_node.element_node.element.file_path = new_file_path.to_string();
                        relation_node.element_node.element.section = new_section.to_string();
                    }
                }
            }

            // Update relation identifiers for cross-file references
            self.update_relation_identifiers(element_id, &old_file_path, new_file_path);

            log::debug!(
                "Moved element '{}' from '{}:{}' to new file '{}:{}'",
                element_id, old_file_path, old_section, new_file_path, new_section
            );

            Ok(())
        } else {
            Err(ReqvireError::MissingElement(format!("Element '{}' not found in graph", element_id)))
        }
    }

    /// Gets all available locations (file:section combinations) in the graph
    pub fn get_available_locations(&self) -> Vec<(String, String)> {
        let mut locations = std::collections::BTreeSet::new();

        for node in self.nodes.values() {
            locations.insert((node.element.file_path.clone(), node.element.section.clone()));
        }

        locations.into_iter().collect()
    }

    /// Gets all elements that would be affected by moving the specified element
    pub fn get_move_impact(&self, element_id: &str) -> Vec<String> {
        let mut affected_elements = Vec::new();

        // Find all elements that reference this element
        for (id, node) in &self.nodes {
            if id == element_id {
                continue; // Skip the element being moved
            }

            // Check if this element has relations pointing to the element being moved
            let has_reference = node.element.relations.iter().any(|relation| {
                matches!(&relation.target.link, LinkType::Identifier(link_id) if link_id == element_id)
            });

            if has_reference {
                affected_elements.push(id.clone());
            }
        }

        affected_elements.sort();
        affected_elements
    }

    pub fn get_impact_tree(&self, root_id: &str) -> ElementNode {
        let mut visited = BTreeSet::new();
        self.build_impact_tree_recursive(root_id, &mut visited)
    }

    fn build_impact_tree_recursive(&self, current_id: &str, visited: &mut BTreeSet<String>) -> ElementNode {
        if !visited.insert(current_id.to_string()) {
            // Already visited, stop recursion to prevent cycles
            let current_node = self.nodes.get(current_id).unwrap();
            return ElementNode {
                element: current_node.element.clone(),
                relations: Vec::new(), // Empty relations to break the cycle
            };
        }

        let current_node = self.nodes.get(current_id).unwrap();
        let mut child_nodes = Vec::new();

        for relation_node in &current_node.relations {
            let target_id = &relation_node.element_node.element.identifier;

            // Skip relations to already visited nodes to prevent cycles
            if visited.contains(target_id) {
                continue;
            }

            let subtree = self.build_impact_tree_recursive(target_id, visited);
            child_nodes.push(RelationNode {
                relation_trigger: relation_node.relation_trigger.clone(),
                element_node: subtree,
            });
        }

        ElementNode {
            element: current_node.element.clone(),
            relations: child_nodes,
        }
    }

    /// Gets all elements as a vector
    pub fn get_all_elements(&self) -> Vec<&Element> {
        self.nodes.values().map(|node| &node.element).collect()
    }

    /// Collects all InternalPath targets from element relations
    pub fn get_internal_path_targets(&self) -> HashSet<PathBuf> {
        self.collect_internal_path_targets()
    }

    /// Gets requirements grouped by root folder
    pub fn get_requirements_by_root(&self) -> BTreeMap<String, Vec<&Element>> {
        let mut requirements_by_root = BTreeMap::new();
        let parent_relation_types = ["containedBy", "derivedFrom", "refine", "satisfy", "verify"];

        let all_elements = self.get_all_elements();

        // Find root elements (elements without parent relations)
        let root_elements: Vec<&Element> = all_elements.iter()
            .filter(|element| {
                !element.relations.iter().any(|rel| {
                    parent_relation_types.contains(&rel.relation_type.name)
                })
            })
            .copied()
            .collect();

        // For each root element, find all its descendants recursively
        for root_element in &root_elements {
            let mut descendants = vec![*root_element];
            self.collect_descendants(&all_elements, &mut descendants);
            requirements_by_root.insert(root_element.name.clone(), descendants);
        }

        // If no root elements found, group by file path as fallback
        if requirements_by_root.is_empty() {
            for element in &all_elements {
                let root_folder = if let Some(slash_pos) = element.file_path.find('/') {
                    element.file_path[..slash_pos].to_string()
                } else {
                    "root".to_string()
                };
                requirements_by_root
                    .entry(root_folder)
                    .or_insert_with(Vec::new)
                    .push(*element);
            }
        }

        requirements_by_root
    }

    /// Recursively collect all descendants of the elements already in descendants
    fn collect_descendants<'a>(&self, all_elements: &[&'a Element], descendants: &mut Vec<&'a Element>) {
        let mut found_new = true;

        while found_new {
            found_new = false;
            let descendants_len = descendants.len();

            for element in all_elements {
                // Skip if already collected
                if descendants.iter().any(|d| d.identifier == element.identifier) {
                    continue;
                }

                // Check if this element has a parent relation pointing to any element in descendants
                let has_parent_in_descendants = element.relations.iter().any(|rel| {
                    matches!(&rel.target.link, crate::relation::LinkType::Identifier(target_id)
                        if descendants.iter().any(|d| d.identifier == *target_id)
                        && ["containedBy", "derivedFrom", "refine", "satisfy", "verify"].contains(&rel.relation_type.name))
                });

                if has_parent_in_descendants {
                    descendants.push(*element);
                    found_new = true;
                }
            }

            // If we didn't find any new descendants, break to avoid infinite loop
            if descendants.len() == descendants_len {
                break;
            }
        }
    }

    /// Change impact analysis with relation information
    pub fn change_impact_with_relation(&self, element: &Element) -> Vec<(String, Vec<crate::relation::Relation>)> {
        if let Some(node) = self.nodes.get(&element.identifier) {
            // Group original relations by target ID using BTreeMap for deterministic ordering
            let mut relations_by_target: std::collections::BTreeMap<String, Vec<crate::relation::Relation>> = std::collections::BTreeMap::new();

            for relation in &node.element.relations {
                let target_id = match &relation.target.link {
                    crate::relation::LinkType::Identifier(ref target_id) => target_id.clone(),
                    crate::relation::LinkType::InternalPath(ref path) => path.to_string_lossy().to_string(),
                    crate::relation::LinkType::ExternalUrl(_) => continue, // Skip external URLs for change impact
                };

                relations_by_target
                    .entry(target_id)
                    .or_insert_with(Vec::new)
                    .push(relation.clone());
            }

            relations_by_target.into_iter().collect()
        } else {
            Vec::new()
        }
    }

    /// Gets a specific element by ID
    pub fn get_element(&self, element_id: &str) -> Option<&Element> {
        self.nodes.get(element_id).map(|node| &node.element)
    }

    /// Creates a virtual section in an existing file
    pub fn create_virtual_section(&mut self, file_path: &str, section: &str) -> Result<(), ReqvireError> {
        self.add_section_to_file(file_path, section)
    }

    /// Creates a virtual file with initial section
    pub fn create_virtual_file(&mut self, file_path: &str, section: &str) -> Result<(), ReqvireError> {
        self.add_file_location(file_path, section)
    }

    /// Collects all InternalPath targets from element relations
    pub fn collect_internal_path_targets(&self) -> HashSet<PathBuf> {
        let mut internal_paths = HashSet::new();

        for node in self.nodes.values() {
            for relation in &node.element.relations {
                if let LinkType::InternalPath(ref path) = relation.target.link {
                    internal_paths.insert(path.clone());
                }
            }
        }

        internal_paths
    }


    fn element_to_markdown_with_context(&self, element: &Element, _current_file: &str) -> String {
        let mut markdown = String::new();

        // Add the element header
        markdown.push_str(&format!("### {}\n\n", element.name));

        // Add the element content
        if !element.content.trim().is_empty() {
            let trimmed_content = element.content.trim_end();
            markdown.push_str(trimmed_content);
            // Always add two newlines after content to ensure blank line before subsections
            markdown.push_str("\n\n");
        }

        // Add metadata subsection if there are custom metadata
        let mut custom_metadata: Vec<_> = element.metadata.iter()
            .filter(|(key, _)| *key != "type") // type is handled separately
            .collect();
        custom_metadata.sort_by_key(|(key, _)| *key);

        // Only add metadata section if there are custom metadata or non-default type
        // Default type is only "requirement" - user-requirement should always be explicit
        let has_non_default_type = element.element_type.as_str() != "requirement";

        if !custom_metadata.is_empty() || has_non_default_type {
            markdown.push_str("#### Metadata\n");

            // Add type metadata
            markdown.push_str(&format!("  * type: {}\n", element.element_type.as_str()));

            // Add other metadata
            for (key, value) in custom_metadata {
                markdown.push_str(&format!("  * {}: {}\n", key, value));
            }
            markdown.push_str("\n\n");
        }

        // Add relations subsection if there are user-created relations
        let user_relations: Vec<_> = element.relations.iter().filter(|r| r.user_created).collect();
        if !user_relations.is_empty() {
            markdown.push_str("#### Relations\n");
            for relation in user_relations {
                // Format relation target based on type
                // Format as proper markdown link using element name when possible
                let target_text = match &relation.target.link {
                    LinkType::ExternalUrl(_) => {
                        // For external URLs, use the text as-is (might already be a full URL)
                        relation.target.text.clone()
                    },
                    LinkType::Identifier(target_id) => {
                        // Extract fragment to look up the target element
                        let fragment = if let Some(fragment_pos) = target_id.find('#') {
                            &target_id[fragment_pos + 1..]
                        } else {
                            target_id
                        };

                        // Use actual element name if available, otherwise fallback to fragment conversion
                        let display_name = if let Some(target_node) = self.nodes.get(target_id) {
                            target_node.element.name.clone()
                        } else {
                            // Fallback: convert fragment to title case
                            fragment.replace('-', " ")
                                .split_whitespace()
                                .map(|word| {
                                    let mut chars = word.chars();
                                    match chars.next() {
                                        None => String::new(),
                                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                                    }
                                })
                                .collect::<Vec<String>>()
                                .join(" ")
                        };

                        // Check if target is in the same file
                        let target_file = if let Some(file_pos) = target_id.find('#') {
                            &target_id[..file_pos]
                        } else {
                            target_id
                        };

                        // Get current file path for comparison
                        let current_file_path = std::path::PathBuf::from(_current_file);
                        let current_file_str = _current_file;

                        // If target is in the same file, use just the fragment
                        if target_file.is_empty() || target_file == current_file_str ||
                           target_id.starts_with('#') {
                            format!("[{}](#{})", display_name, fragment)
                        } else {
                            // Make the link relative using just the folder of the current file
                            let current_folder = current_file_path.parent()
                                .unwrap_or_else(|| std::path::Path::new("."))
                                .to_path_buf();

                            let relative_link = crate::utils::to_relative_identifier(
                                relation.target.link.as_str(),
                                &current_folder,
                                false
                            ).unwrap_or_else(|_| relation.target.link.as_str().to_string());

                            format!("[{}]({})", display_name, relative_link)
                        }
                    },
                    LinkType::InternalPath(path) => {
                        // For InternalPath, use the filename as display text and full relative path as link
                        let path_str = path.to_str().unwrap_or("invalid_path");
                        let display_name = std::path::Path::new(path_str)
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or(path_str);

                        // Make the path relative using just the folder of the current file
                        let current_file_path = std::path::PathBuf::from(_current_file);
                        let current_folder = current_file_path.parent()
                            .unwrap_or_else(|| std::path::Path::new("."))
                            .to_path_buf();

                        let relative_link = crate::utils::to_relative_identifier(
                            relation.target.link.as_str(),
                            &current_folder,
                            false
                        ).unwrap_or_else(|_| relation.target.link.as_str().to_string());

                        format!("[{}]({})", display_name, relative_link)
                    }
                };

                markdown.push_str(&format!("  * {}: {}\n",
                    relation.relation_type.name,
                    target_text
                ));
            }
            markdown.push('\n');
        }

        markdown
    }

    /// Groups elements by their file path and section
    fn group_elements_by_location(&self) -> HashMap<String, HashMap<String, Vec<&Element>>> {
        let mut file_sections: HashMap<String, HashMap<String, Vec<&Element>>> = HashMap::new();

        for node in self.nodes.values() {
            let element = &node.element;

            // Skip virtual placeholder elements
            if element.identifier.starts_with("__virtual__") {
                continue;
            }

            file_sections
                .entry(element.file_path.clone())
                .or_insert_with(HashMap::new)
                .entry(element.section.clone())
                .or_insert_with(Vec::new)
                .push(element);
        }

        // Sort elements within each section by their original order index
        for file_map in file_sections.values_mut() {
            for elements in file_map.values_mut() {
                elements.sort_by_key(|element| element.section_order_index);
            }
        }

        file_sections
    }

    /// Generates markdown content for a file
    fn generate_file_markdown(&self, file_path: &str, sections: &HashMap<String, Vec<&Element>>) -> String {
        let mut markdown = String::new();

        // Get file title from file path (remove extension and path)
        let file_title = Path::new(file_path)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .replace('_', " ")
            .replace('-', " ");

        // Check if page content already has a header to avoid duplication
        let mut has_page_header = false;
        if let Some(page) = self.pages.get(file_path) {
            if !page.frontmatter_content.trim().is_empty() {
                // Check if page content starts with a header (# at beginning of line)
                if page.frontmatter_content.trim_start().starts_with('#') {
                    has_page_header = true;
                }
            }
        }

        // Add file header only if page content doesn't already have one
        if !has_page_header {
            markdown.push_str(&format!("# {}\n\n", file_title));
        }

        // Add page content if available
        if let Some(page) = self.pages.get(file_path) {
            if !page.frontmatter_content.trim().is_empty() {
                markdown.push_str(&page.frontmatter_content);
                if !page.frontmatter_content.ends_with('\n') {
                    markdown.push('\n');
                }
                markdown.push('\n');
            }
        }

        // Get all sections for this file (including those without elements)
        let mut all_sections: HashMap<String, Vec<&Element>> = sections.clone();

        // Add sections without elements from the sections registry
        for (section_key, _) in &self.sections {
            if section_key.file_path == file_path {
                all_sections.entry(section_key.section_name.clone()).or_insert_with(Vec::new);
            }
        }

        // Sort sections by their original order (section_order)
        let mut sorted_sections: Vec<_> = all_sections.iter().collect();
        sorted_sections.sort_by_key(|(section_name, _)| {
            let section_key = SectionKey {
                file_path: file_path.to_string(),
                section_name: (*section_name).clone(),
            };
            self.sections.get(&section_key)
                .map(|section| section.section_order)
                .unwrap_or(usize::MAX) // Put sections without order at the end
        });

        for (section_name, elements) in sorted_sections {
            if !section_name.is_empty() && section_name != "Default" {
                markdown.push_str(&format!("## {}\n\n", section_name));
            }

            // Add section content if available
            let section_key = SectionKey {
                file_path: file_path.to_string(),
                section_name: section_name.clone(),
            };
            let has_section_content = if let Some(section) = self.sections.get(&section_key) {
                if !section.content.trim().is_empty() {
                    markdown.push_str(&section.content);
                    if !section.content.ends_with('\n') {
                        markdown.push('\n');
                    }
                    markdown.push('\n');
                    true
                } else {
                    false
                }
            } else {
                false
            };

            // Add elements in this section
            for (i, element) in elements.iter().enumerate() {
                // Add separator before each element
                // - Always before 2nd, 3rd, etc. elements (i > 0)
                // - Before first element if there was section content
                if i > 0 || has_section_content {
                    markdown.push_str("---\n\n");
                }
                markdown.push_str(&self.element_to_markdown_with_context(element, file_path));
            }

            // Add final separator after the last element in the section (if there were any elements)
            if !elements.is_empty() {
                markdown.push_str("---\n\n");
            }
        }

        markdown
    }

    /// Copies InternalPath files to the output directory
    fn copy_internal_path_files(&self, internal_paths: &HashSet<PathBuf>, output_dir: &Path) -> Result<usize, ReqvireError> {
        let base_dir = match git_commands::get_git_root_dir() {
            Ok(git_root) => git_root,
            Err(_) => {
                // If Git repository root can't be found, use the current working directory
                std::env::current_dir()
                    .map_err(|e| ReqvireError::PathError(format!("Failed to get current directory: {}", e)))?
            }
        };

        let mut files_copied = 0;

        for internal_path in internal_paths {
            // Resolve the source path relative to base directory
            let src_path = if internal_path.is_absolute() {
                internal_path.clone()
            } else {
                base_dir.join(internal_path)
            };

            // Skip if source file doesn't exist
            if !src_path.is_file() {
                warn!("Skipping missing InternalPath file: {:?}", src_path);
                continue;
            }

            // Build the destination path
            let dst_path = output_dir.join(internal_path);

            // Create parent directories if needed
            if let Some(parent_dir) = dst_path.parent() {
                fs::create_dir_all(parent_dir)
                    .map_err(|e| ReqvireError::IoError(e))?;
            }

            // Copy the file
            match fs::copy(&src_path, &dst_path) {
                Ok(_) => {
                    debug!("Copied InternalPath file: {:?} -> {:?}", src_path, dst_path);
                    files_copied += 1;
                }
                Err(e) => {
                    warn!("Failed to copy InternalPath file {:?}: {}", src_path, e);
                }
            }
        }

        Ok(files_copied)
    }

    /// Flushes all elements to markdown files and copies InternalPath files to the specified directory
    pub fn flush_to_directory(&self, output_dir: &Path) -> Result<(usize, usize), ReqvireError> {
        // Create output directory if it doesn't exist
        if !output_dir.exists() {
            fs::create_dir_all(output_dir)
                .map_err(|e| ReqvireError::IoError(e))?;
        }

        // Generate and write markdown files
        let grouped_elements = self.group_elements_by_location();
        let mut markdown_files_written = 0;

        for (file_path, sections) in grouped_elements {
            // Generate the markdown content for this file
            let markdown_content = self.generate_file_markdown(&file_path, &sections);

            // Determine the output file path
            let output_file_path = output_dir.join(&file_path);

            // Create parent directories if needed
            if let Some(parent_dir) = output_file_path.parent() {
                fs::create_dir_all(parent_dir)
                    .map_err(|e| ReqvireError::IoError(e))?;
            }

            // Write the markdown file
            fs::write(&output_file_path, markdown_content)
                .map_err(|e| ReqvireError::IoError(e))?;

            debug!("Flushed {} elements to {}",
                sections.values().map(|v| v.len()).sum::<usize>(),
                output_file_path.display()
            );

            markdown_files_written += 1;
        }

        // Copy InternalPath files
        let internal_paths = self.collect_internal_path_targets();
        let internal_files_copied = self.copy_internal_path_files(&internal_paths, output_dir)?;

        log::info!("Successfully flushed {} markdown files and copied {} internal files to {}",
                   markdown_files_written, internal_files_copied, output_dir.display());

        Ok((markdown_files_written, internal_files_copied))
    }

    /// Flushes elements from specific files to markdown files and copies related InternalPath files
    pub fn flush_files_to_directory(&self, file_paths: &[String], output_dir: &Path) -> Result<(usize, usize), ReqvireError> {
        // Create output directory if it doesn't exist
        if !output_dir.exists() {
            fs::create_dir_all(output_dir)
                .map_err(|e| ReqvireError::IoError(e))?;
        }

        let grouped_elements = self.group_elements_by_location();
        let mut markdown_files_written = 0;
        let mut related_internal_paths = HashSet::new();

        for file_path in file_paths {
            if let Some(sections) = grouped_elements.get(file_path) {
                // Generate the markdown content for this file
                let markdown_content = self.generate_file_markdown(file_path, sections);

                // Determine the output file path
                let output_file_path = output_dir.join(file_path);

                // Create parent directories if needed
                if let Some(parent_dir) = output_file_path.parent() {
                    fs::create_dir_all(parent_dir)
                        .map_err(|e| ReqvireError::IoError(e))?;
                }

                // Write the markdown file
                fs::write(&output_file_path, markdown_content)
                    .map_err(|e| ReqvireError::IoError(e))?;

                // Collect InternalPath relations from elements in this file
                for elements in sections.values() {
                    for element in elements {
                        for relation in &element.relations {
                            if let LinkType::InternalPath(ref path) = relation.target.link {
                                related_internal_paths.insert(path.clone());
                            }
                        }
                    }
                }

                debug!("Flushed {} elements to {}",
                    sections.values().map(|v| v.len()).sum::<usize>(),
                    output_file_path.display()
                );

                markdown_files_written += 1;
            }
        }

        // Copy related InternalPath files
        let internal_files_copied = self.copy_internal_path_files(&related_internal_paths, output_dir)?;

        log::info!("Successfully flushed {} markdown files and copied {} internal files to {}",
                   markdown_files_written, internal_files_copied, output_dir.display());

        Ok((markdown_files_written, internal_files_copied))
    }

    /// Format files in place with optional dry-run to show diffs
    /// Uses current working directory as base (respects subdirectory operations)
    pub fn format_files(&self, dry_run: bool) -> Result<FormatResult, ReqvireError> {
        let base_dir = std::env::current_dir()
            .map_err(|e| ReqvireError::PathError(format!("Failed to get current directory: {}", e)))?;

        let grouped_elements = self.group_elements_by_location();
        let mut files_changed = 0;
        let mut files_with_diffs = Vec::new();

        for (file_path, sections) in grouped_elements {
            // Generate the new markdown content for this file
            let mut new_content = self.generate_file_markdown(&file_path, &sections);

            // Apply linting rules to ensure consistent formatting
            new_content = self.apply_formatting_rules(&new_content, &file_path);

            // Construct the full file path relative to current directory
            let full_file_path = base_dir.join(&file_path);

            // Read current content if file exists
            let current_content = if full_file_path.exists() {
                fs::read_to_string(&full_file_path)
                    .map_err(|e| ReqvireError::IoError(e))?
            } else {
                String::new() // File doesn't exist, treat as empty
            };

            // Check if content has changed
            if current_content != new_content {
                files_changed += 1;

                if dry_run {
                    // Generate and store diff
                    let diff = self.generate_file_diff(&file_path, &current_content, &new_content);
                    files_with_diffs.push(diff);
                } else {
                    // Create parent directories if needed
                    if let Some(parent_dir) = full_file_path.parent() {
                        fs::create_dir_all(parent_dir)
                            .map_err(|e| ReqvireError::IoError(e))?;
                    }

                    // Write the new content
                    fs::write(&full_file_path, new_content)
                        .map_err(|e| ReqvireError::IoError(e))?;

                    debug!("Formatted {} with {} elements",
                        file_path, sections.values().map(|v| v.len()).sum::<usize>());
                }
            }
        }

        Ok(FormatResult {
            files_changed,
            diffs: if dry_run { files_with_diffs } else { Vec::new() },
            dry_run,
        })
    }

    /// Apply basic formatting rules to generated markdown content
    fn apply_formatting_rules(&self, content: &str, _file_path: &str) -> String {
        // Since we're generating from the model, we only need basic cleanup
        // of content that comes from original files (element content, page content, section content)

        // Trim extra whitespace at both beginning and end of the content and ensure proper file ending
        let mut formatted = content.trim().to_string();

        // Ensure file ends with exactly one newline
        if !formatted.is_empty() {
            formatted.push('\n');
        }

        formatted
    }


    /// Generate a diff showing changes between current and new content
    fn generate_file_diff(&self, file_path: &str, current: &str, new: &str) -> FileDiff {
        use difference::{Difference, Changeset};

        let changeset = Changeset::new(current, new, "\n");
        let mut diff_lines = Vec::new();

        // Calculate max line numbers to determine padding width
        let max_current_lines = current.lines().count();
        let max_new_lines = new.lines().count();
        let max_line_num = std::cmp::max(max_current_lines, max_new_lines);
        let width = max_line_num.to_string().len();

        let mut old_line_num = 1;
        let mut new_line_num = 1;
        let mut previous_was_change = false;

        let context_lines = 3; // Number of context lines to show before and after changes

        for (i, diff) in changeset.diffs.iter().enumerate() {
            match diff {
                Difference::Same(text) => {
                    let lines: Vec<&str> = text.lines().collect();
                    let line_count = lines.len();

                    // Determine if we should show context lines
                    let next_has_change = changeset.diffs.get(i + 1).map_or(false, |d| !matches!(d, Difference::Same(_)));
                    let show_context = previous_was_change || next_has_change;

                    if show_context && line_count > 0 {
                        // Show context lines
                        let start_lines = if previous_was_change {
                            std::cmp::min(context_lines, line_count)
                        } else {
                            0
                        };
                        let end_lines = if next_has_change {
                            std::cmp::min(context_lines, line_count.saturating_sub(start_lines))
                        } else {
                            0
                        };

                        // Show leading context (after a change)
                        for line_idx in 0..start_lines {
                            diff_lines.push(DiffLine {
                                prefix: format!("{:0width$}", new_line_num + line_idx, width = width),
                                content: format!("    {}", lines[line_idx]),
                                color: "context".to_string(),
                            });
                        }

                        // Show separator if there's a gap in the middle
                        if line_count > start_lines + end_lines && (start_lines > 0 || end_lines > 0) {
                            diff_lines.push(DiffLine {
                                prefix: "".to_string(),
                                content: "".to_string(),
                                color: "separator".to_string(),
                            });
                        }

                        // Show trailing context (before a change)
                        let start_end_lines = line_count.saturating_sub(end_lines);
                        for line_idx in start_end_lines..line_count {
                            diff_lines.push(DiffLine {
                                prefix: format!("{:0width$}", new_line_num + line_idx, width = width),
                                content: format!("    {}", lines[line_idx]),
                                color: "context".to_string(),
                            });
                        }
                    } else if previous_was_change && line_count > context_lines {
                        // Add separator if previous was a change and this is a large gap
                        diff_lines.push(DiffLine {
                            prefix: "".to_string(),
                            content: "".to_string(),
                            color: "separator".to_string(),
                        });
                    }

                    old_line_num += line_count;
                    new_line_num += line_count;
                    previous_was_change = false;
                }
                Difference::Add(text) => {
                    for line in text.lines() {
                        diff_lines.push(DiffLine {
                            prefix: format!("{:0width$}", new_line_num, width = width),
                            content: format!("+   {}", line),
                            color: "green".to_string(),
                        });
                        new_line_num += 1;
                    }
                    previous_was_change = true;
                }
                Difference::Rem(text) => {
                    for line in text.lines() {
                        // Only show special characters for trailing whitespace
                        let visible_line = if line.ends_with(' ') || line.ends_with('\t') {
                            let trimmed = line.trim_end();
                            let trailing = &line[trimmed.len()..];
                            format!("{}{}", trimmed, trailing.replace(' ', "·").replace('\t', "→"))
                        } else {
                            line.to_string()
                        };
                        diff_lines.push(DiffLine {
                            prefix: format!("{:0width$}", new_line_num, width = width),
                            content: format!("-   {}", visible_line),
                            color: "red".to_string(),
                        });
                        old_line_num += 1;
                        // Don't increment new_line_num for removed lines - they don't exist in new file
                    }
                    previous_was_change = true;
                }
            }
        }

        FileDiff {
            file_path: file_path.to_string(),
            lines: diff_lines,
        }
    }

    // Dynamic graph manipulation methods

    /// Updates relation identifiers when elements move between files
    fn update_relation_identifiers(&mut self, moved_element_id: &str, _old_file_path: &str, new_file_path: &str) {
        // 1. Update relations FROM other elements TO the moved element
        for (_id, node) in self.nodes.iter_mut() {
            // Skip the moved element itself for now
            if node.element.identifier == moved_element_id {
                continue;
            }

            // Update relations in the Element object (for markdown generation)
            for relation in &mut node.element.relations {
                if let crate::relation::LinkType::Identifier(ref mut target_id) = relation.target.link {
                    if target_id == moved_element_id {
                        // The target element moved to a different file
                        if node.element.file_path != new_file_path {
                            // Cross-file reference needed
                            *target_id = format!("{}#{}", new_file_path, moved_element_id);
                            relation.target.text = format!("{}#{}", new_file_path, moved_element_id);
                        }
                        // If same file, keep as-is
                    }
                }
            }
        }

        // 2. Update relations FROM the moved element TO other elements
        // First collect target file paths to avoid borrowing issues
        let target_file_paths: std::collections::HashMap<String, String> = self.nodes.values()
            .map(|node| (node.element.identifier.clone(), node.element.file_path.clone()))
            .collect();

        if let Some(moved_node) = self.nodes.get_mut(moved_element_id) {
            for relation in &mut moved_node.element.relations {
                if let crate::relation::LinkType::Identifier(ref mut target_id) = relation.target.link {
                    // Extract the original target identifier (remove any file path prefix)
                    let original_target_id = if target_id.contains('#') {
                        target_id.split('#').last().unwrap_or("").to_string()
                    } else {
                        target_id.clone()
                    };

                    // Find the target element to check its file location
                    if let Some(target_file_path) = target_file_paths.get(&original_target_id) {
                        // If moved element is now in different file than target
                        if new_file_path != target_file_path {
                            // Update to cross-file reference
                            *target_id = format!("{}#{}", target_file_path, original_target_id);
                            relation.target.text = format!("{}#{}", target_file_path, original_target_id);
                        } else {
                            // Same file, use simple reference
                            *target_id = original_target_id.clone();
                            relation.target.text = original_target_id;
                        }
                    }
                }
            }
        }
    }

    /// Adds a new element to the graph
    pub fn add_element(&mut self, element: Element) -> Result<(), ReqvireError> {
        let element_id = element.identifier.clone();

        if self.nodes.contains_key(&element_id) {
            return Err(ReqvireError::ElementMoveError(format!("Element '{}' already exists in the graph", element_id)));
        }

        self.nodes.insert(element_id, ElementNode {
            element,
            relations: Vec::new(),
        });

        Ok(())
    }

    /// Removes an element from the graph and all relations pointing to it
    pub fn remove_element(&mut self, element_id: &str) -> Result<(), ReqvireError> {
        if !self.nodes.contains_key(element_id) {
            return Err(ReqvireError::LocationNotFound(format!("Element '{}' not found in the graph", element_id)));
        }

        // Remove the element itself
        self.nodes.remove(element_id);

        // Remove all relations pointing to this element
        for node in self.nodes.values_mut() {
            node.relations.retain(|rel| rel.element_node.element.identifier != element_id);
        }

        Ok(())
    }

    /// Adds a relation between two elements in the graph
    pub fn add_relation(&mut self, source_id: &str, target_id: &str, relation_type: &str) -> Result<(), ReqvireError> {
        // Validate both elements exist
        if !self.nodes.contains_key(source_id) {
            return Err(ReqvireError::LocationNotFound(format!("Source element '{}' not found", source_id)));
        }
        if !self.nodes.contains_key(target_id) {
            return Err(ReqvireError::LocationNotFound(format!("Target element '{}' not found", target_id)));
        }

        // Check if relation type is valid for impact propagation
        if !relation::IMPACT_PROPAGATION_RELATIONS.contains(&relation_type) {
            return Err(ReqvireError::ProcessError(format!("Relation type '{}' is not valid for impact propagation", relation_type)));
        }

        // Get the target node to create the relation
        let target_node = self.nodes.get(target_id).unwrap().clone();

        // Add the relation to the source element
        let source_node = self.nodes.get_mut(source_id).unwrap();

        // Check if relation already exists
        let relation_exists = source_node.relations.iter().any(|rel|
            rel.element_node.element.identifier == target_id && rel.relation_trigger == relation_type
        );

        if relation_exists {
            return Err(ReqvireError::ProcessError(format!("Relation '{}' from '{}' to '{}' already exists", relation_type, source_id, target_id)));
        }

        source_node.relations.push(RelationNode {
            relation_trigger: relation_type.to_string(),
            element_node: target_node,
        });

        Ok(())
    }

    /// Removes a specific relation between two elements
    pub fn remove_relation(&mut self, source_id: &str, target_id: &str, relation_type: &str) -> Result<(), ReqvireError> {
        if !self.nodes.contains_key(source_id) {
            return Err(ReqvireError::LocationNotFound(format!("Source element '{}' not found", source_id)));
        }

        let source_node = self.nodes.get_mut(source_id).unwrap();
        let initial_count = source_node.relations.len();

        source_node.relations.retain(|rel|
            !(rel.element_node.element.identifier == target_id && rel.relation_trigger == relation_type)
        );

        if source_node.relations.len() == initial_count {
            return Err(ReqvireError::ProcessError(format!("Relation '{}' from '{}' to '{}' not found", relation_type, source_id, target_id)));
        }

        Ok(())
    }

    /// Lists all relations for a given element
    pub fn list_relations(&self, element_id: &str) -> Result<Vec<(String, String)>, ReqvireError> {
        let node = self.nodes.get(element_id)
            .ok_or_else(|| ReqvireError::LocationNotFound(format!("Element '{}' not found", element_id)))?;

        let relations = node.relations.iter()
            .map(|rel| (rel.relation_trigger.clone(), rel.element_node.element.identifier.clone()))
            .collect();

        Ok(relations)
    }

    /// Gets statistics about the graph
    pub fn get_graph_stats(&self) -> (usize, usize) {
        let element_count = self.nodes.len();
        let relation_count = self.nodes.values()
            .map(|node| node.relations.len())
            .sum();

        (element_count, relation_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::{Element, ElementType, RequirementType};
    use crate::relation::{Relation, RelationTarget, LinkType, RELATION_TYPES};

    fn make_element(id: &str, name: &str) -> Element {
        let mut element = Element::new(
            name,
            id,
            "file.md",
            "TestSection",
            Some(ElementType::Requirement(RequirementType::System)),
        );
        element.content = format!("This is {}", name);
        element.freeze_content();
        element
    }

    fn add_relation(from: &mut Element, relation_type: &'static str, to_id: &str) {
        let relation_info = RELATION_TYPES.get(relation_type).unwrap();
        from.relations.push(Relation {
            relation_type: relation_info,
            target: RelationTarget {
                text: to_id.to_string(),
                link: LinkType::Identifier(to_id.to_string()),
            },
            user_created: true,
        });
    }

    #[test]
    fn test_graph_from_registry_resolves_forward_links() {
        let mut registry = GraphRegistry::new();
        let mut a = make_element("A", "Element A");
        let b = make_element("B", "Element B");

        add_relation(&mut a, "contain", "B");

        registry.register_element(a.clone(), "file.md").unwrap();
        registry.register_element(b.clone(), "file.md").unwrap();

        let mut graph = registry;
        graph.build_relation_graph();

        let a_node = graph.nodes.get("A").unwrap();
        assert_eq!(a_node.relations.len(), 1);
        assert_eq!(a_node.relations[0].relation_trigger, "contain");
        assert_eq!(a_node.relations[0].element_node.element.identifier, "B");
    }

    #[test]
    fn test_update_identifier_updates_links_and_graph() {
        let mut registry = GraphRegistry::new();
        let mut a = make_element("A", "Element A");
        let b = make_element("B", "Element B");

        add_relation(&mut a, "contain", "B");

        registry.register_element(a.clone(), "file.md").unwrap();
        registry.register_element(b.clone(), "file.md").unwrap();

        let mut graph = registry;
        graph.build_relation_graph();
        graph.update_identifier("B", "B_NEW");

        // B should no longer exist, B_NEW should
        assert!(graph.nodes.get("B").is_none());
        assert!(graph.nodes.get("B_NEW").is_some());

        // A’s relation should now point to B_NEW
        let a_node = graph.nodes.get("A").unwrap();
        assert_eq!(a_node.relations.len(), 1);
        assert_eq!(a_node.relations[0].element_node.element.identifier, "B_NEW");
    }

    #[test]
    fn test_get_impact_tree_traverses_correctly() {
        let mut registry = GraphRegistry::new();
        let mut a = make_element("A", "Element A");
        let mut b = make_element("B", "Element B");
        let c = make_element("C", "Element C");

        add_relation(&mut a, "contain", "B");
        add_relation(&mut b, "derive", "C");

        registry.register_element(a.clone(), "file.md").unwrap();
        registry.register_element(b.clone(), "file.md").unwrap();
        registry.register_element(c.clone(), "file.md").unwrap();

        let mut graph = registry;
        graph.build_relation_graph();
        let tree = graph.get_impact_tree("A");

        assert_eq!(tree.element.identifier, "A");
        assert_eq!(tree.relations.len(), 1);

        let b_node = &tree.relations[0].element_node;
        assert_eq!(b_node.element.identifier, "B");
        assert_eq!(b_node.relations.len(), 1);
        assert_eq!(b_node.relations[0].element_node.element.identifier, "C");
    }

    #[test]
    fn test_cycle_is_handled_gracefully() {
        let mut registry = GraphRegistry::new();
        let mut a = make_element("A", "Element A");
        let mut b = make_element("B", "Element B");

        // A -> B and B -> A (cycle)
        add_relation(&mut a, "contain", "B");
        add_relation(&mut b, "derive", "A");

        registry.register_element(a.clone(), "file.md").unwrap();
        registry.register_element(b.clone(), "file.md").unwrap();

        let mut graph = registry;
        graph.build_relation_graph();
        let tree = graph.get_impact_tree("A");

        assert_eq!(tree.element.identifier, "A");
        assert_eq!(tree.relations.len(), 1);
        assert_eq!(tree.relations[0].element_node.element.identifier, "B");

        // Because of cycle protection, B should not recurse into A again
        assert_eq!(tree.relations[0].element_node.relations.len(), 0);
    }

    #[test]
    fn test_move_element_to_existing_location() {
        let mut registry = GraphRegistry::new();

        // Create elements in different locations
        let mut a = make_element("A", "Element A");
        a.file_path = "file1.md".to_string();
        a.section = "Section1".to_string();

        let mut b = make_element("B", "Element B");
        b.file_path = "file2.md".to_string();
        b.section = "Section2".to_string();

        add_relation(&mut a, "contain", "B");

        registry.register_element(a.clone(), "file1.md").unwrap();
        registry.register_element(b.clone(), "file2.md").unwrap();

        let mut graph = registry;

        // Move A to B's location
        let result = graph.move_element_to_location("A", "file2.md", "Section2");
        assert!(result.is_ok());

        // Verify A is now at B's location
        let a_node = graph.nodes.get("A").unwrap();
        assert_eq!(a_node.element.file_path, "file2.md");
        assert_eq!(a_node.element.section, "Section2");
    }

    #[test]
    fn test_move_element_to_nonexistent_location() {
        let mut registry = GraphRegistry::new();
        let a = make_element("A", "Element A");

        registry.register_element(a.clone(), "file.md").unwrap();
        let mut graph = registry;

        // Try to move to non-existent location
        let result = graph.move_element_to_location("A", "nonexistent.md", "NonexistentSection");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist in the graph"));
    }

    #[test]
    fn test_get_available_locations() {
        let mut registry = GraphRegistry::new();

        let mut a = make_element("A", "Element A");
        a.file_path = "file1.md".to_string();
        a.section = "Section1".to_string();

        let mut b = make_element("B", "Element B");
        b.file_path = "file2.md".to_string();
        b.section = "Section2".to_string();

        let mut c = make_element("C", "Element C");
        c.file_path = "file1.md".to_string();
        c.section = "Section1".to_string(); // Same location as A

        registry.register_element(a.clone(), "file1.md").unwrap();
        registry.register_element(b.clone(), "file2.md").unwrap();
        registry.register_element(c.clone(), "file1.md").unwrap();

        let graph = registry;
        let locations = graph.get_available_locations();

        // Should only have 2 unique locations
        assert_eq!(locations.len(), 2);
        assert!(locations.contains(&("file1.md".to_string(), "Section1".to_string())));
        assert!(locations.contains(&("file2.md".to_string(), "Section2".to_string())));
    }

    #[test]
    fn test_get_move_impact() {
        let mut registry = GraphRegistry::new();
        let a = make_element("A", "Element A");
        let mut b = make_element("B", "Element B");
        let mut c = make_element("C", "Element C");

        // B and C both reference A
        add_relation(&mut b, "derive", "A");
        add_relation(&mut c, "contain", "A");

        registry.register_element(a.clone(), "file.md").unwrap();
        registry.register_element(b.clone(), "file.md").unwrap();
        registry.register_element(c.clone(), "file.md").unwrap();

        let graph = registry;
        let impact = graph.get_move_impact("A");

        // Both B and C should be affected by moving A
        assert_eq!(impact.len(), 2);
        assert!(impact.contains(&"B".to_string()));
        assert!(impact.contains(&"C".to_string()));
    }

    #[test]
    fn test_move_element_to_new_section() {
        let mut registry = GraphRegistry::new();
        let mut a = make_element("A", "Element A");
        a.file_path = "file1.md".to_string();
        a.section = "Section1".to_string();

        registry.register_element(a.clone(), "file1.md").unwrap();
        let mut graph = registry;

        // Move A to a new section in the same file
        let result = graph.move_element_to_new_section("A", "file1.md", "NewSection");
        assert!(result.is_ok());

        // Verify A is now in the new section
        let a_node = graph.nodes.get("A").unwrap();
        assert_eq!(a_node.element.file_path, "file1.md");
        assert_eq!(a_node.element.section, "NewSection");
    }

    #[test]
    fn test_move_element_to_new_file() {
        let mut registry = GraphRegistry::new();
        let a = make_element("A", "Element A");

        registry.register_element(a.clone(), "file.md").unwrap();
        let mut graph = registry;

        // Move A to a new file with new section
        let result = graph.move_element_to_new_file("A", "new_file.md", "NewSection");
        assert!(result.is_ok());

        // Verify A is now in the new file and section
        let a_node = graph.nodes.get("A").unwrap();
        assert_eq!(a_node.element.file_path, "new_file.md");
        assert_eq!(a_node.element.section, "NewSection");
    }

    #[test]
    fn test_add_section_to_existing_file() {
        let mut registry = GraphRegistry::new();
        let a = make_element("A", "Element A");

        registry.register_element(a.clone(), "file.md").unwrap();
        let mut graph = registry;

        // Add a new section to existing file
        let result = graph.add_section_to_file("file.md", "NewSection");
        assert!(result.is_ok());

        // Try to add the same section again (should fail)
        let result = graph.add_section_to_file("file.md", "NewSection");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_add_file_location() {
        let mut registry = GraphRegistry::new();
        let mut a = make_element("A", "Element A");
        a.file_path = "existing.md".to_string();

        registry.register_element(a.clone(), "existing.md").unwrap();
        let mut graph = registry;

        // Add a new file location
        let result = graph.add_file_location("new_file.md", "Section1");
        assert!(result.is_ok());

        // Try to add the same file again (should fail)
        let result = graph.add_file_location("existing.md", "Section1");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_move_element_updates_relation_identifiers() {
        let mut registry = GraphRegistry::new();

        // Create elements A, B, C
        let mut a = make_element("A", "Element A");
        a.file_path = "file1.md".to_string();
        a.section = "Section1".to_string();

        let mut b = make_element("B", "Element B");
        b.file_path = "file1.md".to_string();
        b.section = "Section1".to_string();

        let mut c = make_element("C", "Element C");
        c.file_path = "file2.md".to_string();
        c.section = "Section2".to_string();

        // Create relations: B -> A, C -> A
        add_relation(&mut b, "contain", "A");
        add_relation(&mut c, "derive", "A");

        registry.register_element(a.clone(), "file1.md").unwrap();
        registry.register_element(b.clone(), "file1.md").unwrap();
        registry.register_element(c.clone(), "file2.md").unwrap();

        let mut graph = registry;
        graph.build_relation_graph();

        // Verify initial relations exist
        let b_relations = graph.list_relations("B").unwrap();
        let c_relations = graph.list_relations("C").unwrap();
        assert_eq!(b_relations.len(), 1);
        assert_eq!(c_relations.len(), 1);
        assert_eq!(b_relations[0], ("contain".to_string(), "A".to_string()));
        assert_eq!(c_relations[0], ("derive".to_string(), "A".to_string()));

        // Move A to a new file - this should update its identifier
        let result = graph.move_element_to_new_file("A", "file3.md", "Section3");
        assert!(result.is_ok());

        // Check that A's location has changed
        let a_element = graph.get_element("A").unwrap();
        assert_eq!(a_element.file_path, "file3.md");
        assert_eq!(a_element.section, "Section3");

        // CRITICAL: Relations from B and C should still point to A
        // But they should be pointing to the NEW identifier if A's identifier changed
        let b_relations_after = graph.list_relations("B").unwrap();
        let c_relations_after = graph.list_relations("C").unwrap();

        // These should still exist and point to the moved element
        assert_eq!(b_relations_after.len(), 1, "B should still have 1 relation after A is moved");
        assert_eq!(c_relations_after.len(), 1, "C should still have 1 relation after A is moved");

        // The target should still be "A" (or updated identifier if it changed)
        let b_target = &b_relations_after[0].1;
        let c_target = &c_relations_after[0].1;

        // Verify the targets still exist in the graph
        assert!(graph.get_element(b_target).is_some(), "B's relation target '{}' should exist in graph", b_target);
        assert!(graph.get_element(c_target).is_some(), "C's relation target '{}' should exist in graph", c_target);

        println!("B relations after move: {:?}", b_relations_after);
        println!("C relations after move: {:?}", c_relations_after);
    }

    #[test]
    fn test_move_element_updates_identifiers_in_flushed_markdown() {
        let mut registry = GraphRegistry::new();

        // Create elements A, B where B references A
        let mut a = make_element("A", "Element A");
        a.file_path = "file1.md".to_string();
        a.section = "Section1".to_string();

        let mut b = make_element("B", "Element B");
        b.file_path = "file1.md".to_string();
        b.section = "Section1".to_string();

        // B has a relation pointing to A
        add_relation(&mut b, "contain", "A");

        registry.register_element(a.clone(), "file1.md").unwrap();
        registry.register_element(b.clone(), "file1.md").unwrap();

        let mut graph = registry;

        // Move A to a different file
        let result = graph.move_element_to_new_file("A", "file2.md", "Section2");
        assert!(result.is_ok());

        // Now let's check what would be written to markdown when flushed
        // First, let's see what B's relation looks like in the markdown
        let b_node = graph.nodes.get("B").unwrap();

        // Check B's original relations in the element
        println!("B's original relations from element:");
        for relation in &b_node.element.relations {
            println!("  {} -> {}", relation.relation_type.name,
                    match &relation.target.link {
                        crate::relation::LinkType::Identifier(id) => id.clone(),
                        crate::relation::LinkType::InternalPath(path) => path.to_string_lossy().to_string(),
                        crate::relation::LinkType::ExternalUrl(url) => url.clone(),
                    });
        }

        // The issue: B's element still has a relation pointing to "A"
        // But A is now in file2.md, so the relation should be "file2.md#A" if it's a cross-file reference
        // Or if identifiers include file paths, it should be updated accordingly

        // Check if this would cause issues in markdown generation
        // We expect that when we flush, the relations should be correctly written
        // based on the current location of elements

        // The PROBLEM: B's relation still points to "A" but A is now in a different file
        // When B gets written to file1.md, it should reference A as "file2.md#A" not just "A"

        // Let's check what the markdown would look like:
        let b_element = graph.nodes.get("B").unwrap().element.clone();
        let b_markdown = graph.element_to_markdown(&b_element);
        println!("B's markdown after A is moved:");
        println!("{}", b_markdown);

        // The relation should be "file2.md#A" since A is now in a different file
        // but it's probably still "A" which would be incorrect
        assert!(b_markdown.contains("file2.md#A") || b_markdown.contains("[A](file2.md#A)"),
                "B's relation should reference A in its new location: {}", b_markdown);
    }

    #[test]
    fn test_moved_element_relations_update_paths() {
        let mut registry = GraphRegistry::new();

        // Create elements A, B, C where A has relations to both B and C
        let mut a = make_element("A", "Element A");
        a.file_path = "file1.md".to_string();
        a.section = "Section1".to_string();

        let mut b = make_element("B", "Element B");
        b.file_path = "file2.md".to_string();  // B is in different file
        b.section = "Section2".to_string();

        let mut c = make_element("C", "Element C");
        c.file_path = "file1.md".to_string();  // C is in same file as A initially
        c.section = "Section1".to_string();

        // A has relations to both B (cross-file) and C (same-file)
        add_relation(&mut a, "contain", "B");
        add_relation(&mut a, "derive", "C");

        registry.register_element(a.clone(), "file1.md").unwrap();
        registry.register_element(b.clone(), "file2.md").unwrap();
        registry.register_element(c.clone(), "file1.md").unwrap();

        let mut graph = registry;

        // Check A's initial relations in markdown
        let a_element_initial = graph.nodes.get("A").unwrap().element.clone();
        let a_markdown_initial = graph.element_to_markdown(&a_element_initial);
        println!("A's initial markdown (in file1.md):");
        println!("{}", a_markdown_initial);

        // A is in file1.md, B is in file2.md, C is in file1.md
        // So A should reference B as "file2.md#B" and C as just "C" (same file)

        // Move A to file3.md
        let result = graph.move_element_to_new_file("A", "file3.md", "Section3");
        assert!(result.is_ok());

        // Check A's relations after the move
        let a_element_moved = graph.nodes.get("A").unwrap().element.clone();
        let a_markdown_moved = graph.element_to_markdown(&a_element_moved);
        println!("A's markdown after move to file3.md:");
        println!("{}", a_markdown_moved);

        // Now A is in file3.md, so:
        // - A should reference B as "file2.md#B" (cross-file, B is in file2.md)
        // - A should reference C as "file1.md#C" (cross-file, C is in file1.md)
        // Both should be cross-file references now since A moved to file3.md

        println!("A's relations after move:");
        for relation in &a_element_moved.relations {
            println!("  {} -> {}", relation.relation_type.name,
                    match &relation.target.link {
                        crate::relation::LinkType::Identifier(id) => id.clone(),
                        crate::relation::LinkType::InternalPath(path) => path.to_string_lossy().to_string(),
                        crate::relation::LinkType::ExternalUrl(url) => url.clone(),
                    });
        }

        // PROBLEM: A's relations likely still point to "B" and "C"
        // but should now point to "file2.md#B" and "file1.md#C" respectively
        // since A is now in a different file than both of them

        assert!(a_markdown_moved.contains("file2.md#B") || a_markdown_moved.contains("[B](file2.md#B)"),
                "A should reference B with file path since they're in different files: {}", a_markdown_moved);
        assert!(a_markdown_moved.contains("file1.md#C") || a_markdown_moved.contains("[C](file1.md#C)"),
                "A should reference C with file path since they're in different files: {}", a_markdown_moved);
    }

    #[test]
    fn test_flush_creates_proper_markdown_with_cross_file_relations() {
        use std::fs;
        use tempfile::TempDir;

        let mut registry = GraphRegistry::new();

        // Create elements in different files with cross-file relations
        let mut a = make_element("ElementA", "Element A Description");
        a.file_path = "file1.md".to_string();
        a.section = "Section1".to_string();

        let mut b = make_element("ElementB", "Element B Description");
        b.file_path = "file2.md".to_string();
        b.section = "Section2".to_string();

        let mut c = make_element("ElementC", "Element C Description");
        c.file_path = "file1.md".to_string(); // Same file as A
        c.section = "Section1".to_string();

        // Create cross-file relations:
        // A -> B (file1.md -> file2.md)
        // B -> A (file2.md -> file1.md)
        // A -> C (file1.md -> file1.md, same file)
        add_relation(&mut a, "contain", "ElementB");
        add_relation(&mut a, "derive", "ElementC");
        add_relation(&mut b, "refinedBy", "ElementA");

        registry.register_element(a.clone(), "file1.md").unwrap();
        registry.register_element(b.clone(), "file2.md").unwrap();
        registry.register_element(c.clone(), "file1.md").unwrap();

        let mut graph = registry;

        // Move ElementB to file3.md to create more cross-file relations
        let result = graph.move_element_to_new_file("ElementB", "file3.md", "Section3");
        assert!(result.is_ok());

        // Create temp directory for flush output
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path();

        // Flush the graph to markdown files
        let result = graph.flush_to_directory(output_path);
        assert!(result.is_ok());

        // List what files were actually created
        println!("Files created in output directory:");
        for entry in fs::read_dir(output_path).unwrap() {
            let entry = entry.unwrap();
            println!("  {}", entry.file_name().to_string_lossy());
        }

        // Read the generated markdown files and verify their content
        let file1_content = fs::read_to_string(output_path.join("file1.md")).unwrap();
        let file3_content = fs::read_to_string(output_path.join("file3.md")).unwrap();

        // file2.md might not exist if it only contained ElementB which moved to file3.md
        let file2_content = fs::read_to_string(output_path.join("file2.md")).unwrap_or_else(|_| {
            println!("file2.md does not exist (expected if no elements remain in it)");
            String::new()
        });

        println!("=== file1.md content ===");
        println!("{}", file1_content);
        println!("=== file2.md content ===");
        println!("{}", file2_content);
        println!("=== file3.md content ===");
        println!("{}", file3_content);

        // Verify file1.md content (contains ElementA and ElementC)
        assert!(file1_content.contains("### Element A Description"));
        assert!(file1_content.contains("### Element C Description"));
        assert!(!file1_content.contains("### Element B Description")); // ElementB moved to file3.md

        // Verify ElementA's relations in file1.md
        // A -> B should be cross-file reference with proper display name and fragment anchor
        assert!(file1_content.contains("[Element B Description](file3.md#ElementB)"),
                "ElementA should reference ElementB with proper display name: {}", file1_content);

        // A -> C should be same-file reference (no file prefix needed)
        assert!(file1_content.contains("[ElementC](ElementC)") ||
                file1_content.contains("[ElementC](#ElementC)") ||
                file1_content.contains("ElementC"),
                "ElementA should reference ElementC in same file: {}", file1_content);

        // Verify file3.md content (contains ElementB)
        assert!(file3_content.contains("### Element B Description"));
        assert!(!file3_content.contains("### Element A Description"));
        assert!(!file3_content.contains("### Element C Description"));

        // Verify ElementB's relations in file3.md
        // B -> A should be cross-file reference with proper display name and fragment anchor
        assert!(file3_content.contains("[Element A Description](file1.md#ElementA)"),
                "ElementB should reference ElementA with proper display name: {}", file3_content);

        // Verify no virtual placeholder content appears in any file
        assert!(!file1_content.contains("Virtual placeholder"));
        assert!(!file2_content.contains("Virtual placeholder"));
        assert!(!file3_content.contains("Virtual placeholder"));

        // Verify proper markdown structure
        assert!(file1_content.starts_with("# file1\n"));
        assert!(file3_content.starts_with("# file3\n"));
    }

    #[test]
    fn test_move_to_new_section_nonexistent_file() {
        let mut registry = GraphRegistry::new();
        let a = make_element("A", "Element A");

        registry.register_element(a.clone(), "file.md").unwrap();
        let mut graph = registry;

        // Try to move to new section in non-existent file (should fail)
        let result = graph.move_element_to_new_section("A", "nonexistent.md", "NewSection");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist in the graph"));
    }

    #[test]
    fn test_flush_includes_page_content() {
        use std::fs;
        use tempfile::TempDir;

        let mut registry = GraphRegistry::new();

        // Create an element
        let mut a = make_element("ElementA", "Element A Description");
        a.file_path = "test_file.md".to_string();
        a.section = "Section1".to_string();

        registry.register_element(a.clone(), "test_file.md").unwrap();

        // Add page content
        let page = Page::new("This is page frontmatter content.\n\nMore page content here.".to_string());
        registry.pages.insert("test_file.md".to_string(), page);

        let graph = registry;

        // Create temp directory for flush output
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path();

        // Flush the graph to markdown files
        let result = graph.flush_to_directory(output_path);
        assert!(result.is_ok());

        // Read the generated markdown file
        let file_content = fs::read_to_string(output_path.join("test_file.md")).unwrap();

        println!("=== Generated file content ===");
        println!("{}", file_content);

        // Verify file header is present
        assert!(file_content.starts_with("# test file\n\n"));

        // Verify page content is included after header and before sections
        assert!(file_content.contains("This is page frontmatter content."));
        assert!(file_content.contains("More page content here."));

        // Verify element is still present
        assert!(file_content.contains("### Element A Description"));

        // Verify order: header, page content, section, element
        let header_pos = file_content.find("# test file").unwrap();
        let page_content_pos = file_content.find("This is page frontmatter content.").unwrap();
        let section_pos = file_content.find("## Section1").unwrap();
        let element_pos = file_content.find("### Element A Description").unwrap();

        assert!(header_pos < page_content_pos);
        assert!(page_content_pos < section_pos);
        assert!(section_pos < element_pos);
    }

    #[test]
    fn test_flush_includes_section_content() {
        use std::fs;
        use tempfile::TempDir;

        let mut registry = GraphRegistry::new();

        // Create an element
        let mut a = make_element("ElementA", "Element A Description");
        a.file_path = "test_file.md".to_string();
        a.section = "Section1".to_string();

        registry.register_element(a.clone(), "test_file.md").unwrap();

        // Add section content
        let section_key = SectionKey {
            file_path: "test_file.md".to_string(),
            section_name: "Section1".to_string(),
        };
        let section = Section::new("This is section content.\n\nSection description here.".to_string());
        registry.sections.insert(section_key, section);

        let graph = registry;

        // Create temp directory for flush output
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path();

        // Flush the graph to markdown files
        let result = graph.flush_to_directory(output_path);
        assert!(result.is_ok());

        // Read the generated markdown file
        let file_content = fs::read_to_string(output_path.join("test_file.md")).unwrap();

        println!("=== Generated file content ===");
        println!("{}", file_content);

        // Verify section content is included after section header and before elements
        assert!(file_content.contains("This is section content."));
        assert!(file_content.contains("Section description here."));

        // Verify element is still present
        assert!(file_content.contains("### Element A Description"));

        // Verify order: section header, section content, element
        let section_header_pos = file_content.find("## Section1").unwrap();
        let section_content_pos = file_content.find("This is section content.").unwrap();
        let element_pos = file_content.find("### Element A Description").unwrap();

        assert!(section_header_pos < section_content_pos);
        assert!(section_content_pos < element_pos);
    }

    #[test]
    fn test_flush_includes_both_page_and_section_content() {
        use std::fs;
        use tempfile::TempDir;

        let mut registry = GraphRegistry::new();

        // Create elements in multiple sections
        let mut a = make_element("ElementA", "Element A Description");
        a.file_path = "test_file.md".to_string();
        a.section = "Section1".to_string();

        let mut b = make_element("ElementB", "Element B Description");
        b.file_path = "test_file.md".to_string();
        b.section = "Section2".to_string();

        registry.register_element(a.clone(), "test_file.md").unwrap();
        registry.register_element(b.clone(), "test_file.md").unwrap();

        // Add page content
        let page = Page::new("Page frontmatter content.".to_string());
        registry.pages.insert("test_file.md".to_string(), page);

        // Add section content for both sections
        let section1_key = SectionKey {
            file_path: "test_file.md".to_string(),
            section_name: "Section1".to_string(),
        };
        let section1 = Section::new("Section 1 content.".to_string());
        registry.sections.insert(section1_key, section1);

        let section2_key = SectionKey {
            file_path: "test_file.md".to_string(),
            section_name: "Section2".to_string(),
        };
        let section2 = Section::new("Section 2 content.".to_string());
        registry.sections.insert(section2_key, section2);

        let graph = registry;

        // Create temp directory for flush output
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path();

        // Flush the graph to markdown files
        let result = graph.flush_to_directory(output_path);
        assert!(result.is_ok());

        // Read the generated markdown file
        let file_content = fs::read_to_string(output_path.join("test_file.md")).unwrap();

        println!("=== Generated file content ===");
        println!("{}", file_content);

        // Verify all content is present
        assert!(file_content.contains("Page frontmatter content."));
        assert!(file_content.contains("Section 1 content."));
        assert!(file_content.contains("Section 2 content."));
        assert!(file_content.contains("### Element A Description"));
        assert!(file_content.contains("### Element B Description"));

        // Verify proper ordering
        let page_pos = file_content.find("Page frontmatter content.").unwrap();
        let section1_header_pos = file_content.find("## Section1").unwrap();
        let section1_content_pos = file_content.find("Section 1 content.").unwrap();
        let element_a_pos = file_content.find("### Element A Description").unwrap();
        let section2_header_pos = file_content.find("## Section2").unwrap();
        let section2_content_pos = file_content.find("Section 2 content.").unwrap();
        let element_b_pos = file_content.find("### Element B Description").unwrap();

        // Page content should come first
        assert!(page_pos < section1_header_pos);

        // Section 1: header, content, element
        assert!(section1_header_pos < section1_content_pos);
        assert!(section1_content_pos < element_a_pos);

        // Section 2: header, content, element
        assert!(section2_header_pos < section2_content_pos);
        assert!(section2_content_pos < element_b_pos);
    }

    #[test]
    fn test_flush_handles_empty_page_and_section_content() {
        use std::fs;
        use tempfile::TempDir;

        let mut registry = GraphRegistry::new();

        // Create an element
        let mut a = make_element("ElementA", "Element A Description");
        a.file_path = "test_file.md".to_string();
        a.section = "Section1".to_string();

        registry.register_element(a.clone(), "test_file.md").unwrap();

        // Add empty page content (should be skipped)
        let page = Page::new("   \n\t  \n  ".to_string()); // only whitespace
        registry.pages.insert("test_file.md".to_string(), page);

        // Add empty section content (should be skipped)
        let section_key = SectionKey {
            file_path: "test_file.md".to_string(),
            section_name: "Section1".to_string(),
        };
        let section = Section::new("".to_string()); // empty string
        registry.sections.insert(section_key, section);

        let graph = registry;

        // Create temp directory for flush output
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path();

        // Flush the graph to markdown files
        let result = graph.flush_to_directory(output_path);
        assert!(result.is_ok());

        // Read the generated markdown file
        let file_content = fs::read_to_string(output_path.join("test_file.md")).unwrap();

        println!("=== Generated file content ===");
        println!("{}", file_content);

        // Verify that empty/whitespace-only content is not included
        // The file should go directly from header to section header to element
        assert!(file_content.starts_with("# test file\n\n## Section1\n\n### Element A Description"));

        // Verify element is still present
        assert!(file_content.contains("### Element A Description"));
    }

    #[test]
    fn test_flush_detects_duplicate_headers_in_page_content() {
        use std::fs;
        use tempfile::TempDir;

        let mut registry = GraphRegistry::new();

        // Create an element in MOEs.md
        let mut a = make_element("ElementA", "Element A Description");
        a.file_path = "MOEs.md".to_string();
        a.section = "Section1".to_string();

        registry.register_element(a.clone(), "MOEs.md").unwrap();

        // Add page content that contains a header matching the file title
        let page = Page::new("# MOEs\n\nThis is the MOEs page content.".to_string());
        registry.pages.insert("MOEs.md".to_string(), page);

        let graph = registry;

        // Create temp directory for flush output
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path();

        // Flush the graph to markdown files
        let result = graph.flush_to_directory(output_path);
        assert!(result.is_ok());

        // Read the generated markdown file
        let file_content = fs::read_to_string(output_path.join("MOEs.md")).unwrap();

        println!("=== Generated file content with potential duplicate header ===");
        println!("{}", file_content);

        // Count occurrences of "# MOEs" - should be 1, not 2
        let header_count = file_content.matches("# MOEs").count();
        println!("Header count: {}", header_count);

        // Should have only one header now that auto-generation is skipped when page content has header
        assert_eq!(header_count, 1, "Should have only one '# MOEs' header, but found {}", header_count);

        // Verify the content starts with the page header (not auto-generated)
        assert!(file_content.starts_with("# MOEs\n\nThis is the MOEs page content."));
    }

    #[test]
    fn test_flush_generates_header_when_page_content_has_no_header() {
        use std::fs;
        use tempfile::TempDir;

        let mut registry = GraphRegistry::new();

        // Create an element
        let mut a = make_element("ElementA", "Element A Description");
        a.file_path = "test_file.md".to_string();
        a.section = "Section1".to_string();

        registry.register_element(a.clone(), "test_file.md").unwrap();

        // Add page content WITHOUT a header
        let page = Page::new("This is page content without a header.\n\nMore content here.".to_string());
        registry.pages.insert("test_file.md".to_string(), page);

        let graph = registry;

        // Create temp directory for flush output
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path();

        // Flush the graph to markdown files
        let result = graph.flush_to_directory(output_path);
        assert!(result.is_ok());

        // Read the generated markdown file
        let file_content = fs::read_to_string(output_path.join("test_file.md")).unwrap();

        println!("=== Generated file content with auto-generated header ===");
        println!("{}", file_content);

        // Should have auto-generated header since page content doesn't have one
        assert!(file_content.starts_with("# test file\n\n"));

        // Should contain page content after the auto-generated header
        assert!(file_content.contains("This is page content without a header."));

        // Verify order: auto-generated header, page content, section, element
        let header_pos = file_content.find("# test file").unwrap();
        let page_content_pos = file_content.find("This is page content without a header.").unwrap();
        let section_pos = file_content.find("## Section1").unwrap();
        let element_pos = file_content.find("### Element A Description").unwrap();

        assert!(header_pos < page_content_pos);
        assert!(page_content_pos < section_pos);
        assert!(section_pos < element_pos);
    }
}
