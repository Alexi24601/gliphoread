//! Data models for gliphoread annotations.

use serde::{Deserialize, Serialize};

/// Semantic role of a word in a recipe page.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum Role {
    /// Recipe name / header.
    Title,
    /// Bullet marker or numbering.
    ListItem,
    /// Ingredient name ("uova", "zucchero", "farina").
    Ingredient,
    /// Numeric quantity ("180", "160°", "60'").
    Measurement,
    /// Measurement unit ("g", "Kg", "ml").
    Unit,
    /// Preparation step text or cooking params ("160° 60'").
    Instruction,
    /// Non-recipe text, notes, marginalia.
    Comment,
    /// Unlabeled word (default).
    #[default]
    Unlabeled,
}

impl Role {
    /// Convert to string label for display.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::ListItem => "list_item",
            Self::Ingredient => "ingredient",
            Self::Measurement => "measurement",
            Self::Unit => "unit",
            Self::Instruction => "instruction",
            Self::Comment => "comment",
            Self::Unlabeled => "unlabeled",
        }
    }

    /// CSS color for visual distinction.
    pub fn color(&self) -> &'static str {
        match self {
            Self::Title => "#ef4444",
            Self::ListItem => "#8b5cf6",
            Self::Ingredient => "#22c55e",
            Self::Measurement => "#3b82f6",
            Self::Unit => "#f59e0b",
            Self::Instruction => "#a855f7",
            Self::Comment => "#6b7280",
            Self::Unlabeled => "#9ca3af",
        }
    }
}

/// A single word annotation with bounding box and semantic role.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordAnnotation {
    /// Bounding box [x1, y1, x2, y2] in pixel coordinates.
    pub bbox: [u32; 4],
    /// Recognized text (may be inaccurate for handwriting).
    pub text: String,
    /// Tesseract confidence score (0-100).
    #[serde(default)]
    pub confidence: f32,
    /// Semantic role of this word.
    #[serde(default)]
    pub role: Role,
    /// Ordinal position within the ingredient list (1..30), or None.
    #[serde(default)]
    pub list_pos: Option<u8>,
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

impl PageAnnotation {
    /// Create a new empty annotation for a page.
    pub fn new(path: &std::path::Path, width: u32, height: u32) -> Self {
        Self {
            page: path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default(),
            image_path: path.to_string_lossy().to_string(),
            width,
            height,
            words: Vec::new(),
        }
    }

    /// Add a new word annotation.
    pub fn add_word(&mut self, word: WordAnnotation) {
        self.words.push(word);
    }

    /// Remove a word annotation by index.
    pub fn remove_word(&mut self, index: usize) {
        if index < self.words.len() {
            self.words.remove(index);
        }
    }
}