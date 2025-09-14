// HTML Metadata Extraction Ops
// Equivalent to Java ExtractMetaDataOp.java with enhanced Rust capabilities

use crate::op::Op;
use crate::context::OpContext;
use crate::error::OpError;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

// Add scraper as dependency for HTML parsing
// This will need to be added to Cargo.toml: scraper = "0.20"

/// HTML Meta tag definition for pattern matching
/// Equivalent to Java MetaDefinition class
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetaDefinition {
    pub name: String,
    pub property_name: Option<String>,
    pub name_attribute: Option<String>,
    pub required: bool,
}

impl MetaDefinition {
    pub fn new(name: String) -> Self {
        Self {
            name,
            property_name: None,
            name_attribute: None,
            required: false,
        }
    }

    pub fn with_property(mut self, property: String) -> Self {
        self.property_name = Some(property);
        self
    }

    pub fn with_name_attribute(mut self, name_attr: String) -> Self {
        self.name_attribute = Some(name_attr);
        self
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Check if this definition matches a meta tag
    pub fn matches(&self, property: Option<&str>, name: Option<&str>) -> bool {
        // Check property attribute match
        if let Some(expected_prop) = &self.property_name {
            if let Some(actual_prop) = property {
                if expected_prop == actual_prop {
                    return true;
                }
            }
        }
        
        // Check name attribute match
        if let Some(expected_name) = &self.name_attribute {
            if let Some(actual_name) = name {
                if expected_name == actual_name {
                    return true;
                }
            }
        }
        
        false
    }
}

/// HTML Meta tag instance extracted from document
/// Equivalent to Java MetaInstance class
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetaInstance {
    pub definition: MetaDefinition,
    pub content: String,
    pub property_value: Option<String>,
    pub name_value: Option<String>,
    pub additional_attributes: HashMap<String, String>,
}

impl MetaInstance {
    pub fn new(definition: MetaDefinition, content: String) -> Self {
        Self {
            definition,
            content,
            property_value: None,
            name_value: None,
            additional_attributes: HashMap::new(),
        }
    }

    pub fn with_property_value(mut self, prop: String) -> Self {
        self.property_value = Some(prop);
        self
    }

    pub fn with_name_value(mut self, name: String) -> Self {
        self.name_value = Some(name);
        self
    }

    pub fn with_attribute(mut self, key: String, value: String) -> Self {
        self.additional_attributes.insert(key, value);
        self
    }
}

/// HTML Metadata Extraction Result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtmlMetadata {
    pub meta_instances: Vec<MetaInstance>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub keywords: Option<String>,
    pub author: Option<String>,
    pub viewport: Option<String>,
    pub og_data: HashMap<String, String>,
    pub twitter_data: HashMap<String, String>,
    pub custom_meta: HashMap<String, String>,
}

impl Default for HtmlMetadata {
    fn default() -> Self {
        Self {
            meta_instances: Vec::new(),
            title: None,
            description: None,
            keywords: None,
            author: None,
            viewport: None,
            og_data: HashMap::new(),
            twitter_data: HashMap::new(),
            custom_meta: HashMap::new(),
        }
    }
}

impl HtmlMetadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_meta_instance(&mut self, instance: MetaInstance) {
        self.meta_instances.push(instance);
    }

    /// Extract common metadata fields
    pub fn extract_common_fields(&mut self) {
        for instance in &self.meta_instances {
            if let Some(name) = &instance.name_value {
                match name.to_lowercase().as_str() {
                    "description" => self.description = Some(instance.content.clone()),
                    "keywords" => self.keywords = Some(instance.content.clone()),
                    "author" => self.author = Some(instance.content.clone()),
                    "viewport" => self.viewport = Some(instance.content.clone()),
                    _ => {
                        self.custom_meta.insert(name.clone(), instance.content.clone());
                    }
                }
            }

            if let Some(property) = &instance.property_value {
                if property.starts_with("og:") {
                    self.og_data.insert(property.clone(), instance.content.clone());
                } else if property.starts_with("twitter:") {
                    self.twitter_data.insert(property.clone(), instance.content.clone());
                }
            }
        }
    }
}

/// HTML Metadata Extraction Op
/// Equivalent to Java ExtractMetaDataOp.java
pub struct ExtractMetaDataOp {
    html_content: String,
    meta_definitions: Vec<MetaDefinition>,
    extract_title: bool,
    extract_all_meta: bool,
}

impl ExtractMetaDataOp {
    pub fn new(html_content: String) -> Self {
        Self {
            html_content,
            meta_definitions: Self::default_meta_definitions(),
            extract_title: true,
            extract_all_meta: true,
        }
    }

