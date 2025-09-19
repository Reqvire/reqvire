use crate::element;
use crate::graph_registry::GraphRegistry;
use serde::Serialize;
use std::collections::HashMap;
use crate::error::ReqvireError;
use crate::relation;
use globset::{Glob, GlobMatcher};
use regex::Regex;



pub struct Filters {
    file_glob:    Option<GlobMatcher>,
    name_re:      Option<Regex>,
    section_glob: Option<GlobMatcher>,
    type_pat:     Option<String>,
    content_re:   Option<Regex>,
    not_verified: bool,
    not_satisfied: bool,
}

impl Filters {
    /// Builds a Filters struct, or returns a ReqvireError::InvalidGlob / InvalidRegex
    pub fn new(
        file: Option<&str>,
        name_regex: Option<&str>,
        section: Option<&str>,
        typ: Option<&str>,
        content: Option<&str>,
        is_not_verified: bool,
        is_not_satisfied: bool,
    ) -> Result<Self, ReqvireError> {
        fn compile_glob(pat: &str) -> Result<GlobMatcher, ReqvireError> {
            let glob =Glob::new(pat)
                .map_err(|e| ReqvireError::InvalidGlob(e.to_string()))?
                .compile_matcher();
            Ok(glob)

        }

        let file_glob = file.map(|p| compile_glob(p)).transpose()?;
        let name_re = match name_regex {
            Some(r) => Some(Regex::new(r).map_err(|e| ReqvireError::InvalidRegex(e.to_string()))?),
            None => None,
        };
        let section_glob = section.map(|p| compile_glob(p)).transpose()?;
        let type_pat = typ.map(|s| s.to_lowercase());
        let content_re = match content {
            Some(r) => Some(Regex::new(r).map_err(|e| ReqvireError::InvalidRegex(e.to_string()))?),
            None => None,
        };

        Ok(Filters {
            file_glob,
            name_re,
            section_glob,
            type_pat,
            content_re,
            not_verified: is_not_verified,
            not_satisfied: is_not_satisfied,
        })
    }

    /// Returns true if this element passes *all* of the user’s filters.
    pub fn matches(&self, e: &element::Element) -> bool {
        // 1) file glob
        if let Some(g) = &self.file_glob {
            if !g.is_match(&e.file_path) {
                return false;
            }
        }
        // 2) name regex
        if let Some(re) = &self.name_re {
            if !re.is_match(&e.name) {
                return false;
            }
        }
        // 3) section glob
        if let Some(g) = &self.section_glob {
            if !g.is_match(&e.section) {
                return false;
            }
        }
        // 4) type filter
        if let Some(tp) = &self.type_pat {
            let filter_type = element::ElementType::from_metadata(tp);
            if &e.element_type != &filter_type {
                return false;
            }
        }
        // 5) content regex
        if let Some(re) = &self.content_re {
            let text = e.content.clone();
            if !re.is_match(&text) {
                return false;
            }
        }

        // Pre-compute verify/satisfy counts for later filters
        let verified_count = e.relations.iter()
            .filter(|r| relation::is_verification_relation(r.relation_type))
            .count();

        let satisfied_count = e.relations.iter()
            .filter(|r| relation::is_satisfaction_relation(r.relation_type))
            .count();
            
            
        // 6) not_verified: exclude any element that *has* a verified relation
        if self.not_verified && verified_count > 0 {
            return false;
        }
        // 7) not_satisfied: exclude any element that *has* a satisfied relation
        if self.not_satisfied && satisfied_count > 0 {
            return false;
        }

        // passed all filters
        true
    }
}

#[derive(Serialize)]
struct Summary {
    files: HashMap<String, FileSummary>,
    global_counters: GlobalCounters,
}

#[derive(Serialize)]
struct FileSummary {
    sections: HashMap<String, SectionSummary>,
    page_content: Option<String>,
    total_sections: usize,
    total_elements: usize,
}

#[derive(Serialize)]
struct SectionSummary {
    elements: Vec<ElementSummary>,
    section_content: Option<String>,
    element_count: usize,
}

