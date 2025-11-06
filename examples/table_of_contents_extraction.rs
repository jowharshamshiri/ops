use ops::prelude::*;
use ops::{batch, TableOfContents, TocEntry, TocMetadata, generate_toc_schema};
use serde_json::json;

// Mock structured query op for table of contents extraction
#[derive(Debug)]
struct TableOfContentsExtractionOp {
    mock_data: bool,
}

impl TableOfContentsExtractionOp {
    fn new() -> Self {
        Self { mock_data: true }
    }
}

#[async_trait]
impl Op<TableOfContents> for TableOfContentsExtractionOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<TableOfContents> {
        let content: String = dry.get("selected_content").unwrap_or_else(|| {
            "TABLE OF CONTENTS\n\nPart I: Foundations\nChapter 1: Introduction.............................3\n  1.1 Overview.....................................3\n  1.2 Scope........................................5\nChapter 2: Background..............................8\n\nPart II: Applications\nChapter 3: Methods................................15\n  3.1 Basic Methods...............................15\n  3.2 Advanced Techniques.........................18\nChapter 4: Results................................22\n\nAppendix A: Reference Data........................30\nIndex.............................................35".to_string()
        });
        
        println!("Extracting TOC from content (length: {} chars)", content.len());
        
        // Create a realistic hierarchical TOC structure
        let mut toc = TableOfContents::new();
        toc.document_title = Some("Advanced Research Methods".to_string());
        
        // Part I with chapters and sections
        let mut part1 = TocEntry::new("Part I: Foundations".to_string(), Some("1".to_string()), 0)
            .with_type("part".to_string());
        
        let mut chapter1 = TocEntry::new("Chapter 1: Introduction".to_string(), Some("3".to_string()), 1)
            .with_type("chapter".to_string());
        chapter1.add_child(TocEntry::new("1.1 Overview".to_string(), Some("3".to_string()), 2)
            .with_type("section".to_string()));
        chapter1.add_child(TocEntry::new("1.2 Scope".to_string(), Some("5".to_string()), 2)
            .with_type("section".to_string()));
        
        let chapter2 = TocEntry::new("Chapter 2: Background".to_string(), Some("8".to_string()), 1)
            .with_type("chapter".to_string());
        
        part1.add_child(chapter1);
        part1.add_child(chapter2);
        
        // Part II with chapters and sections
        let mut part2 = TocEntry::new("Part II: Applications".to_string(), Some("15".to_string()), 0)
            .with_type("part".to_string());
        
        let mut chapter3 = TocEntry::new("Chapter 3: Methods".to_string(), Some("15".to_string()), 1)
            .with_type("chapter".to_string());
        chapter3.add_child(TocEntry::new("3.1 Basic Methods".to_string(), Some("15".to_string()), 2)
            .with_type("section".to_string()));
        chapter3.add_child(TocEntry::new("3.2 Advanced Techniques".to_string(), Some("18".to_string()), 2)
            .with_type("section".to_string()));
        
        let chapter4 = TocEntry::new("Chapter 4: Results".to_string(), Some("22".to_string()), 1)
            .with_type("chapter".to_string());
        
        part2.add_child(chapter3);
        part2.add_child(chapter4);
        
        // Appendices
        let appendix = TocEntry::new("Appendix A: Reference Data".to_string(), Some("30".to_string()), 0)
            .with_type("appendix".to_string());
        
        let index = TocEntry::new("Index".to_string(), Some("35".to_string()), 0)
            .with_type("index".to_string());
        
        toc.entries = vec![part1, part2, appendix, index];
        
        // Set metadata
        toc.metadata = TocMetadata {
            numbering_style: Some("mixed".to_string()),
            has_leaders: true,
            page_style: Some("arabic".to_string()),
            total_entries: toc.flatten().len(),
            levels: toc.max_depth() + 1,
            structure_type: Some("parts_chapters".to_string()),
        };
        
        toc.confidence = 0.95;
        
        Ok(toc)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("TableOfContentsExtractionOp")
            .description("Extracts hierarchical table of contents from document content")
            .input_schema(json!({
                "type": "object",
                "properties": {
                    "selected_content": {
                        "type": "string",
                        "description": "Document content containing table of contents"
                    }
                },
                "required": ["selected_content"]
            }))
            .output_schema(generate_toc_schema())
            .build()
    }
}

// Mock content loading op
#[derive(Debug)]
struct LoadTocContentOp;

#[async_trait]
impl Op<()> for LoadTocContentOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        // Mock loading TOC content
        let mock_content = r#"
TABLE OF CONTENTS

Preface................................................xi

Part I: Theoretical Foundations
Chapter 1: Introduction to Advanced Methods............3
  1.1 Historical Context.............................5
  1.2 Current State of the Field....................8
  1.3 Research Questions............................12

Chapter 2: Conceptual Framework.......................15
  2.1 Theoretical Models...........................17
  2.2 Methodological Approaches....................22
    2.2.1 Quantitative Methods.....................24
    2.2.2 Qualitative Methods......................28
  2.3 Integration Strategies.......................32

Part II: Practical Applications
Chapter 3: Case Studies..............................35
  3.1 Manufacturing Sector.........................37
  3.2 Service Industry............................42
  3.3 Technology Sector...........................48

Chapter 4: Implementation Guidelines.................52
  4.1 Planning Phase..............................54
  4.2 Execution Phase.............................58
  4.3 Evaluation Phase............................63

Part III: Advanced Topics
Chapter 5: Emerging Trends..........................68
Chapter 6: Future Directions........................75

Appendices
Appendix A: Statistical Tables......................82
Appendix B: Survey Instruments......................87
Appendix C: Interview Protocols.....................92

