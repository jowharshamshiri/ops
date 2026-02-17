use crate::prelude::*;
use ops::*;
use std::path::Path;
use tracing::{info, warn};

// Test with &mut OpContext
op!(generate_thumbnail(ctx: &mut OpContext, file_path: String) -> String {
    ctx.increment("thumbnails_generated");
    let path = Path::new(&file_path);
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    let thumbnail_name = format!("{}_thumb.jpg", 
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown"));
    
    match extension.as_str() {
        "pdf" => {
            info!("Generated PDF thumbnail for: {}", file_path);
            Ok(thumbnail_name)
        }
        "txt" | "md" => {
            info!("Generated text preview thumbnail for: {}", file_path);
            Ok(thumbnail_name)
        }
        "epub" => {
            info!("Generated EPUB cover thumbnail for: {}", file_path);
            Ok(thumbnail_name)
        }
        "docx" | "doc" => {
            info!("Generated Word document thumbnail for: {}", file_path);
            Ok(thumbnail_name)
        }
        _ => {
            warn!("Cannot generate thumbnail for unsupported format: {}", extension);
            Ok("default_thumb.jpg".to_string())
        }
    }
});

// Test with OpContext (should also work as reference)
op!(process_file(ctx: OpContext, file_path: String, priority: i32) -> String {
    ctx.set("last_processed".to_string(), serde_json::json!(file_path));
    ctx.increment("files_processed");
    
    info!("Processing file: {} with priority: {}", file_path, priority);
    
    if priority > 5 {
        Ok(format!("High priority processing of {}", file_path))
    } else {
        Ok(format!("Normal processing of {}", file_path))
    }
});

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    // TEST091: Use the op! macro to generate a thumbnail op and verify it processes the file and updates context
    #[tokio::test]
    async fn test_091_generate_thumbnail_macro() {
        let mut ctx = OpContext::new();
        ctx.put("file_path", "document.pdf".to_string());
        
        let op = GenerateThumbnail::new();
        let result = op.perform(&mut ctx).await;
        
        assert!(result.is_ok());
        
        // Check that context was modified
        let count: Option<i64> = ctx.get("thumbnails_generated");
        assert_eq!(count, Some(1));
        
        // Check result
        let thumbnail: Option<String> = ctx.get("result");
        assert_eq!(thumbnail, Some("document_thumb.jpg".to_string()));
    }

    // TEST092: Use the op! macro to generate a file processing op and verify context updates and result string
    #[tokio::test]
    async fn test_092_process_file_macro() {
        let mut ctx = OpContext::new();
        ctx.put("file_path", "important.txt".to_string());
        ctx.put("priority", 8i32);
        
        let op = ProcessFile::new();
        let result = op.perform(&mut ctx).await;
        
        assert!(result.is_ok());
        
        // Check that context was modified
        let count: Option<i64> = ctx.get("files_processed");
        assert_eq!(count, Some(1));
        
        let last_processed: Option<serde_json::Value> = ctx.get("last_processed");
        assert_eq!(last_processed, Some(serde_json::json!("important.txt")));
        
        // Check result
        let result_str: Option<String> = ctx.get("result");
        assert_eq!(result_str, Some("High priority processing of important.txt".to_string()));
    }
}