#[derive(Serialize)]
struct ElementSummary {
    identifier: String,
    name: String,
    section: String,
    file: String,
    #[serde(rename = "type")]
    element_type: String,
    content: String,
    verified_relations_count: usize,
    satisfied_relations_count: usize,
    relations: Vec<RelationSummary>,
}

#[derive(Serialize)]
struct RelationSummary {
    relation_type: String,
    target: TargetSummary,
}

#[derive(Serialize)]
struct TargetSummary {
    target: String,
    #[serde(rename = "type")]
    link_type: String,
}

#[derive(Serialize, Default)]
struct GlobalCounters {
    total_elements: usize,
    total_files: usize,
    total_sections: usize,

    // Requirements by type
    total_requirements_system: usize,
    total_requirements_user: usize,

    // Verifications by type
    total_verifications_test: usize,
    total_verifications_analysis: usize,
    total_verifications_inspection: usize,
    total_verifications_demonstration: usize,

    // All requirements missing relations
    requirements_not_verified: usize,
    requirements_not_satisfied: usize,
}

pub enum SummaryOutputFormat {
    Text,
    Json,
    Cypher,
}

pub fn print_registry_summary(
    registry: &GraphRegistry,
    output_format: SummaryOutputFormat,
    filters: &Filters,
) {
    let summary = build_summary(registry, filters);

    match output_format {
        SummaryOutputFormat::Text => print_summary_text(&summary),
        SummaryOutputFormat::Json => {
            let text = serde_json::to_string_pretty(&summary)
                .expect("failed to serialize summary");
            println!("{}", text);
        }
        SummaryOutputFormat::Cypher => {
            let cypher_output = print_summary_cypher(&summary);
            println!("{}", cypher_output);
        }
    }
}



fn build_summary(
    registry: &GraphRegistry,
    filters: &Filters,
) -> Summary {
    let mut files: HashMap<String, FileSummary> = HashMap::new();
    let mut counters = GlobalCounters::default();

    for elem in registry.get_all_elements() {
        // Apply filters
        if !filters.matches(elem) {
            continue;
        }

        counters.total_elements += 1;

        // Count by element type, and accumulate missing‐relations for all requirements
        let (vc, sc) = match &elem.element_type {
            element::ElementType::Requirement(req_t) => {
                match req_t {
                    element::RequirementType::System => counters.total_requirements_system += 1,
                    element::RequirementType::User   => counters.total_requirements_user += 1,
                }

                    
                let (mut vcount, mut scount) = (0, 0);
                for r in &elem.relations {
                    if relation::is_verification_relation(r.relation_type) {
                        vcount += 1;
                    } else if relation::is_satisfaction_relation(r.relation_type) {
                        scount += 1;
                    }
                }
                        
                if vcount == 0 {
                    counters.requirements_not_verified += 1;
                }
                if scount == 0 {
                    counters.requirements_not_satisfied += 1;
                }
                (vcount, scount)
            }
            element::ElementType::Verification(ver_t) => {
                match ver_t {
                    element::VerificationType::Default       => counters.total_verifications_test += 1,                
                    element::VerificationType::Test          => counters.total_verifications_test += 1,
                    element::VerificationType::Analysis      => counters.total_verifications_analysis += 1,
                    element::VerificationType::Inspection    => counters.total_verifications_inspection += 1,
                    element::VerificationType::Demonstration => counters.total_verifications_demonstration += 1,
                }
                (0, 0)
            }
            _ => (0, 0),
        };

        let rels: Vec<RelationSummary> = elem.relations.iter()
            .map(|relation| {
                let (tgt, lt) = match &relation.target.link {
                    relation::LinkType::Identifier(id)   => (id.clone(), "identifier".to_string()),
                    relation::LinkType::ExternalUrl(url) => (url.clone(), "external-url".to_string()),
                    relation::LinkType::InternalPath(path) => (path.to_string_lossy().to_string(), "internal-path".to_string()),
                };
                RelationSummary {
                    relation_type: relation.relation_type.name.to_string(),
                    target: TargetSummary { target: tgt, link_type: lt },
                }
            })
            .collect();

        // Assemble the element summary
        let es = ElementSummary {
            identifier: elem.identifier.clone(),
            name: elem.name.clone(),
            section: elem.section.clone(),
            file: elem.file_path.clone(),
            element_type: elem.element_type.as_str().to_string(),
            content: elem.content.clone(),
            verified_relations_count: vc,
            satisfied_relations_count: sc,
            relations: rels,
        };

        // Insert into nested file→section map
        files.entry(elem.file_path.clone())
            .or_insert_with(|| FileSummary {
                sections: HashMap::new(),
                page_content: None,
                total_sections: 0,
                total_elements: 0,
            })
            .sections.entry(elem.section.clone())
            .or_insert_with(|| SectionSummary {
                elements: Vec::new(),
                section_content: Some(String::new()),
                element_count: 0,
            })
            .elements.push(es);
    }

    // Add page content and calculate counts
    for (file_path, file_summary) in &mut files {
        // Get page content
        if let Some(page) = registry.pages.get(file_path) {
            file_summary.page_content = Some(page.frontmatter_content.clone());
        }

        // Get section content and calculate counts
        for (section_name, section_summary) in &mut file_summary.sections {
            // Get section content
            let section_key = crate::graph_registry::SectionKey::new(
                file_path.clone(),
                section_name.clone()
            );
            if let Some(section) = registry.sections.get(&section_key) {
                section_summary.section_content = Some(section.content.clone());
            }

            // Calculate element count for this section
            section_summary.element_count = section_summary.elements.len();
        }

        // Calculate file-level counts
        file_summary.total_sections = file_summary.sections.len();
        file_summary.total_elements = file_summary.sections.values()
            .map(|s| s.elements.len())
            .sum();
    }

    // Calculate global counts
    counters.total_files = files.len();
    counters.total_sections = registry.sections.len();

    Summary {
        files,
        global_counters: counters,
    }
}

