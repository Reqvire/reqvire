use std::collections::HashMap;
use crate::relation::{Relation};
use crate::utils;
use serde::Serialize;


#[derive(Debug, PartialEq, Hash, Eq, Clone)]
pub enum SubSection {
    Other(String),
    Requirement,    
    Relations,
    Metadata,
    Details,
    Properties,
}
impl SubSection {
    pub fn name(&self) -> &str {
        match self {
            SubSection::Requirement => "Requirement",        
            SubSection::Relations => "Relations",
            SubSection::Metadata => "Metadata",
            SubSection::Details => "Details",
            SubSection::Properties => "Properties",
            SubSection::Other(name) => name.as_str(),            
        }
    }
    
    pub fn from_str(s: &str) -> Self {
        match s {
            "Requirement" => SubSection::Requirement,        
            "Relations" => SubSection::Relations,
            "Metadata" => SubSection::Metadata,
            "Details" =>   SubSection::Details,
            "Properties" => SubSection::Properties,
            other => SubSection::Other(other.to_string()),
        }
    }    
}




#[derive(Debug, Clone, PartialEq, Eq, Serialize)] 
pub enum RequirementType {
    System,
    User,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)] 
pub enum VerificationType {
    Default, 
    Test,
    Analysis,
    Inspection,
    Demonstration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)] 
pub enum ElementType {
    Requirement(RequirementType),
    Verification(VerificationType),
    File,
    Other(String),
}


impl ElementType {
    /// Returns the metadata key corresponding to this ElementType,
    /// e.g. "user_requirement", "analysis-verification", or the
    /// raw string for Other.
    pub fn as_str(&self) -> &str {
        match self {
            ElementType::Requirement(req) => match req {
                RequirementType::User   => "user-requirement",
                RequirementType::System => "requirement",
            },
            ElementType::Verification(ver) => match ver {
                VerificationType::Default       => "verification",            
                VerificationType::Test          => "verification",
                VerificationType::Analysis      => "analysis-verification",
                VerificationType::Inspection    => "inspection-verification",
                VerificationType::Demonstration => "demonstration-verification",
            },
            ElementType::File => "file",
            ElementType::Other(s) => s.as_str(),
        }
    }

    
    /// Parses a string into an ElementType
    pub fn from_metadata(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "user-requirement" => ElementType::Requirement(RequirementType::User),
            "requirement" => ElementType::Requirement(RequirementType::System),
            
            // Different verification types
            "verification" => ElementType::Verification(VerificationType::Test),
            "test-verification" => ElementType::Verification(VerificationType::Test),
            "analysis-verification" => ElementType::Verification(VerificationType::Analysis),
            "inspection-verification" => ElementType::Verification(VerificationType::Inspection),
            "demonstration-verification" => ElementType::Verification(VerificationType::Demonstration),
            
            "file" => ElementType::File,
            other => ElementType::Other(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Element {
    pub name: String,
    pub content: String,
    pub section: String,
    pub relations: Vec<Relation>,
    pub identifier: String,
    pub file_path: String,
    pub element_type: ElementType,
    pub metadata: HashMap<String, String>,
    //
    // hash of content that is taken into impact change detection
    pub hash_impact_content: String,
    //
    pub changed_since_commit: bool,
    //
    // Order index within the section (used for preserving original order)
    pub section_order_index: usize,
}



impl Element {
    pub fn new(name: &str, identifier: &str, file_path: &str, section: &str, element_type: Option<ElementType>) -> Self {
        Self {
            name: name.to_string(),
            content: "".to_string(),
            hash_impact_content: "".to_string(),
            section: section.to_string(),
            relations: vec![],
            identifier: identifier.to_string(),
            file_path: file_path.to_string(),
            element_type: element_type.unwrap_or(ElementType::Requirement(RequirementType::System)),
            metadata: HashMap::new(),
            changed_since_commit: false,
            section_order_index: 0, // Will be set during parsing
        }
    }

    pub fn add_relation(&mut self, relation: Relation) -> () {
    
    
      self.relations.push(relation);
    }

    pub fn add_content(&mut self, content: &str) {
        self.content.push_str(content);
    }

    pub fn freeze_content(&mut self) {
        // Trim newlines and tabs from the beginning and end.
        let trimmed = self.content.trim_matches(&['\n', '\t'][..]);     
                
        // Normalize content by removing all whitespace (spaces, tabs, newlines, etc.)
        let normalized: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();

        self.content=trimmed.to_string();
        self.hash_impact_content=utils::hash_content(&normalized);
    }
        
    pub fn set_type_from_metadata(&mut self) {
        if let Some(type_value) = self.metadata.get("type") {
            self.element_type = ElementType::from_metadata(type_value);
        }
     }
   
    pub fn extract_fragment(&self) -> String {
        match self.identifier.split_once('#') {
            Some((_, fragment)) => fragment.to_string(),
            None => "".to_string(),
        }
    }



}