Bibliography.......................................96
Index.............................................103
        "#;
        
        dry.insert("selected_content", mock_content.to_string());
        println!("Loaded TOC content for extraction");
        Ok(())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("LoadTocContentOp")
            .description("Loads table of contents content for extraction")
            .build()
    }
}

// Step-by-step pipeline using unit aggregation for mixed op types
batch! {
    TocLoadingStep<()> -> unit = [
        LoadTocContentOp
    ]
}

batch! {
    TocExtractionStep<TableOfContents> -> last = [
        TableOfContentsExtractionOp::new()
    ]
}

// Analysis ops for working with extracted TOC
#[derive(Debug)]
struct TocAnalysisOp;

#[async_trait]
impl Op<()> for TocAnalysisOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        let toc: TableOfContents = dry.get("extracted_toc")
            .ok_or_else(|| OpError::ExecutionFailed("No TOC found in context".to_string()))?;
        
        println!("\n📚 TABLE OF CONTENTS ANALYSIS");
        println!("===============================");
        
        if let Some(title) = &toc.document_title {
            println!("Document: {}", title);
        }
        
        println!("Structure: {:?}", toc.metadata.structure_type);
        println!("Total entries: {}", toc.metadata.total_entries);
        println!("Hierarchy levels: {}", toc.metadata.levels);
        println!("Has page leaders: {}", toc.metadata.has_leaders);
        println!("Confidence: {:.1}%", toc.confidence * 100.0);
        
        println!("\n📑 HIERARCHICAL STRUCTURE:");
        for entry in &toc.entries {
            print_entry(entry, 0);
        }
        
        println!("\n📋 FLAT VIEW:");
        let flat = toc.flatten();
        for (i, entry) in flat.iter().enumerate() {
            let indent = "  ".repeat(entry.level as usize);
            let page_info = entry.page.as_ref()
                .map(|p| format!(" (page {})", p))
                .unwrap_or_default();
            let type_info = entry.entry_type.as_ref()
                .map(|t| format!(" [{}]", t))
                .unwrap_or_default();
            
            println!("{}{}. {}{}{}{}", 
                indent, i + 1, entry.title, page_info, type_info,
                if entry.level > 0 { format!(" → {}", entry.path.join(" → ")) } else { String::new() }
            );
        }
        
        println!("\n🔍 LEVEL ANALYSIS:");
        for level in 0..=toc.max_depth() {
            let entries_at_level = toc.entries_at_level(level);
            println!("Level {}: {} entries", level, entries_at_level.len());
            for entry in entries_at_level.iter().take(3) {
                println!("  - {}", entry.title);
            }
            if entries_at_level.len() > 3 {
                println!("  ... and {} more", entries_at_level.len() - 3);
            }
        }
        
        Ok(())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("TocAnalysisOp")
            .description("Analyzes extracted table of contents structure")
            .build()
    }
}

fn print_entry(entry: &TocEntry, depth: usize) {
    let indent = "  ".repeat(depth);
    let page_info = entry.page.as_ref()
        .map(|p| format!(" → page {}", p))
        .unwrap_or_default();
    let type_info = entry.entry_type.as_ref()
        .map(|t| format!(" [{}]", t))
        .unwrap_or_default();
    
    println!("{}{}{}{}", indent, entry.title, page_info, type_info);
    
    for child in &entry.children {
        print_entry(child, depth + 1);
    }
}

// Storage op to bridge between different return types
#[derive(Debug)]
struct StoreTocOp;

#[async_trait]
impl Op<()> for StoreTocOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        // Get TOC from the extraction step and store it for analysis
        let toc: TableOfContents = dry.get("extracted_toc_temp")
            .ok_or_else(|| OpError::ExecutionFailed("No temporary TOC found".to_string()))?;
        dry.insert("extracted_toc", toc);
        Ok(())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("StoreTocOp")
            .description("Stores extracted TOC for subsequent analysis")
            .build()
    }
}

// Analysis-only pipeline
batch! {
    TocAnalysisPipeline<()> -> unit = [
        TocAnalysisOp
    ]
}

#[tokio::main]
async fn main() -> OpResult<()> {
    let mut dry = DryContext::new();
    let mut wet = WetContext::new();
    
    println!("🚀 TABLE OF CONTENTS EXTRACTION DEMO");
    println!("=====================================\n");
    
    // Step 1: Load content
    let loading_step = TocLoadingStep::new();
    loading_step.perform(&mut dry, &mut wet).await?;
    
    // Step 2: Extract TOC
    let extraction_step = TocExtractionStep::new();
    let extracted_toc = extraction_step.perform(&mut dry, &mut wet).await?;
    
    // Store for analysis
    dry.insert("extracted_toc", extracted_toc.clone());
    
    // Step 3: Analyze TOC
    let analysis_pipeline = TocAnalysisPipeline::new();
    analysis_pipeline.perform(&mut dry, &mut wet).await?;
    
    println!("\n✅ SCHEMA VALIDATION");
    println!("====================");
    let schema = generate_toc_schema();
    println!("Generated JSON Schema with {} properties", 
        schema["properties"].as_object().unwrap().len());
    println!("Schema supports {} entry types", 
        schema["definitions"]["TocEntry"]["properties"]["entry_type"]["enum"]
            .as_array().unwrap().len());
    
    println!("\n🎯 USE CASES SUPPORTED:");
    println!("- Simple flat TOCs (title + page)");
    println!("- Chapter-based hierarchies");
    println!("- Part → Chapter → Section structures");
    println!("- Mixed academic/technical document formats");
    println!("- Books, papers, reports, manuals");
    println!("- Different numbering styles (numeric, roman, alphabetic)");
    
    Ok(())
}