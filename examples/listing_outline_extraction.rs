use ops::prelude::*;
use ops::{batch, ListingOutline, OutlineEntry, OutlineMetadata, generate_outline_schema};
use serde_json::json;

// Mock structured query op for table of contents extraction
#[derive(Debug)]
struct ListingOutlineExtractionOp {
    mock_data: bool,
}

impl ListingOutlineExtractionOp {
    fn new() -> Self {
        Self { mock_data: true }
    }
}

#[async_trait]
impl Op<ListingOutline> for ListingOutlineExtractionOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<ListingOutline> {
        let content: String = dry.get("selected_content").unwrap_or_else(|| {
            "TABLE OF CONTENTS\n\nPart I: Foundations\nChapter 1: Introduction.............................3\n  1.1 Overview.....................................3\n  1.2 Scope........................................5\nChapter 2: Background..............................8\n\nPart II: Applications\nChapter 3: Methods................................15\n  3.1 Basic Methods...............................15\n  3.2 Advanced Techniques.........................18\nChapter 4: Results................................22\n\nAppendix A: Reference Data........................30\nIndex.............................................35".to_string()
        });
        
        println!("Deep Dive into content (length: {} chars)", content.len());
        
        // Create a realistic hierarchical Outline structure
        let mut outline = ListingOutline::new();
        outline.document_title = Some("Advanced Research Methods".to_string());
        
        // Part I with chapters and sections
        let mut part1 = OutlineEntry::new("Part I: Foundations".to_string(), Some("1".to_string()), 0)
            .with_type("part".to_string());
        
        let mut chapter1 = OutlineEntry::new("Chapter 1: Introduction".to_string(), Some("3".to_string()), 1)
            .with_type("chapter".to_string());
        chapter1.add_child(OutlineEntry::new("1.1 Overview".to_string(), Some("3".to_string()), 2)
            .with_type("section".to_string()));
        chapter1.add_child(OutlineEntry::new("1.2 Scope".to_string(), Some("5".to_string()), 2)
            .with_type("section".to_string()));
        
        let chapter2 = OutlineEntry::new("Chapter 2: Background".to_string(), Some("8".to_string()), 1)
            .with_type("chapter".to_string());
        
        part1.add_child(chapter1);
        part1.add_child(chapter2);
        
        // Part II with chapters and sections
        let mut part2 = OutlineEntry::new("Part II: Applications".to_string(), Some("15".to_string()), 0)
            .with_type("part".to_string());
        
        let mut chapter3 = OutlineEntry::new("Chapter 3: Methods".to_string(), Some("15".to_string()), 1)
            .with_type("chapter".to_string());
        chapter3.add_child(OutlineEntry::new("3.1 Basic Methods".to_string(), Some("15".to_string()), 2)
            .with_type("section".to_string()));
        chapter3.add_child(OutlineEntry::new("3.2 Advanced Techniques".to_string(), Some("18".to_string()), 2)
            .with_type("section".to_string()));
        
        let chapter4 = OutlineEntry::new("Chapter 4: Results".to_string(), Some("22".to_string()), 1)
            .with_type("chapter".to_string());
        
        part2.add_child(chapter3);
        part2.add_child(chapter4);
        
        // Appendices
        let appendix = OutlineEntry::new("Appendix A: Reference Data".to_string(), Some("30".to_string()), 0)
            .with_type("appendix".to_string());
        
        let index = OutlineEntry::new("Index".to_string(), Some("35".to_string()), 0)
            .with_type("index".to_string());
        
        outline.entries = vec![part1, part2, appendix, index];
        
        // Set metadata
        outline.metadata = OutlineMetadata {
            numbering_style: Some("mixed".to_string()),
            has_leaders: true,
            page_style: Some("arabic".to_string()),
            total_entries: outline.flatten().len(),
            levels: outline.max_depth() + 1,
            structure_type: Some("parts_chapters".to_string()),
        };
        
        outline.confidence = 0.95;
        
        Ok(outline)
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("ListingOutlineExtractionOp")
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
            .output_schema(generate_outline_schema())
            .build()
    }
}

// Mock content loading op
#[derive(Debug)]
struct LoadOutlineContentOp;

#[async_trait]
impl Op<()> for LoadOutlineContentOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        // Mock loading Outline content
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
        println!("Loaded Outline content for extraction");
        Ok(())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("LoadOutlineContentOp")
            .description("Loads table of contents content for extraction")
            .build()
    }
}

