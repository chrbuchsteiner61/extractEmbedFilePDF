use crate::extraction_engine::ExtractionEngine;
use crate::{EmbeddedFile, Result};

/// Extraction and file discovery functionality for PdfAnalyzer.
impl super::PdfAnalyzer {
    // ── Embedded file discovery ───────────────────────────────────────────────

    /// Returns `Ok(true)` when the document contains at least one embedded file.
    pub fn has_embedded_files(&self) -> Result<bool> {
        let engine = ExtractionEngine::new(self.document(), self.config());
        engine.has_files()
    }

    /// Returns the number of embedded files in the document.
    pub fn count_embedded_files(&self) -> Result<usize> {
        let engine = ExtractionEngine::new(self.document(), self.config());
        engine.count_files()
    }

    // ── Extraction ────────────────────────────────────────────────────────────

    /// Extract every embedded file from the document.
    ///
    /// Files are decoded (decompressed) before being returned. If
    /// [`ExtractorConfig::extract_to_disk`] is `true` and
    /// [`ExtractorConfig::output_directory`] is set, each file is also written
    /// to that directory immediately.
    ///
    /// Returns [`ExtractError::NoEmbeddedFiles`] when no file specifications
    /// are found, or when every specification fails to decode.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use extractembedfilepdf::PdfAnalyzer;
    ///
    /// let analyzer = PdfAnalyzer::from_path("invoice.pdf").unwrap();
    /// for file in analyzer.extract_embedded_files().unwrap() {
    ///     println!("{} — {} bytes", file.filename, file.data.len());
    ///     file.save_to_disk("./out").unwrap();
    /// }
    /// ```
    pub fn extract_embedded_files(&self) -> Result<Vec<EmbeddedFile>> {
        let engine = ExtractionEngine::new(self.document(), self.config());
        engine.extract_all_files()
    }
}