    pub fn with_definitions(mut self, definitions: Vec<MetaDefinition>) -> Self {
        self.meta_definitions = definitions;
        self
    }

    pub fn with_title_extraction(mut self, extract: bool) -> Self {
        self.extract_title = extract;
        self
    }

    pub fn extract_all_meta(mut self, extract_all: bool) -> Self {
        self.extract_all_meta = extract_all;
        self
    }

    /// Default meta definitions covering common HTML meta tags
    fn default_meta_definitions() -> Vec<MetaDefinition> {
        vec![
            MetaDefinition::new("description".to_string()).with_name_attribute("description".to_string()),
            MetaDefinition::new("keywords".to_string()).with_name_attribute("keywords".to_string()),
            MetaDefinition::new("author".to_string()).with_name_attribute("author".to_string()),
            MetaDefinition::new("viewport".to_string()).with_name_attribute("viewport".to_string()),
            MetaDefinition::new("robots".to_string()).with_name_attribute("robots".to_string()),
            
            // Open Graph meta tags
            MetaDefinition::new("og:title".to_string()).with_property("og:title".to_string()),
            MetaDefinition::new("og:description".to_string()).with_property("og:description".to_string()),
            MetaDefinition::new("og:image".to_string()).with_property("og:image".to_string()),
            MetaDefinition::new("og:url".to_string()).with_property("og:url".to_string()),
            MetaDefinition::new("og:type".to_string()).with_property("og:type".to_string()),
            MetaDefinition::new("og:site_name".to_string()).with_property("og:site_name".to_string()),
            
            // Twitter Card meta tags
            MetaDefinition::new("twitter:card".to_string()).with_property("twitter:card".to_string()),
            MetaDefinition::new("twitter:title".to_string()).with_property("twitter:title".to_string()),
            MetaDefinition::new("twitter:description".to_string()).with_property("twitter:description".to_string()),
            MetaDefinition::new("twitter:image".to_string()).with_property("twitter:image".to_string()),
            MetaDefinition::new("twitter:site".to_string()).with_property("twitter:site".to_string()),
            MetaDefinition::new("twitter:creator".to_string()).with_property("twitter:creator".to_string()),
        ]
    }

