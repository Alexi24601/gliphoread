//! Annotation CLI for gliphoread.
//!
//! Calls Tesseract to extract word-level bounding boxes from page images,
//! then outputs JSON for manual semantic role annotation.

use anyhow::{Context, Result};
use clap::Parser;
use image::GenericImageView;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Annotation CLI for extracting word bounding boxes via Tesseract.
#[derive(Parser, Debug)]
#[command(name = "annotate", about = "Extract word bounding boxes from page images")]
struct Args {
    /// Input image path (PNG)
    #[arg(short, long)]
    input: PathBuf,

    /// Output JSON file (default: <input-stem>_annot.json)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

/// A single word annotation with bounding box and text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordAnnotation {
    /// Bounding box [x1, y1, x2, y2] in pixel coordinates.
    pub bbox: [u32; 4],
    /// Recognized text (may be inaccurate for handwriting).
    pub text: String,
    /// Tesseract confidence score (0-100).
    pub confidence: f32,
}

/// Full page annotation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageAnnotation {
    /// Basename of the source page image.
    pub page: String,
    /// Full path to the source image.
    pub image_path: String,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Detected words with bounding boxes.
    pub words: Vec<WordAnnotation>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if !args.input.exists() {
        anyhow::bail!("Input file not found: {}", args.input.display());
    }

    let output = args.output.as_ref().map(|p| p.clone()).unwrap_or_else(|| {
        let stem = args.input
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        PathBuf::from(format!("{}_annot.json", stem))
    });

    let annotation = annotate_image(&args.input)?;

    let json = serde_json::to_string_pretty(&annotation)
        .context("Failed to serialize annotation")?;
    fs::write(&output, &json)
        .with_context(|| format!("Failed to write {}", output.display()))?;

    println!("Saved annotation to {}", output.display());
    println!(
        "Page: {}x{} — {} words detected",
        annotation.width, annotation.height, annotation.words.len()
    );

    Ok(())
}

/// Run Tesseract on a page image and return word-level bounding boxes.
fn annotate_image(image_path: &Path) -> Result<PageAnnotation> {
    // Get image dimensions
    let img = image::open(image_path)
        .context("Failed to open image")?;
    let (width, height) = img.dimensions();

    // Run Tesseract
    let output = Command::new("tesseract")
        .arg(image_path)
        .arg("stdout")
        .arg("--psm")
        .arg("6")
        .arg("tsv")
        .output()
        .context("Failed to run Tesseract")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Tesseract failed: {}", stderr);
    }

    // Parse Tesseract TSV output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut words = Vec::new();

    // Skip header line, process word-level rows (level 5)
    for (i, line) in stdout.lines().enumerate() {
        if i == 0 {
            continue; // Skip header
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 12 && parts[0] == "5" {
            let left: u32 = parts[6].parse().unwrap_or(0);
            let top: u32 = parts[7].parse().unwrap_or(0);
            let w: u32 = parts[8].parse().unwrap_or(0);
            let h: u32 = parts[9].parse().unwrap_or(0);
            let conf: f32 = parts[10].parse().unwrap_or(0.0);
            let text = parts[11].to_string();

            words.push(WordAnnotation {
                bbox: [left, top, left + w, top + h],
                text,
                confidence: conf,
            });
        }
    }

    let page = image_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    Ok(PageAnnotation {
        page,
        image_path: image_path.to_string_lossy().to_string(),
        width,
        height,
        words,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_annotation_serialization() {
        let annotation = PageAnnotation {
            page: "test.png".to_string(),
            image_path: "/path/to/test.png".to_string(),
            width: 2000,
            height: 3000,
            words: vec![
                WordAnnotation {
                    bbox: [100, 200, 300, 400],
                    text: "test".to_string(),
                    confidence: 85.0,
                },
            ],
        };

        let json = serde_json::to_string_pretty(&annotation).unwrap();
        let deserialized: PageAnnotation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.words.len(), 1);
        assert_eq!(deserialized.words[0].text, "test");
    }
}