// Step-by-step pipeline using unit aggregation for mixed op types
batch! {
    OutlineLoadingStep<()> -> unit = [
        LoadOutlineContentOp
    ]
}

batch! {
    OutlineExtractionStep<ListingOutline> -> last = [
        ListingOutlineExtractionOp::new()
    ]
}

// Analysis ops for working with extracted Outline
#[derive(Debug)]
struct OutlineAnalysisOp;

#[async_trait]
impl Op<()> for OutlineAnalysisOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        let outline: ListingOutline = dry.get("extracted_outline")
            .ok_or_else(|| OpError::ExecutionFailed("No Outline found in context".to_string()))?;
        
        println!("\n TABLE OF CONTENTS ANALYSIS");
        println!("===============================");
        
        if let Some(title) = &outline.document_title {
            println!("Document: {}", title);
        }
        
        println!("Structure: {:?}", outline.metadata.structure_type);
        println!("Total entries: {}", outline.metadata.total_entries);
        println!("Hierarchy levels: {}", outline.metadata.levels);
        println!("Has page leaders: {}", outline.metadata.has_leaders);
        println!("Confidence: {:.1}%", outline.confidence * 100.0);
        
        println!("\nHIERARCHICAL STRUCTURE:");
        for entry in &outline.entries {
            print_entry(entry, 0);
        }
        
        println!("\n FLAT VIEW:");
        let flat = outline.flatten();
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
        
        println!("\n LEVEL ANALYSIS:");
        for level in 0..=outline.max_depth() {
            let entries_at_level = outline.entries_at_level(level);
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
        OpMetadata::builder("OutlineAnalysisOp")
            .description("Analyzes extracted table of contents structure")
            .build()
    }
}

fn print_entry(entry: &OutlineEntry, depth: usize) {
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
struct StoreOutlineOp;

#[async_trait]
impl Op<()> for StoreOutlineOp {
    async fn perform(&self, dry: &mut DryContext, _wet: &mut WetContext) -> OpResult<()> {
        // Get Outline from the extraction step and store it for analysis
        let outline: ListingOutline = dry.get("extracted_outline_temp")
            .ok_or_else(|| OpError::ExecutionFailed("No temporary Outline found".to_string()))?;
        dry.insert("extracted_outline", outline);
        Ok(())
    }
    
    fn metadata(&self) -> OpMetadata {
        OpMetadata::builder("StoreOutlineOp")
            .description("Stores extracted Outline for subsequent analysis")
            .build()
    }
}

// Analysis-only pipeline
batch! {
    OutlineAnalysisPipeline<()> -> unit = [
        OutlineAnalysisOp
    ]
}

#[tokio::main]
async fn main() -> OpResult<()> {
    let mut dry = DryContext::new();
    let mut wet = WetContext::new();
    
    println!(" TABLE OF CONTENTS EXTRACTION DEMO");
    println!("=====================================\n");
    
    // Step 1: Load content
    let loading_step = OutlineLoadingStep::new();
    loading_step.perform(&mut dry, &mut wet).await?;
    
    // Step 2: Extract Outline
    let extraction_step = OutlineExtractionStep::new();
    let extracted_outline = extraction_step.perform(&mut dry, &mut wet).await?;
    
    // Store for analysis
    dry.insert("extracted_outline", extracted_outline.clone());
    
    // Step 3: Analyze Outline
    let analysis_pipeline = OutlineAnalysisPipeline::new();
    analysis_pipeline.perform(&mut dry, &mut wet).await?;
    
    println!("\nOK SCHEMA VALIDATION");
    println!("====================");
    let schema = generate_outline_schema();
    println!("Generated JSON Schema with {} properties", 
        schema["properties"].as_object().unwrap().len());
    println!("Schema supports {} entry types", 
        schema["definitions"]["OutlineEntry"]["properties"]["entry_type"]["enum"]
            .as_array().unwrap().len());
    
    println!("\n USE CASES SUPPORTED:");
    println!("- Simple flat TOCs (title + page)");
    println!("- Chapter-based hierarchies");
    println!("- Part → Chapter → Section structures");
    println!("- Mixed academic/technical document formats");
    println!("- Books, papers, reports, manuals");
    println!("- Different numbering styles (numeric, roman, alphabetic)");
    
    Ok(())
}