fn print_summary_cypher(summary: &Summary) -> String {
    let mut cypher_statements = Vec::new();

    // Create nodes
    for (file, fsum) in &summary.files {
        for (section, ssum) in &fsum.sections {
            for e in &ssum.elements {
                let props = vec![
                    format!("id: \"{}\"", escape(&e.identifier)),
                    format!("name: \"{}\"", escape(&e.name)),
                    format!("file: \"{}\"", escape(file)),
                    format!("section: \"{}\"", escape(section)),
                    format!("element_type: \"{}\"", e.element_type),
                    format!("content: \"{}\"", escape(&e.content)),
                ];

                let node = format!("CREATE (:Element:{} {{ {} }});",
                    e.element_type.replace("-", "_"), // label
                    props.join(", ")
                );

                cypher_statements.push(node);
            }
        }
    }

    // Create relationships
    for fsum in summary.files.values() {
        for ssum in fsum.sections.values() {
            for e in &ssum.elements {
                for rel in &e.relations {
                    let rel_type = rel.relation_type.to_uppercase();
                    let rel_label = format!(
                        "[:{} {{type: \"{}\"}}]",
                        rel_type, rel.target.link_type
                    );

                    let relationship = format!(
                        "MATCH (a:Element {{id: \"{}\"}}), (b:Element {{id: \"{}\"}}) CREATE (a)-{}->(b);",
                        escape(&e.identifier),
                        escape(&rel.target.target),
                        rel_label
                    );

                    cypher_statements.push(relationship);
                }
            }
        }
    }

    cypher_statements.join("\n")
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\")      // Escape backslashes
     .replace('"', "\\\"")       // Escape double quotes
     .replace('\n', "\\n")       // Escape newlines
     .replace('\r', "")          // Remove carriage returns
     .replace("```", "~~~")      // Replace triple-backtick code blocks
}

