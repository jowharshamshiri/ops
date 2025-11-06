use crate::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// A hierarchical table of contents entry that can represent various TOC structures
/// 
/// This flexible model can handle:
/// - Simple flat TOCs: title + page
/// - Chapter-based TOCs: chapters with subsections
/// - Part-based TOCs: parts with chapters with sections
/// - Mixed hierarchies: any combination of the above
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocEntry {
    /// The title/heading text (required)
    pub title: String,
    
    /// Page number or range (e.g., "15", "15-20", "iv", "A-1")
    /// Optional because some entries might be section dividers
    pub page: Option<String>,
    
    /// Hierarchical level (0 = top level, 1 = subsection, etc.)
    /// Helps maintain structure without complex nesting
    pub level: u8,
    
    /// Optional type/category for semantic meaning
    /// Examples: "part", "chapter", "section", "appendix", "index"
    pub entry_type: Option<String>,
    
    /// Child entries for hierarchical structures
    /// Empty for leaf entries
    pub children: Vec<TocEntry>,
}

impl TocEntry {
    pub fn new(title: String, page: Option<String>, level: u8) -> Self {
        Self {
            title,
            page,
            level,
            entry_type: None,
            children: Vec::new(),
        }
    }
    
    pub fn with_type(mut self, entry_type: String) -> Self {
        self.entry_type = Some(entry_type);
        self
    }
    
    pub fn with_children(mut self, children: Vec<TocEntry>) -> Self {
        self.children = children;
        self
    }
    
    /// Add a child entry
    pub fn add_child(&mut self, child: TocEntry) {
        self.children.push(child);
    }
    
    /// Get all entries flattened with their hierarchical context
    pub fn flatten(&self) -> Vec<FlatTocEntry> {
        let mut result = Vec::new();
        self.flatten_recursive(&mut result, Vec::new());
        result
    }
    
    fn flatten_recursive(&self, result: &mut Vec<FlatTocEntry>, mut path: Vec<String>) {
        path.push(self.title.clone());
        
        result.push(FlatTocEntry {
            title: self.title.clone(),
            page: self.page.clone(),
            level: self.level,
            entry_type: self.entry_type.clone(),
            path: path.clone(),
        });
        
        for child in &self.children {
            child.flatten_recursive(result, path.clone());
        }
    }
}

/// A flattened representation of a TOC entry with full hierarchical path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatTocEntry {
    pub title: String,
    pub page: Option<String>,
    pub level: u8,
    pub entry_type: Option<String>,
    /// Full path from root to this entry (e.g., ["Part I", "Chapter 1", "Section 1.1"])
    pub path: Vec<String>,
}

/// Complete table of contents structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableOfContents {
    /// Document title (optional)
    pub document_title: Option<String>,
    
    /// Main TOC entries
    pub entries: Vec<TocEntry>,
    
    /// Extraction confidence (0.0 - 1.0)
    pub confidence: f64,
    
    /// Additional metadata about the TOC structure
    pub metadata: TocMetadata,
}

impl TableOfContents {
    pub fn new() -> Self {
        Self {
            document_title: None,
            entries: Vec::new(),
            confidence: 0.0,
            metadata: TocMetadata::default(),
        }
    }
    
    /// Get all entries as a flat list
    pub fn flatten(&self) -> Vec<FlatTocEntry> {
        self.entries.iter()
            .flat_map(|entry| entry.flatten())
            .collect()
    }
    
    /// Get entries at a specific level
    pub fn entries_at_level(&self, level: u8) -> Vec<&TocEntry> {
        fn collect_at_level<'a>(entries: &'a [TocEntry], target_level: u8, result: &mut Vec<&'a TocEntry>) {
            for entry in entries {
                if entry.level == target_level {
                    result.push(entry);
                }
                collect_at_level(&entry.children, target_level, result);
            }
        }
        
        let mut result = Vec::new();
        collect_at_level(&self.entries, level, &mut result);
        result
    }
    
    /// Get the maximum depth of the TOC
    pub fn max_depth(&self) -> u8 {
        fn max_depth_recursive(entries: &[TocEntry]) -> u8 {
            entries.iter()
                .map(|entry| {
                    let child_depth = if entry.children.is_empty() {
                        0
                    } else {
                        max_depth_recursive(&entry.children)
                    };
                    entry.level.max(child_depth)
                })
                .max()
                .unwrap_or(0)
        }
        
        max_depth_recursive(&self.entries)
    }
}

