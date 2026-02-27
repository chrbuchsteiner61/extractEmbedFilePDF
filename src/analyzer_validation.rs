use crate::validator::PdfValidator;
use crate::Result;

/// Validation functionality for PdfAnalyzer.
impl super::PdfAnalyzer {
    // ── Validation ────────────────────────────────────────────────────────────

    /// Returns `Ok(true)` when the loaded document is a structurally valid PDF.
    ///
    /// This verifies that the document has a catalog, at least one page, and a
    /// non-empty trailer — the three elements that lopdf already requires to
    /// parse the file, so an `Err` here is a programming error (e.g. an empty
    /// byte slice was passed to [`from_bytes`]).
    ///
    /// [`from_bytes`]: PdfAnalyzer::from_bytes
    pub fn is_pdf(&self) -> Result<bool> {
        PdfValidator::new(self.document()).validate_pdf_structure()
    }

    /// Returns `Ok(true)` when the XMP metadata declares PDF/A-3 conformance.
    ///
    /// Both attribute-style (`pdfaid:part="3"`) and element-style
    /// (`<pdfaid:part>3</pdfaid:part>`) XMP serialisations are recognised.
    /// Conformance levels A, B, and U are accepted.
    ///
    /// When [`ExtractorConfig::strict_pdfa3_validation`] is `true`, a document
    /// that is not PDF/A-3 causes `Err(ExtractError::NotPdfA3(…))` instead of
    /// `Ok(false)`.
    pub fn is_pdfa3(&self) -> Result<bool> {
        PdfValidator::new(self.document()).validate_pdfa3(self.config())
    }

    /// Returns the PDF/A conformance level string (e.g. `"PDF/A-3B"`) when the
    /// XMP metadata declares one, or `None` otherwise.
    pub fn conformance_level(&self) -> Option<String> {
        PdfValidator::new(self.document()).conformance_level()
    }
}