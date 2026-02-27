use crate::{ExtractorConfig, Result};
use lopdf::Document;
use std::path::Path;

// ── PdfAnalyzer ───────────────────────────────────────────────────────────────

/// Entry point for all PDF analysis and embedded-file extraction.
///
/// # Creating an analyzer
///
/// ```no_run
/// use extractembedfilepdf::{PdfAnalyzer, ExtractorConfig};
///
/// // From a file path
/// let a = PdfAnalyzer::from_path("invoice.pdf").unwrap();
///
/// // From an in-memory buffer
/// let bytes = std::fs::read("invoice.pdf").unwrap();
/// let a = PdfAnalyzer::from_bytes(&bytes).unwrap();
///
/// // With custom configuration
/// let cfg = ExtractorConfig {
///     strict_pdfa3_validation: true,
///     max_embedded_file_size: Some(10 * 1024 * 1024),
///     ..Default::default()
/// };
/// let a = PdfAnalyzer::with_config("invoice.pdf", cfg).unwrap();
/// ```
pub struct PdfAnalyzer {
    document: Document,
    config: ExtractorConfig,
}

impl PdfAnalyzer {
    // ── Constructors ──────────────────────────────────────────────────────────

    /// Load a PDF from the file system.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self {
            document: Document::load(path)?,
            config: ExtractorConfig::default(),
        })
    }

    /// Load a PDF from an in-memory byte slice.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        Ok(Self {
            document: Document::load_mem(data)?,
            config: ExtractorConfig::default(),
        })
    }

    /// Load a PDF from the file system with a custom [`ExtractorConfig`].
    pub fn with_config<P: AsRef<Path>>(path: P, config: ExtractorConfig) -> Result<Self> {
        Ok(Self {
            document: Document::load(path)?,
            config,
        })
    }

    // ── Accessors ─────────────────────────────────────────────────────────────

    /// Returns a reference to the underlying [`lopdf::Document`].
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// Returns a reference to the active [`ExtractorConfig`].
    pub fn config(&self) -> &ExtractorConfig {
        &self.config
    }
}