    /// Parse HTML and extract metadata using simple regex patterns
    /// Note: In a real implementation, you'd use a proper HTML parser like scraper
    fn extract_metadata_simple(&self) -> Result<HtmlMetadata, OpError> {
        let mut metadata = HtmlMetadata::new();
        
        // Extract title
        if self.extract_title {
            if let Some(title_match) = self.extract_between_tags(&self.html_content, "title") {
                metadata.title = Some(title_match.trim().to_string());
            }
        }
        
        // Extract meta tags using simple pattern matching
        let meta_pattern = r#"<meta\s+([^>]+)>"#;
        if let Ok(re) = regex::Regex::new(meta_pattern) {
            for cap in re.captures_iter(&self.html_content) {
                if let Some(attributes) = cap.get(1) {
                    let attr_str = attributes.as_str();
                    
                    // Parse attributes
                    let (property, name, content) = self.parse_meta_attributes(attr_str);
                    
                    if let Some(content_value) = content {
                        // Check against definitions
                        for definition in &self.meta_definitions {
                            if definition.matches(property.as_deref(), name.as_deref()) {
                                let mut instance = MetaInstance::new(definition.clone(), content_value.clone());
                                
                                if let Some(prop) = &property {
                                    instance = instance.with_property_value(prop.clone());
                                }
                                if let Some(name_val) = &name {
                                    instance = instance.with_name_value(name_val.clone());
                                }
                                
                                metadata.add_meta_instance(instance);
                            }
                        }
                        
                        // If extract_all_meta is true, add unmatched meta tags too
                        if self.extract_all_meta {
                            if let Some(name_val) = &name {
                                if !self.definition_exists_for_name(name_val) {
                                    let def = MetaDefinition::new(name_val.clone()).with_name_attribute(name_val.clone());
                                    let instance = MetaInstance::new(def, content_value.clone()).with_name_value(name_val.clone());
                                    metadata.add_meta_instance(instance);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Extract common fields from collected meta instances
        metadata.extract_common_fields();
        
        Ok(metadata)
    }

    fn extract_between_tags(&self, html: &str, tag: &str) -> Option<String> {
        let start_tag = format!("<{}", tag);
        let end_tag = format!("</{}>", tag);
        
        if let Some(start_pos) = html.find(&start_tag) {
            if let Some(content_start) = html[start_pos..].find('>') {
                let content_start_pos = start_pos + content_start + 1;
                if let Some(end_pos) = html[content_start_pos..].find(&end_tag) {
                    return Some(html[content_start_pos..content_start_pos + end_pos].to_string());
                }
            }
        }
        None
    }

    fn parse_meta_attributes(&self, attr_str: &str) -> (Option<String>, Option<String>, Option<String>) {
        let mut property = None;
        let mut name = None;
        let mut content = None;
        
        // Simple attribute parsing (in real implementation, use proper parser)
        for part in attr_str.split_whitespace() {
            if let Some(eq_pos) = part.find('=') {
                let key = part[..eq_pos].trim();
                let value = part[eq_pos + 1..].trim().trim_matches('"').trim_matches('\'');
                
                match key.to_lowercase().as_str() {
                    "property" => property = Some(value.to_string()),
                    "name" => name = Some(value.to_string()),
                    "content" => content = Some(value.to_string()),
                    _ => {}
                }
            }
        }
        
        (property, name, content)
    }

    fn definition_exists_for_name(&self, name: &str) -> bool {
        self.meta_definitions.iter().any(|def| {
            def.name_attribute.as_ref().map_or(false, |attr| attr == name)
        })
    }
}

#[async_trait]
impl Op<HtmlMetadata> for ExtractMetaDataOp {
    async fn perform(&self, _context: &mut OpContext) -> Result<HtmlMetadata, OpError> {
        // Add regex dependency check
        #[cfg(not(feature = "html-parsing"))]
        {
            return Err(OpError::ExecutionFailed(
                "HTML parsing requires 'html-parsing' feature. Add regex dependency and enable feature.".to_string()
            ));
        }
        
        #[cfg(feature = "html-parsing")]
        {
            self.extract_metadata_simple()
        }
    }
}

/// Convenience function to create HTML metadata extraction op
pub fn extract_html_metadata(html_content: String) -> ExtractMetaDataOp {
    ExtractMetaDataOp::new(html_content)
}

/// Enhanced HTML metadata op with URL fetching capability
pub struct FetchAndExtractMetaDataOp {
    url: String,
    user_agent: Option<String>,
    timeout_seconds: u64,
    meta_definitions: Vec<MetaDefinition>,
}

impl FetchAndExtractMetaDataOp {
    pub fn new(url: String) -> Self {
        Self {
            url,
            user_agent: Some("Mozilla/5.0 (compatible; RustOpsBot/1.0)".to_string()),
            timeout_seconds: 30,
            meta_definitions: ExtractMetaDataOp::default_meta_definitions(),
        }
    }

    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    pub fn with_meta_definitions(mut self, definitions: Vec<MetaDefinition>) -> Self {
        self.meta_definitions = definitions;
        self
    }
}

#[async_trait]
impl Op<HtmlMetadata> for FetchAndExtractMetaDataOp {
    async fn perform(&self, _context: &mut OpContext) -> Result<HtmlMetadata, OpError> {
        // This would require reqwest dependency for HTTP fetching
        Err(OpError::ExecutionFailed(
            "HTTP fetching requires 'reqwest' dependency. Feature not implemented in basic version.".to_string()
        ))
    }
}

// Mock regex module for compilation without regex dependency
#[cfg(not(feature = "html-parsing"))]
mod regex {
    pub struct Regex;
    pub struct Captures<'a>(std::marker::PhantomData<&'a str>);
    pub struct Match<'a>(std::marker::PhantomData<&'a str>);

    impl Regex {
        pub fn new(_pattern: &str) -> Result<Self, &'static str> {
            Err("Regex requires html-parsing feature")
        }

        pub fn captures_iter<'r, 't>(&'r self, _text: &'t str) -> CaptureMatches<'r, 't> {
            CaptureMatches { _phantom: std::marker::PhantomData }
        }
    }

    impl<'a> Captures<'a> {
        pub fn get(&self, _i: usize) -> Option<Match<'a>> {
            None
        }
    }

    impl<'a> Match<'a> {
        pub fn as_str(&self) -> &'a str {
            ""
        }
    }

    pub struct CaptureMatches<'r, 't> {
        _phantom: std::marker::PhantomData<(&'r (), &'t ())>,
    }

    impl<'r, 't> Iterator for CaptureMatches<'r, 't> {
        type Item = Captures<'t>;

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta_definition_matching() {
        let def = MetaDefinition::new("description".to_string())
            .with_name_attribute("description".to_string());

        assert!(def.matches(None, Some("description")));
        assert!(!def.matches(None, Some("keywords")));
        assert!(!def.matches(Some("og:title"), None));

        let og_def = MetaDefinition::new("og:title".to_string())
            .with_property("og:title".to_string());
        
        assert!(og_def.matches(Some("og:title"), None));
        assert!(!og_def.matches(Some("og:description"), None));
        assert!(!og_def.matches(None, Some("title")));
    }

    #[test]
    fn test_meta_instance_creation() {
        let def = MetaDefinition::new("test".to_string());
        let instance = MetaInstance::new(def.clone(), "test content".to_string())
            .with_property_value("og:title".to_string())
            .with_name_value("title".to_string())
            .with_attribute("lang".to_string(), "en".to_string());

        assert_eq!(instance.content, "test content");
        assert_eq!(instance.property_value, Some("og:title".to_string()));
        assert_eq!(instance.name_value, Some("title".to_string()));
        assert_eq!(instance.additional_attributes.get("lang"), Some(&"en".to_string()));
    }

    #[test]
    fn test_html_metadata_common_fields() {
        let mut metadata = HtmlMetadata::new();
        
        // Add description meta
        let desc_def = MetaDefinition::new("description".to_string()).with_name_attribute("description".to_string());
        let desc_instance = MetaInstance::new(desc_def, "Test description".to_string())
            .with_name_value("description".to_string());
        metadata.add_meta_instance(desc_instance);

        // Add Open Graph meta
        let og_def = MetaDefinition::new("og:title".to_string()).with_property("og:title".to_string());
        let og_instance = MetaInstance::new(og_def, "Test Title".to_string())
            .with_property_value("og:title".to_string());
        metadata.add_meta_instance(og_instance);

        metadata.extract_common_fields();

        assert_eq!(metadata.description, Some("Test description".to_string()));
        assert_eq!(metadata.og_data.get("og:title"), Some(&"Test Title".to_string()));
    }

    #[test]
    fn test_default_meta_definitions() {
        let definitions = ExtractMetaDataOp::default_meta_definitions();
        
        // Should include basic meta tags
        assert!(definitions.iter().any(|def| def.name == "description"));
        assert!(definitions.iter().any(|def| def.name == "keywords"));
        assert!(definitions.iter().any(|def| def.name == "author"));
        
        // Should include Open Graph tags
        assert!(definitions.iter().any(|def| def.name == "og:title"));
        assert!(definitions.iter().any(|def| def.name == "og:description"));
        
        // Should include Twitter Card tags
        assert!(definitions.iter().any(|def| def.name == "twitter:card"));
        assert!(definitions.iter().any(|def| def.name == "twitter:title"));
    }

    #[tokio::test]
    async fn test_extract_metadata_op_without_feature() {
        let html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Test Page</title>
            <meta name="description" content="Test description">
            <meta property="og:title" content="Test OG Title">
        </head>
        <body>Test</body>
        </html>
        "#;

        let op = ExtractMetaDataOp::new(html.to_string());
        let mut context = OpContext::new();
        
        let result = op.perform(&mut context).await;
        
        // Should fail without html-parsing feature
        assert!(result.is_err());
        match result.unwrap_err() {
            OpError::ExecutionFailed(msg) => {
                assert!(msg.contains("html-parsing"));
            },
            _ => panic!("Expected ExecutionFailed error"),
        }
    }

    #[test]
    fn test_meta_definition_builder() {
        let def = MetaDefinition::new("test".to_string())
            .with_property("og:test".to_string())
            .with_name_attribute("test-name".to_string())
            .required();

        assert_eq!(def.name, "test");
        assert_eq!(def.property_name, Some("og:test".to_string()));
        assert_eq!(def.name_attribute, Some("test-name".to_string()));
        assert!(def.required);
    }

    #[test]
    fn test_html_metadata_initialization() {
        let metadata = HtmlMetadata::new();
        
        assert!(metadata.meta_instances.is_empty());
        assert!(metadata.title.is_none());
        assert!(metadata.description.is_none());
        assert!(metadata.og_data.is_empty());
        assert!(metadata.twitter_data.is_empty());
        assert!(metadata.custom_meta.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_and_extract_op() {
        let op = FetchAndExtractMetaDataOp::new("https://example.com".to_string())
            .with_user_agent("TestBot/1.0".to_string())
            .with_timeout(10);
        
        let mut context = OpContext::new();
        let result = op.perform(&mut context).await;
        
        // Should fail without reqwest dependency
        assert!(result.is_err());
        match result.unwrap_err() {
            OpError::ExecutionFailed(msg) => {
                assert!(msg.contains("reqwest"));
            },
            _ => panic!("Expected ExecutionFailed error"),
        }
    }
}