fn print_summary_text(summary: &Summary) {
    println!("--- MBSE Model summary ---");
    for (file, fsum) in &summary.files {
        println!("📂 File: {} (sections: {}, elements: {})",
                 file, fsum.total_sections, fsum.total_elements);

        // Show page content if available
        if let Some(page_content) = &fsum.page_content {
            if !page_content.trim().is_empty() {
                println!("  📄 Page content: {:?}", page_content);
                println!();
            }
        }

        for (sec, ssum) in &fsum.sections {
            println!("  📖 Section: {} (elements: {})", sec, ssum.element_count);

            // Show section content if available
            if let Some(section_content) = &ssum.section_content {
                if !section_content.trim().is_empty() {
                    println!("    📝 Section content: {:?}", section_content);
                    println!();
                }
            }

            for e in &ssum.elements {
                println!("    🔹 Element: {}", e.identifier);
                println!("      - Name: {}", e.name);
                println!("      - Section: {}", e.section);
                println!("      - File: {}", e.file);
                println!("      - Type: {}", e.element_type);
                println!("      - Content: {:?}", e.content);
                println!("      - Verified relations count: {}", e.verified_relations_count);
                println!("      - Satisfied relations count: {}", e.satisfied_relations_count);
                if e.relations.is_empty() {
                    println!("      - No relations.");
                } else {
                    println!("      - Relations:");
                    for r in &e.relations {
                        println!("        ↪ {}: {} ({})",
                            r.relation_type, r.target.target, r.target.link_type);
                    }
                }
                println!();
            }
        }
    }

    let c = &summary.global_counters;
    println!("------------------------------------");
    println!("📊 Summary Counts:");
    println!("Total files: {}", c.total_files);
    println!("Total sections: {}", c.total_sections);
    println!("Total elements: {}", c.total_elements);
    println!();
    println!("📋 Element Types:");
    println!("System Requirements: {}", c.total_requirements_system);
    println!("User Requirements: {}", c.total_requirements_user);
    println!("Verifications (Test): {}", c.total_verifications_test);
    println!("Verifications (Analysis): {}", c.total_verifications_analysis);
    println!("Verifications (Inspection): {}", c.total_verifications_inspection);
    println!("Verifications (Demonstration): {}", c.total_verifications_demonstration);
    println!();
    println!("⚠️  Missing Relations:");
    println!("Requirements not verified: {}", c.requirements_not_verified);
    println!("Requirements not satisfied: {}", c.requirements_not_satisfied);
}

#[derive(Serialize)]
pub struct CoverageReport {
    summary: CoverageSummary,
    satisfied_verifications: VerificationsByFile,
    unsatisfied_verifications: VerificationsByFile,
}

#[derive(Serialize)]
struct CoverageSummary {
    total_verifications: usize,
    total_satisfied: usize,
    total_unsatisfied: usize,
    coverage_percentage: f64,
    verification_types: VerificationTypeCounts,
}

#[derive(Serialize)]
struct VerificationTypeCounts {
    test: usize,
    analysis: usize,
    inspection: usize,
    demonstration: usize,
}

#[derive(Serialize)]
struct VerificationsByFile {
    files: HashMap<String, Vec<VerificationDetails>>,
}

#[derive(Serialize)]
struct VerificationDetails {
    identifier: String,
    name: String,
    section: String,
    verification_type: String,
    satisfied_by: Vec<String>,
}

impl CoverageReport {
    pub fn print(&self, json_output: bool) {
        if json_output {
            println!("{}", serde_json::to_string_pretty(&self).unwrap());
        } else {
            self.print_text();
        }
    }

