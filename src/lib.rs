//! # extractEmbedFilePDF
//!
//! A Rust library for validating PDF/A-3 documents and extracting their embedded files.
//!
//! ## What this crate does
//!
//! 1. **Validate PDF** — checks that the bytes form a structurally valid PDF document.
//! 2. **Validate PDF/A-3** — reads the XMP metadata stream and confirms the document
//!    declares PDF/A-3 conformance (part 3, level A, B, or U).
//! 3. **Detect embedded files** — walks the PDF name tree and page annotations to find
//!    every embedded-file specification.
//! 4. **Extract embedded files** — reads each embedded stream and returns the
//!    raw bytes together with filename and metadata.
//!
//! ## Quick example
//!
//! ```no_run
//! use extractembedfilepdf::{PdfAnalyzer, ExtractorConfig};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let analyzer = PdfAnalyzer::from_path("invoice.pdf")?;
//!
//! println!("Valid PDF : {}", analyzer.is_pdf()?);
//! println!("PDF/A-3   : {}", analyzer.is_pdfa3()?);
//!
//! if analyzer.has_embedded_files()? {
//!     for file in analyzer.extract_embedded_files()? {
//!         println!("  {} — {} bytes", file.filename, file.data.len());
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use thiserror::Error;

mod analyzer;
mod embedded;
mod validator;

pub use analyzer::PdfAnalyzer;
pub use embedded::{EmbeddedFile, EmbeddedFileMetadata};
// PdfValidator is intentionally *not* re-exported; it is an internal detail.
// Callers use PdfAnalyzer for all operations.

// ── Configuration ────────────────────────────────────────────────────────────

/// Runtime configuration for [`PdfAnalyzer`].
#[derive(Debug, Clone, Default)]
pub struct ExtractorConfig {
    /// When `true`, [`PdfAnalyzer::is_pdfa3`] returns `Err` instead of `Ok(false)`
    /// if the document does not declare PDF/A-3 conformance.
    pub strict_pdfa3_validation: bool,

    /// If set, [`PdfAnalyzer::extract_embedded_files`] returns
    /// [`ExtractError::FileSizeExceeded`] as soon as any single embedded file
    /// exceeds this byte count.
    pub max_embedded_file_size: Option<usize>,

    /// If `true` and `output_directory` is also set, each successfully extracted
    /// file is written to disk automatically inside
    /// [`PdfAnalyzer::extract_embedded_files`].
    pub extract_to_disk: bool,

    /// Directory used when `extract_to_disk` is `true`.
    pub output_directory: Option<String>,
}

// ── Error type ───────────────────────────────────────────────────────────────

/// Every error that this crate can produce.
#[derive(Error, Debug)]
pub enum ExtractError {
    /// A filesystem I/O error occurred (e.g. when loading or saving a file).
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// The input bytes do not form a structurally valid PDF document.
    #[error("Invalid PDF: {0}")]
    InvalidPdf(String),

    /// The document exists but does not declare PDF/A-3 conformance.
    #[error("Not PDF/A-3: {0}")]
    NotPdfA3(String),

    /// The document was parsed successfully but contains no embedded files.
    #[error("No embedded files found in this PDF")]
    NoEmbeddedFiles,

    /// An embedded file was found but its stream could not be decoded.
    #[error("Failed to extract embedded file '{0}': {1}")]
    ExtractionError(String, String),

    /// The underlying lopdf parser returned an error.
    #[error("PDF parse error: {0}")]
    ParseError(#[from] lopdf::Error),

    /// An extracted file exceeds the configured `max_embedded_file_size` limit.
    #[error("Embedded file exceeds the configured maximum size")]
    FileSizeExceeded,
}

/// Convenience alias used throughout this crate.
pub type Result<T> = std::result::Result<T, ExtractError>;
