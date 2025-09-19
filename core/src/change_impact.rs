use std::collections::{HashSet, BTreeSet};
use serde::Serialize;
use std::path::PathBuf;
use crate::relation::{Relation, RelationTarget, LinkType};
use crate::error::ReqvireError;
use crate::graph_registry::{self, ElementNode, RelationNode};
use crate::relation;
use crate::element;
use difference::{Changeset, Difference};
use serde_json::{json, Value};

/// Represents a simplified relation for reporting.
#[derive(Debug, Clone, Serialize)]
pub struct RelationSummary {
    pub relation_type: String,
    pub target: RelationTarget,
    pub target_changed: bool,
    #[serde(skip_serializing)]
    pub user_created: bool,
}

impl RelationSummary {
    pub fn to_repo_url(&self, base_url: &str, commit: &str) -> Option<String> {
        match &self.target.link {
            LinkType::Identifier(id) => {
                let path = PathBuf::from(id);
                Some(format!("{}/blob/{}/{}", base_url, commit, path.to_string_lossy()))
            }
            LinkType::InternalPath(path) => {
                Some(format!("{}/blob/{}/{}", base_url, commit, path.to_string_lossy()))
            }
            _ => None,
        }
    }
}

/// Report for an element that is newly added (only in the current registry).
#[derive(Debug, Serialize)]
pub struct AddedElement {
    pub element_id: String,
    pub name: String,
    pub new_content: String,
    pub added_relations: Vec<RelationSummary>,
    pub change_impact_tree: ElementNode,
}

/// Report for an element that has been removed (only in the reference registry).
#[derive(Debug, Serialize)]
pub struct RemovedElement {
    pub element_id: String,
    pub name: String,
    pub old_content: String,
    pub removed_relations: Vec<RelationSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InvalidatedVerification {
    pub element_id: String,
    pub name: String,
    pub content: String,
}

/// Report for an element that exists in both registries but has differences.
#[derive(Debug,Serialize)]
pub struct ChangedElement {
    pub element_id: String,
    pub name: String,
    pub old_content: String,
    pub new_content: String,
    pub content_changed: bool,
    pub added_relations: Vec<RelationSummary>,
    pub removed_relations: Vec<RelationSummary>,
    pub change_impact_tree: ElementNode,
}

impl ChangedElement {
    pub fn to_repo_url(&self, base_url: &str, commit: &str) -> String {
        let path = PathBuf::from(&self.element_id);
        format!("{}/blob/{}/{}", base_url, commit, path.to_string_lossy())
    }
}

/// Report detailing changes between two registries.
#[derive(Debug, Serialize)]
pub struct ChangeImpactReport {
    pub added: Vec<AddedElement>,
    pub removed: Vec<RemovedElement>,
    pub changed: Vec<ChangedElement>,
    pub invalidated_verifications: Vec<InvalidatedVerification>,
    #[serde(skip)]
    pub all_added_element_ids: HashSet<String>,
}

impl ChangeImpactReport {
    pub fn new() -> Self {
        Self {
            added: Vec::new(),
            removed: Vec::new(),
            changed: Vec::new(),
            invalidated_verifications: Vec::new(),
            all_added_element_ids: HashSet::new(),
        }
    }
   