    fn print_text(&self) {
        println!("=== Verification Coverage Report ===\n");
        
        // Summary
        println!("Summary:");
        println!("  Total Verifications: {}", self.summary.total_verifications);
        println!("  Satisfied: {} ({:.1}%)", self.summary.total_satisfied, self.summary.coverage_percentage);
        println!("  Unsatisfied: {}", self.summary.total_unsatisfied);
        println!();
        
        println!("Verification Types:");
        println!("  Test: {}", self.summary.verification_types.test);
        println!("  Analysis: {}", self.summary.verification_types.analysis);
        println!("  Inspection: {}", self.summary.verification_types.inspection);
        println!("  Demonstration: {}", self.summary.verification_types.demonstration);
        println!();
        
        // Satisfied verifications
        if !self.satisfied_verifications.files.is_empty() {
            println!("Satisfied Verifications:");
            for (file, verifications) in &self.satisfied_verifications.files {
                println!("  📂 {}", file);
                for verification in verifications {
                    println!("    ✅ {} ({})", verification.name, verification.verification_type);
                    if !verification.satisfied_by.is_empty() {
                        println!("       Satisfied by: {}", verification.satisfied_by.join(", "));
                    }
                }
                println!();
            }
        }
        
        // Unsatisfied verifications
        if !self.unsatisfied_verifications.files.is_empty() {
            println!("Unsatisfied Verifications:");
            for (file, verifications) in &self.unsatisfied_verifications.files {
                println!("  📂 {}", file);
                for verification in verifications {
                    println!("    ❌ {} ({})", verification.name, verification.verification_type);
                }
                println!();
            }
        }
    }
}

pub fn generate_coverage_report(registry: &GraphRegistry) -> CoverageReport {
    let mut total_verifications = 0;
    let mut total_satisfied = 0;
    let mut verification_types = VerificationTypeCounts {
        test: 0,
        analysis: 0,
        inspection: 0,
        demonstration: 0,
    };
    
    let mut satisfied_files: HashMap<String, Vec<VerificationDetails>> = HashMap::new();
    let mut unsatisfied_files: HashMap<String, Vec<VerificationDetails>> = HashMap::new();
    
    // Analyze all verification elements
    for element in registry.get_all_elements() {
        if let element::ElementType::Verification(verification_type) = &element.element_type {
            total_verifications += 1;
            
            // Count by verification type
            match verification_type {
                element::VerificationType::Default | element::VerificationType::Test => {
                    verification_types.test += 1;
                }
                element::VerificationType::Analysis => {
                    verification_types.analysis += 1;
                }
                element::VerificationType::Inspection => {
                    verification_types.inspection += 1;
                }
                element::VerificationType::Demonstration => {
                    verification_types.demonstration += 1;
                }
            }
            
            // Check if verification is satisfied (has satisfiedBy relations)
            let satisfied_by: Vec<String> = element.relations.iter()
                .filter(|r| relation::is_satisfaction_relation(r.relation_type))
                .map(|r| match &r.target.link {
                    relation::LinkType::Identifier(id) => id.clone(),
                    relation::LinkType::ExternalUrl(url) => url.clone(),
                    relation::LinkType::InternalPath(path) => path.to_string_lossy().to_string(),
                })
                .collect();
            
            let verification_details = VerificationDetails {
                identifier: element.identifier.clone(),
                name: element.name.clone(),
                section: element.section.clone(),
                verification_type: element.element_type.as_str().to_string(),
                satisfied_by: satisfied_by.clone(),
            };
            
            if satisfied_by.is_empty() {
                // Unsatisfied
                unsatisfied_files.entry(element.file_path.clone())
                    .or_insert_with(Vec::new)
                    .push(verification_details);
            } else {
                // Satisfied
                total_satisfied += 1;
                satisfied_files.entry(element.file_path.clone())
                    .or_insert_with(Vec::new)
                    .push(verification_details);
            }
        }
    }
    
    let coverage_percentage = if total_verifications > 0 {
        (total_satisfied as f64 / total_verifications as f64) * 100.0
    } else {
        0.0
    };
    
    CoverageReport {
        summary: CoverageSummary {
            total_verifications,
            total_satisfied,
            total_unsatisfied: total_verifications - total_satisfied,
            coverage_percentage,
            verification_types,
        },
        satisfied_verifications: VerificationsByFile {
            files: satisfied_files,
        },
        unsatisfied_verifications: VerificationsByFile {
            files: unsatisfied_files,
        },
    }
}