impl Default for TableOfContents {
    fn default() -> Self {
        Self::new()
    }
}

/// Metadata about the table of contents structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TocMetadata {
    /// Detected TOC style (e.g., "numeric", "roman", "alphabetic", "mixed")
    pub numbering_style: Option<String>,
    
    /// Whether the TOC uses dots or other leaders
    pub has_leaders: bool,
    
    /// Page numbering style (e.g., "arabic", "roman", "mixed")
    pub page_style: Option<String>,
    
    /// Total number of entries
    pub total_entries: usize,
    
    /// Number of hierarchical levels
    pub levels: u8,
    
    /// Detected structure type (e.g., "chapters", "parts_chapters", "sections")
    pub structure_type: Option<String>,
}

/// JSON Schema generation for table of contents
pub fn generate_toc_schema() -> serde_json::Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Table of Contents",
        "description": "Hierarchical table of contents structure that can represent various TOC formats",
        "type": "object",
        "properties": {
            "document_title": {
                "type": ["string", "null"],
                "description": "Title of the document (optional)"
            },
            "entries": {
                "type": "array",
                "description": "Main table of contents entries",
                "items": {
                    "$ref": "#/definitions/TocEntry"
                }
            },
            "confidence": {
                "type": "number",
                "minimum": 0.0,
                "maximum": 1.0,
                "description": "Confidence level of the extraction (0.0 - 1.0)"
            },
            "metadata": {
                "$ref": "#/definitions/TocMetadata"
            }
        },
        "required": ["entries", "confidence"],
        "definitions": {
            "TocEntry": {
                "type": "object",
                "description": "A single table of contents entry with optional hierarchy",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "The heading or title text"
                    },
                    "page": {
                        "type": ["string", "null"],
                        "description": "Page number or range (e.g., '15', '15-20', 'iv', 'A-1')"
                    },
                    "level": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 10,
                        "description": "Hierarchical level (0 = top level, 1 = subsection, etc.)"
                    },
                    "entry_type": {
                        "type": ["string", "null"],
                        "description": "Optional semantic type (e.g., 'part', 'chapter', 'section', 'appendix')",
                        "enum": ["part", "chapter", "section", "subsection", "appendix", "index", "bibliography", "preface", "introduction", "conclusion", null]
                    },
                    "children": {
                        "type": "array",
                        "description": "Child entries for hierarchical structures",
                        "items": {
                            "$ref": "#/definitions/TocEntry"
                        }
                    }
                },
                "required": ["title", "level"]
            },
            "TocMetadata": {
                "type": "object",
                "description": "Metadata about the table of contents structure",
                "properties": {
                    "numbering_style": {
                        "type": ["string", "null"],
                        "description": "Detected numbering style",
                        "enum": ["numeric", "roman", "alphabetic", "mixed", null]
                    },
                    "has_leaders": {
                        "type": "boolean",
                        "description": "Whether the TOC uses dots or other leaders"
                    },
                    "page_style": {
                        "type": ["string", "null"],
                        "description": "Page numbering style",
                        "enum": ["arabic", "roman", "alphabetic", "mixed", null]
                    },
                    "total_entries": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Total number of entries"
                    },
                    "levels": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 10,
                        "description": "Number of hierarchical levels"
                    },
                    "structure_type": {
                        "type": ["string", "null"],
                        "description": "Detected overall structure type",
                        "enum": ["flat", "chapters", "parts_chapters", "sections", "mixed", null]
                    }
                },
                "required": ["has_leaders", "total_entries", "levels"]
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_flat_toc() {
        let mut toc = TableOfContents::new();
        toc.entries = vec![
            TocEntry::new("Introduction".to_string(), Some("1".to_string()), 0),
            TocEntry::new("Chapter 1: Getting Started".to_string(), Some("5".to_string()), 0),
            TocEntry::new("Chapter 2: Advanced Topics".to_string(), Some("15".to_string()), 0),
            TocEntry::new("Conclusion".to_string(), Some("25".to_string()), 0),
        ];
        
        assert_eq!(toc.max_depth(), 0);
        assert_eq!(toc.entries_at_level(0).len(), 4);
        assert_eq!(toc.flatten().len(), 4);
    }

    #[test]
    fn test_hierarchical_toc() {
        let mut toc = TableOfContents::new();
        
        let mut chapter1 = TocEntry::new("Chapter 1: Basics".to_string(), Some("10".to_string()), 0)
            .with_type("chapter".to_string());
        chapter1.add_child(TocEntry::new("1.1 Introduction".to_string(), Some("10".to_string()), 1));
        chapter1.add_child(TocEntry::new("1.2 Fundamentals".to_string(), Some("15".to_string()), 1));
        
        let mut chapter2 = TocEntry::new("Chapter 2: Advanced".to_string(), Some("20".to_string()), 0)
            .with_type("chapter".to_string());
        chapter2.add_child(TocEntry::new("2.1 Complex Topics".to_string(), Some("20".to_string()), 1));
        
        toc.entries = vec![chapter1, chapter2];
        
        assert_eq!(toc.max_depth(), 1);
        assert_eq!(toc.entries_at_level(0).len(), 2);
        assert_eq!(toc.entries_at_level(1).len(), 3);
        assert_eq!(toc.flatten().len(), 5); // 2 chapters + 3 sections
    }

    #[test]
    fn test_complex_part_based_toc() {
        let mut toc = TableOfContents::new();
        
        // Part I with chapters
        let mut part1 = TocEntry::new("Part I: Foundations".to_string(), Some("1".to_string()), 0)
            .with_type("part".to_string());
        
        let mut chapter1 = TocEntry::new("Chapter 1: Introduction".to_string(), Some("3".to_string()), 1)
            .with_type("chapter".to_string());
        chapter1.add_child(TocEntry::new("1.1 Overview".to_string(), Some("3".to_string()), 2));
        chapter1.add_child(TocEntry::new("1.2 Scope".to_string(), Some("5".to_string()), 2));
        
        let chapter2 = TocEntry::new("Chapter 2: Background".to_string(), Some("8".to_string()), 1)
            .with_type("chapter".to_string());
        
        part1.add_child(chapter1);
        part1.add_child(chapter2);
        
        // Part II
        let part2 = TocEntry::new("Part II: Applications".to_string(), Some("15".to_string()), 0)
            .with_type("part".to_string());
        
        toc.entries = vec![part1, part2];
        
        assert_eq!(toc.max_depth(), 2);
        assert_eq!(toc.entries_at_level(0).len(), 2); // 2 parts
        assert_eq!(toc.entries_at_level(1).len(), 2); // 2 chapters
        assert_eq!(toc.entries_at_level(2).len(), 2); // 2 sections
        assert_eq!(toc.flatten().len(), 6); // 2 parts + 2 chapters + 2 sections
    }

    #[test]
    fn test_flatten_preserves_hierarchy() {
        let mut toc = TableOfContents::new();
        
        let mut part = TocEntry::new("Part I".to_string(), Some("1".to_string()), 0);
        let mut chapter = TocEntry::new("Chapter 1".to_string(), Some("3".to_string()), 1);
        chapter.add_child(TocEntry::new("Section 1.1".to_string(), Some("3".to_string()), 2));
        part.add_child(chapter);
        toc.entries = vec![part];
        
        let flat = toc.flatten();
        assert_eq!(flat.len(), 3);
        
        // Check paths
        assert_eq!(flat[0].path, vec!["Part I"]);
        assert_eq!(flat[1].path, vec!["Part I", "Chapter 1"]);
        assert_eq!(flat[2].path, vec!["Part I", "Chapter 1", "Section 1.1"]);
    }

    #[test]
    fn test_schema_generation() {
        let schema = generate_toc_schema();
        assert!(schema.is_object());
        assert!(schema["properties"]["entries"].is_object());
        assert!(schema["definitions"]["TocEntry"].is_object());
        assert!(schema["definitions"]["TocMetadata"].is_object());
    }
}