    /// Outputs the report as json with GitHub links included.
    pub fn to_json(
        &self,
        base_url: &str,
        git_commit: &str,
        previous_git_commit: &str,
    ) -> serde_json::Value {
   
       
        let added: Vec<_> = self.added.iter().map(|elem| {
            let element_url = format!("{}/blob/{}/{}", base_url, git_commit, elem.element_id);
            let added_relations: Vec<_> = elem.added_relations.iter().map(|rel| {
                let target_url = match rel.target.link {
                    LinkType::Identifier(ref id) => format!("{}/blob/{}/{}", base_url, git_commit, id),
                    LinkType::InternalPath(ref path) => format!("{}/blob/{}/{}", base_url, previous_git_commit, path.display()),
                    _ => rel.target.link.as_str().to_string(),
                };
                json!({
                    "relation_type": rel.relation_type,
                    "target_changed": rel.target_changed,
                    "target_text": rel.target.text,
                    "target_url": target_url
                })
            }).collect();
            let impact_tree = render_change_impact_tree_json(&elem.change_impact_tree, base_url, git_commit);
            json!({
                "element_id": element_url,
                "new_content": elem.new_content,
                "added_relations": added_relations,
                "change_impact_tree": impact_tree
            })
        }).collect();
        let removed: Vec<_> = self.removed.iter().map(|elem| {
            let element_url = format!("{}/blob/{}/{}", base_url, previous_git_commit, elem.element_id);
            let removed_relations: Vec<_> = elem.removed_relations.iter().map(|rel| {
                let target_url = match rel.target.link {
                    LinkType::Identifier(ref id) => format!("{}/blob/{}/{}", base_url, previous_git_commit, id),
                    LinkType::InternalPath(ref path) => format!("{}/blob/{}/{}", base_url, previous_git_commit, path.display()),
                    _ => rel.target.link.as_str().to_string(),
                };
                json!({
                    "relation_type": rel.relation_type,
                    "target_changed": rel.target_changed,
                    "target_text": rel.target.text,
                    "target_url": target_url
                })
            }).collect();
            json!({
                "element_id": element_url,
                "old_content": elem.old_content,
                "removed_relations": removed_relations
            })
        }).collect();
        let changed: Vec<_> = self.changed.iter().map(|elem| {
            let element_url = format!("{}/blob/{}/{}", base_url, git_commit, elem.element_id);
            let added_relations: Vec<_> = elem.added_relations.iter().map(|rel| {
                let target_url = match rel.target.link {
                    LinkType::Identifier(ref id) => format!("{}/blob/{}/{}", base_url, git_commit, id),
                    LinkType::InternalPath(ref path) => format!("{}/blob/{}/{}", base_url, previous_git_commit, path.display()),
                    _ => rel.target.link.as_str().to_string(),
                };
                json!({
                    "relation_type": rel.relation_type,
                    "target_changed": rel.target_changed,
                    "target_text": rel.target.text,
                    "target_url": target_url
                })
            }).collect();
            let removed_relations: Vec<_> = elem.removed_relations.iter().map(|rel| {
                let target_url = match rel.target.link {
                    LinkType::Identifier(ref id) => format!("{}/blob/{}/{}", base_url, previous_git_commit, id),
                    LinkType::InternalPath(ref path) => format!("{}/blob/{}/{}", base_url, previous_git_commit, path.display()),
                    _ => rel.target.link.as_str().to_string(),
                };
                json!({
                    "relation_type": rel.relation_type,
                    "target_changed": rel.target_changed,
                    "target_text": rel.target.text,
                    "target_url": target_url
                })
            }).collect();
   
            let impact_tree = render_change_impact_tree_json(&elem.change_impact_tree, base_url, git_commit);
            json!({
                "element_id": element_url,
                "old_content": elem.old_content,
                "new_content": elem.new_content,
                "content_changed": elem.content_changed,
                "added_relations": added_relations,
                "removed_relations": removed_relations,
                "change_impact_tree": impact_tree
            })
        }).collect();
        let invalidated_verifications: Vec<_> = self.invalidated_verifications.iter().map(|invalidated_ver| {
            let target_url = format!("{}/blob/{}/{}", base_url, git_commit, invalidated_ver.element_id);
            json!({
                "target_text": invalidated_ver.name,
                "target_url": target_url,
                "content": invalidated_ver.content
            })
        }).collect();
        json!({
            "added": added,
            "removed": removed,
            "changed": changed,
            "invalidated_verifications": invalidated_verifications
        })
    }
    /// Outputs the report as text with GitHub links included.
    pub fn to_text(&self, base_url: &str, git_commit: &str, previous_git_commit: &str) -> String {
        let mut output = String::new();
        output.push_str("## Change Impact Report\n\n");
        
        // Use all_added_element_ids which contains IDs from before filtering
        let new_element_ids = &self.all_added_element_ids;
        // Removed Elements section
        if !self.removed.is_empty() {
            output.push_str("### Removed Elements\n\n");
        }
       
        for elem in &self.removed {
            let element_url = format!("{}/blob/{}/{}", base_url, previous_git_commit, elem.element_id);
            output.push_str("* ");
            output.push_str(&format!(
                "[{}]({})\n",
                elem.name, element_url
            ));
        }
        if !self.removed.is_empty() {
            output.push_str("\n---\n\n");
        }
        // Added Elements section
        if !self.added.is_empty() {
            output.push_str("### New Elements\n\n");
        }
       
        for elem in &self.added {
            let element_url = format!("{}/blob/{}/{}", base_url, git_commit, elem.element_id);
            output.push_str("* ");
            output.push_str(&format!(
                "[{}]({})\n",
                elem.name, element_url
            ));
            let rendered_tree = render_change_impact_tree(&elem.change_impact_tree, 2, base_url, git_commit, &new_element_ids);
            if !rendered_tree.trim().is_empty() {
                output.push_str(&rendered_tree);
                output.push_str("\n\n");
            }
        }
        if !self.added.is_empty() {
            output.push_str("\n---\n\n");
        }
       
        // Changed Elements section
        if !self.changed.is_empty() {
            output.push_str("### Changed Elements\n\n");
        }
        for elem in &self.changed {
            let element_url = format!("{}/blob/{}/{}", base_url, git_commit, elem.element_id);
            output.push_str("* ");
            output.push_str(&format!(
                "[{}]({})\n",
                elem.name, element_url
            ));
            let rendered_tree = render_change_impact_tree(&elem.change_impact_tree, 2, base_url, git_commit, &new_element_ids);
            if !rendered_tree.trim().is_empty() {
                //let markdown_diff = generate_markdown_diff(&elem.old_content, &elem.new_content);
                //output.push_str(&format!("{}\n", markdown_diff));
                output.push_str(&rendered_tree);
                output.push_str("\n\n");
            }
        }
        if !self.changed.is_empty() {
            output.push_str("\n---\n\n");
        }
           
       
        // Invalidated Verifications Section
        if !self.invalidated_verifications.is_empty() {
            output.push_str("## Invalidated Verifications\n\n");
            for invalidated_ver in &self.invalidated_verifications {
                let target_url = format!("{}/blob/{}/{}", base_url, git_commit, invalidated_ver.element_id);
                output.push_str(&format!("- [ ] [{}]({})\n", invalidated_ver.name, target_url));
            }
            output.push_str("\n");
        }
       
        if self.removed.is_empty() && self.added.is_empty() && self.changed.is_empty() {
            output.push_str("\nNothing to report...\n");
        }
        output
    }
   
