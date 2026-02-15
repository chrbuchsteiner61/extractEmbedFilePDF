use crate::{ExtractError, ExtractorConfig, Result};
use lopdf::Document;

// ── PdfValidator ──────────────────────────────────────────────────────────────
//
// This is an internal type.  Callers use PdfAnalyzer, which delegates here.

pub(crate) struct PdfValidator<'a> {
    document: &'a Document,
}

impl<'a> PdfValidator<'a> {
    pub(crate) fn new(document: &'a Document) -> Self {
        Self { document }
    }

    // ── Basic PDF structure ───────────────────────────────────────────────────

    /// Returns `Ok(true)` when the parsed document looks structurally valid.
    /// We rely on lopdf having already parsed the cross-reference table and
    /// object graph; here we just assert the mandatory elements are present.
    pub(crate) fn validate_pdf_structure(&self) -> Result<bool> {
        // Catalog must exist
        self.document
            .catalog()
            .map_err(|e| ExtractError::InvalidPdf(format!("missing or invalid catalog: {e}")))?;

        // At least one page must exist
        if self.document.get_pages().is_empty() {
            return Err(ExtractError::InvalidPdf("document has no pages".into()));
        }

        // Trailer must not be empty
        if self.document.trailer.is_empty() {
            return Err(ExtractError::InvalidPdf("missing trailer dictionary".into()));
        }

        Ok(true)
    }

    // ── PDF/A-3 conformance ───────────────────────────────────────────────────

    /// Returns `Ok(true)` when the document's XMP metadata stream declares
    /// PDF/A-3 conformance.
    ///
    /// All lopdf errors that arise from navigating the catalog → Metadata path
    /// are mapped to `ExtractError::NotPdfA3` so the caller gets a clear
    /// diagnostic rather than a raw parse error.
    pub(crate) fn validate_pdfa3(&self, config: &ExtractorConfig) -> Result<bool> {
        let xmp = self.read_xmp_metadata()?;
        let is_pdfa3 = Self::xmp_declares_pdfa3(&xmp);

        if config.strict_pdfa3_validation && !is_pdfa3 {
            return Err(ExtractError::NotPdfA3(
                "document XMP does not declare PDF/A-3 conformance".into(),
            ));
        }

        Ok(is_pdfa3)
    }

    /// Returns the conformance level string (e.g. `"PDF/A-3B"`) when the XMP
    /// metadata declares one, otherwise `None`.
    pub(crate) fn conformance_level(&self) -> Option<String> {
        let xmp = self.read_xmp_metadata().ok()?;
        Self::extract_conformance_level(&xmp)
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Walk catalog → /Metadata → stream → decompressed bytes → UTF-8 string.
    fn read_xmp_metadata(&self) -> Result<String> {
        let catalog = self.document.catalog().map_err(|e| {
            ExtractError::NotPdfA3(format!("cannot read catalog: {e}"))
        })?;

        let meta_obj = catalog.get(b"Metadata").map_err(|_| {
            ExtractError::NotPdfA3("catalog has no /Metadata entry".into())
        })?;

        let meta_id = meta_obj.as_reference().map_err(|_| {
            ExtractError::NotPdfA3("/Metadata entry is not an indirect reference".into())
        })?;

        let meta_object = self.document.get_object(meta_id).map_err(|e| {
            ExtractError::NotPdfA3(format!("cannot resolve /Metadata object: {e}"))
        })?;

        let stream = meta_object.as_stream().map_err(|_| {
            ExtractError::NotPdfA3("/Metadata object is not a stream".into())
        })?;

        let bytes = stream.decompressed_content().map_err(|e| {
            ExtractError::NotPdfA3(format!("cannot decompress /Metadata stream: {e}"))
        })?;

        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    /// Parse the XMP string for `pdfaid:part` = 3 and a valid
    /// `pdfaid:conformance` level (A, B, or U).
    ///
    /// XMP allows two serialisation forms:
    /// - attribute syntax  : `pdfaid:part="3"`
    /// - element syntax    : `<pdfaid:part>3</pdfaid:part>`
    fn xmp_declares_pdfa3(xmp: &str) -> bool {
        let has_part3 = xmp.contains(r#"pdfaid:part="3""#)
            || xmp.contains("<pdfaid:part>3</pdfaid:part>");

        if !has_part3 {
            return false;
        }

        // Conformance level must be A, B, or U (case-sensitive per the spec)
        for level in ["A", "B", "U"] {
            let attr = format!(r#"pdfaid:conformance="{level}""#);
            let elem = format!("<pdfaid:conformance>{level}</pdfaid:conformance>");
            if xmp.contains(&attr) || xmp.contains(&elem) {
                return true;
            }
        }

        false
    }

    /// Extract a human-readable conformance level string such as `"PDF/A-3B"`.
    fn extract_conformance_level(xmp: &str) -> Option<String> {
        // Determine part
        let part = if xmp.contains(r#"pdfaid:part="3""#)
            || xmp.contains("<pdfaid:part>3</pdfaid:part>")
        {
            "3"
        } else if xmp.contains(r#"pdfaid:part="2""#)
            || xmp.contains("<pdfaid:part>2</pdfaid:part>")
        {
            "2"
        } else if xmp.contains(r#"pdfaid:part="1""#)
            || xmp.contains("<pdfaid:part>1</pdfaid:part>")
        {
            "1"
        } else {
            return None;
        };

        // Determine conformance level
        for level in ["A", "B", "U"] {
            let attr = format!(r#"pdfaid:conformance="{level}""#);
            let elem = format!("<pdfaid:conformance>{level}</pdfaid:conformance>");
            if xmp.contains(&attr) || xmp.contains(&elem) {
                return Some(format!("PDF/A-{part}{level}"));
            }
        }

        None
    }
}