    pub fn print(&self, base_url: &str, git_commit: &str, previous_git_commit: &str, as_json: bool) {
        if as_json {
            println!("{}",serde_json::to_string_pretty(&self.to_json(base_url, git_commit, previous_git_commit)).unwrap());
        } else {
            println!("{}", self.to_text(base_url, git_commit, previous_git_commit));
        }
    }
}

/// Generate a unified diff in a diff-highlighted markdown code fence.
fn _generate_markdown_diff(old: &str, new: &str) -> String {
    let changeset = Changeset::new(old, new, "\n");
    let mut diff_output = String::new();
    for diff in changeset.diffs {
        match diff {
            Difference::Same(ref x) => {
                for line in x.lines() {
                    diff_output.push_str(" ");
                    diff_output.push_str(line);
                    diff_output.push('\n');
                }
            },
            Difference::Rem(ref x) => {
                for line in x.lines() {
                    diff_output.push_str("-");
                    diff_output.push_str(line);
                    diff_output.push('\n');
                }
            },
            Difference::Add(ref x) => {
                for line in x.lines() {
                    diff_output.push_str("+");
                    diff_output.push_str(line);
                    diff_output.push('\n');
                }
            },
        }
    }
    format!("```diff\n{}```", diff_output)
}

/// Render the change impact tree recursively with GitHub links.
fn render_change_impact_tree(
    node: &ElementNode,
    indent: usize,
    base_url: &str,
    git_commit: &str,
    new_element_ids: &HashSet<String>,
) -> String {
    let mut output = String::new();
    let pad = "  ".repeat(indent);
    for relation_node in &node.relations {
        let target = &relation_node.element_node.element;
        let element_url = format!("{}/blob/{}/{}", base_url, git_commit, target.identifier);
        let change_icon = if target.changed_since_commit { " ⚠️" } else { "" };
        let new_icon = if new_element_ids.contains(&target.identifier) { " (new)" } else { "" };
        output.push_str(&format!(
            "{}* {} -> [{}]({}){}{}\n",
            pad,
            relation_node.relation_trigger,
            target.name,
            element_url,
            new_icon,
            change_icon
        ));
        output.push_str(&render_change_impact_tree(
            &relation_node.element_node,
            indent + 1,
            base_url,
            git_commit,
            new_element_ids,
        ));
    }
    output
}

/// Render the change impact tree recursively as structured JSON with GitHub URLs.
fn render_change_impact_tree_json(
    node: &ElementNode,
    base_url: &str,
    git_commit: &str,
) -> Vec<Value> {
    node.relations.iter().map(|relation_node| {
        let child = &relation_node.element_node;
        // Construct the GitHub URL using the base URL and commit hash for the identifier
        let github_url = format!("{}/blob/{}/{}", base_url, git_commit, &child.element.identifier);
        // Render the nested relations in the JSON structure
        let nested_relations = render_change_impact_tree_json(child, base_url, git_commit);
        // Map the relation_trigger as a key and its nested element info as the value
        let mut relation_obj = serde_json::Map::new();
        relation_obj.insert(
            relation_node.relation_trigger.clone(),
            json!({
                "name": child.element.name,
                "identifier": github_url,
                "element_type": format!("{:?}", child.element.element_type),
                "relations": nested_relations
            })
        );
        // Return the relation in the desired format
        json!(relation_obj)
    }).collect()
}

/// Converts a relation into a summarized representation.
fn convert_relation_to_summary(rel: &Relation) -> RelationSummary {
    RelationSummary {
        relation_type: rel.relation_type.name.to_string(),
        target_changed: false,
        target: rel.target.clone(),
        user_created: rel.user_created,
    }
}

/// Builds the change impact tree recursively using `ElementNode` and keeps only forward relations.
pub fn build_change_impact_tree(
    current: &graph_registry::GraphRegistry,
    element_id: String,
    visited: &mut BTreeSet<String>,
    fallback_name: Option<String>,
) -> ElementNode {
    // Fetch the current element or generate a placeholder
    let element = current
        .get_element(&element_id)
        .cloned()
        .unwrap_or_else(|| {
            let display_name = fallback_name.unwrap_or_else(|| "Missing Element".to_string());
            let mut placeholder = element::Element::new(
                &display_name,
                &element_id,
                "unknown",
                "Placeholder",
                None,
            );
            placeholder.content = "Element referenced but not found in registry".to_string();
            placeholder
        });
    // Recursively collect forward-impacting relation nodes
    let relations = current
        .change_impact_with_relation(&element)
        .into_iter()
        .filter_map(|(impacted_id, rels)| {
            // Skip cycles
            if !visited.insert(impacted_id.clone()) {
                return None;
            }

            // Use the text from the first relation as a fallback display name
            let fallback_name = rels.first().map(|rel| rel.target.text.clone());
            let child_node = build_change_impact_tree(
                current,
                impacted_id.clone(),
                visited,
                fallback_name,
            );
            // Only include relations that propagate changes
            let forward_relations: Vec<_> = rels
                .into_iter()
                .filter(|rel| relation::IMPACT_PROPAGATION_RELATIONS.contains(&rel.relation_type.name))
                .map(|rel| RelationNode {
                    relation_trigger: rel.relation_type.name.to_string(),
                    element_node: child_node.clone(),
                })
                .collect();
            if forward_relations.is_empty() {
                None
            } else {
                Some(forward_relations)
            }
        })
        .flatten()
        .collect();
    ElementNode {
        element,
        relations,
    }
}

fn collect_verification_elements_from_impact_tree(
    node: &ElementNode,
) -> Vec<InvalidatedVerification> {
    let mut collected = Vec::new();
    let mut seen = HashSet::new();
    fn walk(
        node: &ElementNode,
        seen: &mut HashSet<String>,
        collected: &mut Vec<InvalidatedVerification>,
    ) {
        if let element::ElementType::Verification(_) = node.element.element_type {
            let id = node.element.identifier.clone();
            if seen.insert(id.clone()) {
                collected.push(InvalidatedVerification {
                    element_id: id,
                    name: node.element.name.clone(),
                    content: node.element.content.clone(),
                });
            }
        }
        for rel in &node.relations {
            walk(&rel.element_node, seen, collected);
        }
    }
    walk(node, &mut seen, &mut collected);
    collected
}

fn propagate_changed_flags(
    node: &mut ElementNode,
    changed_ids: &HashSet<String>,
) {
    for relation in &mut node.relations {
        let child = &mut relation.element_node;
        if changed_ids.contains(&child.element.identifier) {
            child.element.changed_since_commit = true;
        }
        // Recurse further down the tree
        propagate_changed_flags(child, changed_ids);
    }
}

pub fn apply_smart_filtering(
    report: &mut ChangeImpactReport,
    _current: &graph_registry::GraphRegistry,
) {
    // Step 1: Collect ALL added element IDs BEFORE filtering

    // Step 2: Collect IDs of elements referenced in trees (i.e. children, not roots)
    let mut referenced_ids = HashSet::new();
    for added in &report.added {
        for rel_node in &added.change_impact_tree.relations {
            collect_tree_ids_recursively(&rel_node.element_node, &mut referenced_ids);
        }
    }

    for changed in &report.changed {
        for rel_node in &changed.change_impact_tree.relations {
            collect_tree_ids_recursively(&rel_node.element_node, &mut referenced_ids);
        }
    }

    // Step 3: Store information for text output marking (removed actual marking to keep JSON clean)

    // Step 4: Filter out added/changed elements that are not root
    report.added.retain(|e| !referenced_ids.contains(&e.element_id));
    report.changed.retain(|e| !referenced_ids.contains(&e.element_id));
}

fn collect_tree_ids_recursively(
    node: &ElementNode,
    set: &mut HashSet<String>,
) {
    // Insert this node's identifier
    set.insert(node.element.identifier.clone());

    // Recursively process all children
    for relation in &node.relations {
        collect_tree_ids_recursively(&relation.element_node, set);
    }
}




pub fn compute_change_impact(
    current: &graph_registry::GraphRegistry,
    reference: &graph_registry::GraphRegistry,
) -> Result<ChangeImpactReport, ReqvireError> {
    let mut report = ChangeImpactReport::new();
    let current_ids: HashSet<String> = current.get_all_elements().iter().map(|e| e.identifier.clone()).collect();
    let reference_ids: HashSet<String> = reference.get_all_elements().iter().map(|e| e.identifier.clone()).collect();
   
    // Process elements present in both registries.
    for id in current_ids.intersection(&reference_ids) {
        let cur_elem = current.get_element(id).unwrap();
        let ref_elem = reference.get_element(id).unwrap();
        let content_changed = cur_elem.hash_impact_content != ref_elem.hash_impact_content;
       
        // Only track changes to relations that propagate impact according to specifications
        let cur_relations: HashSet<_> = cur_elem.relations.iter()
            .filter(|r| relation::IMPACT_PROPAGATION_RELATIONS.contains(&r.relation_type.name))
            .cloned().collect();
        let ref_relations: HashSet<_> = ref_elem.relations.iter()
            .filter(|r| relation::IMPACT_PROPAGATION_RELATIONS.contains(&r.relation_type.name))
            .cloned().collect();
        let added_relations: Vec<_> = cur_relations
            .difference(&ref_relations)
            .cloned()
            .map(|rel: Relation| convert_relation_to_summary(&rel))
            .collect();
        let removed_relations: Vec<_> = ref_relations
            .difference(&cur_relations)
            .cloned()
            .map(|rel: Relation| convert_relation_to_summary(&rel))
            .collect();
        let has_changed = content_changed || !added_relations.is_empty() || !removed_relations.is_empty();
        if has_changed {
            let mut visited = BTreeSet::new();
            visited.insert(id.clone());
            let change_impact_tree = build_change_impact_tree(current, id.to_string(), &mut visited,None);

            report.changed.push(ChangedElement {
                element_id: id.clone(),
                name: cur_elem.name.clone(),
                old_content: ref_elem.content.clone(),
                new_content: cur_elem.content.clone(),
                content_changed,
                added_relations,
                removed_relations,
                change_impact_tree,
            });
        }
    }
    // Process added elements (present only in current registry).
    for id in current_ids.difference(&reference_ids) {
        let cur_elem = current.get_element(id).unwrap();
        let added_relations: Vec<_> = cur_elem
            .relations
            .iter()
            .filter(|r| relation::IMPACT_PROPAGATION_RELATIONS.contains(&r.relation_type.name))
            .cloned()
            .map(|rel: Relation| convert_relation_to_summary(&rel))
            .collect();
        let mut visited = BTreeSet::new();
        visited.insert(id.clone());
        let change_impact_tree = build_change_impact_tree(current, id.to_string(), &mut visited, None);
        report.added.push(AddedElement {
            element_id: id.clone(),
            name: cur_elem.name.clone(),
            new_content: cur_elem.content.clone(),
            added_relations,
            change_impact_tree,
        });
    }
    // Process removed elements (present only in reference registry).
    for id in reference_ids.difference(&current_ids) {
        let ref_elem = reference.get_element(id).unwrap();
        let removed_relations: Vec<_> = ref_elem
            .relations
            .iter()
            .cloned()
            .map(|rel: Relation| convert_relation_to_summary(&rel))
            .collect();
        report.removed.push(RemovedElement {
            element_id: id.clone(),
            name: ref_elem.name.clone(),
            old_content: ref_elem.content.clone(),
            removed_relations,
        });
    }
    // Collect all changed element IDs to propagate change flags in impact trees
    let changed_element_ids: HashSet<String> = report.changed.iter()
        .map(|elem| elem.element_id.clone())
        .collect();
    // Use changed_element_ids (all changed elements) instead of content_changed_ids
    // to ensure change impact trees show ⚠️ for all changed elements, not just content changes
    for changed in &mut report.changed {
        propagate_changed_flags(&mut changed.change_impact_tree, &changed_element_ids);
    }
    for added in &mut report.added {
        propagate_changed_flags(&mut added.change_impact_tree, &changed_element_ids);
    }
     
               
    // Gather a list of all invalidated verifications from changed and added change impact trees
    let from_added = report
        .added.iter()
        .flat_map(|elem| collect_verification_elements_from_impact_tree(&elem.change_impact_tree));
    let from_changed = report
        .changed.iter()
        .flat_map(|elem| collect_verification_elements_from_impact_tree(&elem.change_impact_tree));
    let mut inv_ver: Vec<_>=from_added.chain(from_changed).collect();
    inv_ver.sort_by_key(|v| v.element_id.clone());
    inv_ver.dedup_by_key(|v| v.element_id.clone());
    report.invalidated_verifications =inv_ver;
    
    // Store all added element IDs before smart filtering is applied
    report.all_added_element_ids = report.added.iter()
        .map(|elem| elem.element_id.clone())
        .collect();
     
    // Apply smart filtering to eliminate redundant new elements
    apply_smart_filtering(&mut report, current);
  
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::Element;
    use crate::relation::{RelationTypeInfo, Relation, RelationTarget, ArrowDirection};
    use crate::GraphRegistry;
   
    /// Helper function to create a simple element.
    fn create_element(identifier: &str, name: &str, content: &str) -> Element {
        let mut element = Element::new(
            name,
            identifier,
            "test.md",
            "TestSection",
            Some(crate::element::ElementType::Requirement(crate::element::RequirementType::System))
        );
        element.add_content(content);
        element
    }
       
    /// Helper function to add a relation to an element.
    fn add_relation(element: &mut Element, relation_type: &'static RelationTypeInfo, target_id: &str) {
        element.relations.push(Relation {
            relation_type,
            target: RelationTarget {
                text: target_id.to_string(),
                link: relation::LinkType::Identifier(target_id.to_string()),
            },
            user_created: true,
        });
    }
       
    #[test]
    fn test_build_change_impact_tree() {
        let mut my_struct = GraphRegistry::new();
        let element_a = create_element("A", "Element A", "Content A");
        let mut element_b = create_element("B", "Element B", "Content B");
        // Define a forward relation from B to A.
        add_relation(
            &mut element_b,
            &RelationTypeInfo {
                name: "derive",
                opposite: Some("derivedFrom"),
                description: "Element B derives from A",
                arrow: "-->",
                label: "label",
                arrow_direction: ArrowDirection::ElementToTarget,
            },
            "A",
        );
        my_struct.register_element(element_a.clone(), "file.md").unwrap();
        my_struct.register_element(element_b.clone(), "file.md").unwrap();
        let mut visited = BTreeSet::new();
        visited.insert("B".to_string());
        let tree = build_change_impact_tree(
            &my_struct,
            "B".to_string(),
            &mut visited,
            None
        );
        assert_eq!(tree.element.identifier, "B");
        assert_eq!(tree.relations.len(), 1);
        // Access the child node via element_node.
        assert_eq!(
            tree.relations[0].element_node.element.identifier,
            "A"
        );
        assert_eq!(tree.relations[0].relation_trigger, "derive");
    }
    #[test]
    fn test_tree_with_cycle() {
        let mut my_struct = GraphRegistry::new();
        let mut element_a = create_element("A", "Element A", "Content A");
        let mut element_b = create_element("B", "Element B", "Content B");
        // Create a cycle: A -> B -> A.
        add_relation(
            &mut element_a,
            &RelationTypeInfo {
                name: "contain",
                opposite: Some("containedBy"),
                description: "Element A contains B",
                arrow: "-->",
                label: "label",
                arrow_direction: ArrowDirection::ElementToTarget,
            },
            "B",
        );
        add_relation(
            &mut element_b,
            &RelationTypeInfo {
                name: "derive",
                opposite: Some("derivedFrom"),
                description: "Element B derives from A",
                arrow: "-->",
                label: "label",
                arrow_direction: ArrowDirection::ElementToTarget,
            },
            "A",
        );
        my_struct.register_element(element_a.clone(), "file.md").unwrap();
        my_struct.register_element(element_b.clone(), "file.md").unwrap();
        let mut visited = BTreeSet::new();
        visited.insert("A".to_string());
        let tree = build_change_impact_tree(
            &my_struct,
            "A".to_string(),
            &mut visited,
            None
        );
        // Check that the cycle is correctly handled and not infinite.
        assert_eq!(tree.element.identifier, "A");
        assert_eq!(tree.relations.len(), 1);
        // For the relation from A to B.
        assert_eq!(
            tree.relations[0].element_node.element.identifier,
            "B"
        );
        assert_eq!(tree.relations[0].relation_trigger, "contain");
       
        // The child node for B has no relations in its tree because B -> A is not processed by the tree
        // since we only consider Forward relations in change_impact_with_relation
        assert_eq!(tree.relations[0].element_node.relations.len(), 0);
       
        // However, the original element B still has its relation to A
        assert_eq!(
            tree.relations[0].element_node.element.relations.len(),
            1
        );
        assert_eq!(
            tree.relations[0].element_node.element.relations[0].target.text,
            "A"
        );
    }
    #[test]
    fn test_smart_filtering_with_parent_child_requirements() {
        // Create current registry with parent and child requirements both added
        let mut current_registry = GraphRegistry::new();
       
        // Create parent requirement with forward relations
        let mut parent_req = create_element("req1.md#parent-requirement", "Parent Requirement", "Parent content");
        parent_req.relations.push(Relation {
            relation_type: &RelationTypeInfo {
                name: "derive",
                opposite: Some("derivedFrom"),
                description: "Parent derives child",
                arrow: "-->",
                label: "derive",
                arrow_direction: ArrowDirection::ElementToTarget,
            },
            target: RelationTarget {
                text: "Child Requirement".to_string(),
                link: LinkType::Identifier("req1.md#child-requirement".to_string()),
            },
            user_created: true,
        });
        parent_req.relations.push(Relation {
            relation_type: &RelationTypeInfo {
                name: "verifiedBy",
                opposite: Some("verify"),
                description: "Verified by test",
                arrow: "-->",
                label: "verifiedBy",
                arrow_direction: ArrowDirection::ElementToTarget,
            },
            target: RelationTarget {
                text: "Parent Verification".to_string(),
                link: LinkType::Identifier("verify.md#parent-verification".to_string()),
            },
            user_created: true,
        });
       
        // Create child requirement with backward relation
        let mut child_req = create_element("req1.md#child-requirement", "Child Requirement", "Child content");
        child_req.relations.push(Relation {
            relation_type: &RelationTypeInfo {
                name: "derivedFrom",
                opposite: Some("derive"),
                description: "Child derived from parent",
                arrow: "<--",
                label: "derivedFrom",
                arrow_direction: ArrowDirection::TargetToElement,
            },
            target: RelationTarget {
                text: "Parent Requirement".to_string(),
                link: LinkType::Identifier("req1.md#parent-requirement".to_string()),
            },
            user_created: false,  // Auto-generated opposite relations
        });
       
        // Create a verification with backward relation
        let mut verification = Element::new(
            "Parent Verification",
            "verify.md#parent-verification",
            "verify.md",
            "Verifications",
            Some(crate::element::ElementType::Verification(crate::element::VerificationType::Test))
        );
        verification.add_content("Verification content");
        verification.relations.push(Relation {
            relation_type: &RelationTypeInfo {
                name: "verify",
                opposite: Some("verifiedBy"),
                description: "Verifies requirement",
                arrow: "<--",
                label: "verify",
                arrow_direction: ArrowDirection::TargetToElement,
            },
            target: RelationTarget {
                text: "Parent Requirement".to_string(),
                link: LinkType::Identifier("req1.md#parent-requirement".to_string()),
            },
            user_created: false,  // Auto-generated opposite relations
        });
       
        current_registry.register_element(parent_req, "req1.md").unwrap();
        current_registry.register_element(child_req, "req1.md").unwrap();
        current_registry.register_element(verification, "verify.md").unwrap();
       
        // Create empty reference registry (all elements are new)
        let reference_registry = GraphRegistry::new();
       
        // Compute change impact
        let report = compute_change_impact(&current_registry, &reference_registry).unwrap();
       
        // Verify results according to smart filtering requirement:
        // Child requirement is filtered out because it's referenced in parent's relations
        // Verification is also filtered out because it's referenced in parent's relations
        // Only parent requirement should remain
        assert_eq!(report.added.len(), 1, "Should have 1 added element after filtering");
       
        let added_ids: Vec<&str> = report.added.iter().map(|e| e.element_id.as_str()).collect();
        assert!(added_ids.contains(&"req1.md#parent-requirement"), "Parent requirement should be in added elements");
        assert!(!added_ids.contains(&"verify.md#parent-verification"), "Verification should be filtered out");
        assert!(!added_ids.contains(&"req1.md#child-requirement"), "Child requirement should be filtered out");
    }
    #[test]
    fn test_smart_filtering_with_requirement_and_verification() {
        // Create current registry with a requirement and its verification both added
        let mut current_registry = GraphRegistry::new();
       
        // Create requirement with verifiedBy relation
        let mut requirement = create_element("req.md#new-requirement", "New Requirement", "Requirement content");
        requirement.relations.push(Relation {
            relation_type: &RelationTypeInfo {
                name: "verifiedBy",
                opposite: Some("verify"),
                description: "Verified by test",
                arrow: "-->",
                label: "verifiedBy",
                arrow_direction: ArrowDirection::ElementToTarget,
            },
            target: RelationTarget {
                text: "New Verification".to_string(),
                link: LinkType::Identifier("verify.md#new-verification".to_string()),
            },
            user_created: true,
        });
       
        // Create verification with verify relation to requirement
        let mut verification = Element::new(
            "New Verification",
            "verify.md#new-verification",
            "verify.md",
            "Verifications",
            Some(crate::element::ElementType::Verification(crate::element::VerificationType::Test))
        );
        verification.add_content("Verification content");
        verification.relations.push(Relation {
            relation_type: &RelationTypeInfo {
                name: "verify",
                opposite: Some("verifiedBy"),
                description: "Verifies requirement",
                arrow: "<--",
                label: "verify",
                arrow_direction: ArrowDirection::TargetToElement,
            },
            target: RelationTarget {
                text: "New Requirement".to_string(),
                link: LinkType::Identifier("req.md#new-requirement".to_string()),
            },
            user_created: false,  // Auto-generated opposite relations
        });
       
        current_registry.register_element(requirement, "req.md").unwrap();
        current_registry.register_element(verification, "verify.md").unwrap();
       
        // Create empty reference registry (all elements are new)
        let reference_registry = GraphRegistry::new();
       

        // Compute change impact
        let report = compute_change_impact(&current_registry, &reference_registry).unwrap();
       

        // According to smart filtering requirement:
        // If both are new, and requirement has verifiedBy pointing to verification,
        // then verification should be filtered out
        assert_eq!(report.added.len(), 1, "Should have 1 added element after filtering");
       
        let added_ids: Vec<&str> = report.added.iter().map(|e| e.element_id.as_str()).collect();
        assert!(added_ids.contains(&"req.md#new-requirement"), "Requirement should be in added elements");
        assert!(!added_ids.contains(&"verify.md#new-verification"), "Verification should be filtered out");
    }
}
