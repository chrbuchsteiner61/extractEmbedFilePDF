use crate::validator::PdfValidator;
use crate::{EmbeddedFile, EmbeddedFileMetadata, ExtractError, ExtractorConfig, Result};
use lopdf::{Document, ObjectId};
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
        PdfValidator::new(&self.document).validate_pdf_structure()
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
        PdfValidator::new(&self.document).validate_pdfa3(&self.config)
    }

    /// Returns the PDF/A conformance level string (e.g. `"PDF/A-3B"`) when the
    /// XMP metadata declares one, or `None` otherwise.
    pub fn conformance_level(&self) -> Option<String> {
        PdfValidator::new(&self.document).conformance_level()
    }

    // ── Embedded file discovery ───────────────────────────────────────────────

    /// Returns `Ok(true)` when the document contains at least one embedded file.
    pub fn has_embedded_files(&self) -> Result<bool> {
        Ok(!self.collect_file_specs()?.is_empty())
    }

    /// Returns the number of embedded files in the document.
    pub fn count_embedded_files(&self) -> Result<usize> {
        Ok(self.collect_file_specs()?.len())
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
        let specs = self.collect_file_specs()?;

        if specs.is_empty() {
            return Err(ExtractError::NoEmbeddedFiles);
        }

        let mut results: Vec<EmbeddedFile> = Vec::new();

        for (name, spec_id) in specs {
            match self.parse_file_spec(&name, spec_id) {
                Err(e) => {
                    // Warn but keep going — we want the other files even if
                    // one is malformed.
                    eprintln!("extractEmbedFilePDF: warning: skipping '{name}': {e}");
                }
                Ok(file) => {
                    if let Some(max) = self.config.max_embedded_file_size {
                        if file.data.len() > max {
                            return Err(ExtractError::FileSizeExceeded);
                        }
                    }

                    if self.config.extract_to_disk {
                        if let Some(ref dir) = self.config.output_directory {
                            let dest = Path::new(dir).join(&file.filename);
                            std::fs::create_dir_all(dir)?;
                            std::fs::write(&dest, &file.data)?;
                        }
                    }

                    results.push(file);
                }
            }
        }

        if results.is_empty() {
            return Err(ExtractError::NoEmbeddedFiles);
        }

        Ok(results)
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

    // ── Private: file-spec discovery ─────────────────────────────────────────

    /// Collect `(name, ObjectId)` pairs for every embedded-file specification
    /// in the document.
    ///
    /// Two sources are searched:
    /// 1. The `/Names/EmbeddedFiles` name tree in the document catalog.
    /// 2. `/FileAttachment` annotations on every page.
    fn collect_file_specs(&self) -> Result<Vec<(String, ObjectId)>> {
        let mut specs: Vec<(String, ObjectId)> = Vec::new();

        // --- Source 1: Names tree ---
        if let Ok(catalog) = self.document.catalog() {
            // catalog() returns &Dictionary; we use if-let chains to avoid ?
            if let Ok(names_val) = catalog.get(b"Names") {
                // /Names may be an inline dict or an indirect reference
                let names_dict = if let Ok(id) = names_val.as_reference() {
                    self.document
                        .get_object(id)
                        .ok()
                        .and_then(|o| o.as_dict().ok().cloned())
                } else {
                    names_val.as_dict().ok().cloned()
                };

                if let Some(names_dict) = names_dict {
                    if let Ok(ef_val) = names_dict.get(b"EmbeddedFiles") {
                        if let Ok(ef_id) = ef_val.as_reference() {
                            specs.extend(self.walk_name_tree(ef_id));
                        } else {
                            // Handle inline /EmbeddedFiles dictionary
                            if let Ok(ef_dict) = ef_val.as_dict() {
                                // For inline dictionaries, we need to check for /Names array directly
                                if let Ok(names_val) = ef_dict.get(b"Names") {
                                    if let Ok(names_array) = names_val.as_array() {
                                        let mut i = 0;
                                        while i + 1 < names_array.len() {
                                            if let Ok(name_bytes) = names_array[i].as_str() {
                                                let name = String::from_utf8_lossy(name_bytes)
                                                    .into_owned();
                                                if let Ok(spec_id) =
                                                    names_array[i + 1].as_reference()
                                                {
                                                    specs.push((name, spec_id));
                                                }
                                            }
                                            i += 2;
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // No /EmbeddedFiles in names dictionary
                    }
                } else {
                    // Could not get names dictionary
                }
            } else {
                // No /Names in catalog
            }
        } else {
            // Could not get catalog
        }

        // --- Source 2: page FileAttachment annotations ---
        let pages = self.document.get_pages();
        for page_id in pages.values() {
            if let Ok(page_obj) = self.document.get_object(*page_id) {
                if let Ok(page_dict) = page_obj.as_dict() {
                    if let Ok(annots_val) = page_dict.get(b"Annots") {
                        // Annots may be an inline array or a reference to one
                        let annots_array = if let Ok(id) = annots_val.as_reference() {
                            self.document
                                .get_object(id)
                                .ok()
                                .and_then(|o| o.as_array().ok().cloned())
                        } else {
                            annots_val.as_array().ok().cloned()
                        };

                        if let Some(arr) = annots_array {
                            for item in &arr {
                                if let Ok(annot_id) = item.as_reference() {
                                    if let Ok(annot_obj) = self.document.get_object(annot_id) {
                                        if let Ok(dict) = annot_obj.as_dict() {
                                            if let Ok(subtype_name) =
                                                dict.get(b"Subtype").and_then(|v| v.as_name())
                                            {
                                                if subtype_name == b"FileAttachment" {
                                                    if let Ok(fs_val) = dict.get(b"FS") {
                                                        if let Ok(fs_id) = fs_val.as_reference() {
                                                            let name = Self::annotation_name(dict);
                                                            specs.push((name, fs_id));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(specs)
    }

    /// Recursively walk a PDF name tree, collecting
    /// `(name_string, file_spec_object_id)` pairs from leaf nodes.
    fn walk_name_tree(&self, node_id: ObjectId) -> Vec<(String, ObjectId)> {
        let mut out = Vec::new();

        let node_obj = match self.document.get_object(node_id) {
            Ok(o) => o,
            Err(_) => return out,
        };
        let node_dict = match node_obj.as_dict() {
            Ok(d) => d,
            Err(_) => return out,
        };

        // Leaf node: has a /Names array of [key, value, key, value, …]
        if let Ok(names_val) = node_dict.get(b"Names") {
            if let Ok(arr) = names_val.as_array() {
                let mut i = 0;
                while i + 1 < arr.len() {
                    if let Ok(raw) = arr[i].as_str() {
                        let name = String::from_utf8_lossy(raw).into_owned();
                        if let Ok(spec_id) = arr[i + 1].as_reference() {
                            out.push((name, spec_id));
                        }
                    }
                    i += 2;
                }
            }
        }

        // Intermediate node: has a /Kids array of references
        if let Ok(kids_val) = node_dict.get(b"Kids") {
            if let Ok(kids) = kids_val.as_array() {
                for kid in kids {
                    if let Ok(kid_id) = kid.as_reference() {
                        out.extend(self.walk_name_tree(kid_id));
                    }
                }
            }
        }

        out
    }

    /// Extract a display name from a FileAttachment annotation dictionary.
    /// Falls back to `"attachment"` if neither `/Contents` nor `/T` is set.
    fn annotation_name(dict: &lopdf::Dictionary) -> String {
        for key in [b"Contents" as &[u8], b"T"] {
            if let Ok(v) = dict.get(key) {
                if let Ok(s) = v.as_str() {
                    let name = String::from_utf8_lossy(s).into_owned();
                    if !name.is_empty() {
                        return name;
                    }
                }
            }
        }
        "attachment".into()
    }

    // ── Private: file specification parsing and stream decoding ─────────────────

    /// Parse a file-specification object and return an [`EmbeddedFile`] with content and metadata.
    ///
    /// Layout of a file specification (PDF spec §7.11.3):
    ///
    /// ```text
    /// <<
    ///   /Type  /Filespec
    ///   /F     (ascii filename)
    ///   /UF    (unicode filename)          ← preferred
    ///   /Desc  (description)
    ///   /EF    <<
    ///              /F   <stream-ref>       ← the actual data stream
    ///              /UF  <stream-ref>       ← alternative key, same stream
    ///          >>
    /// >>
    /// ```
    ///
    /// The `/EF` entry is an **inline dictionary** (not a reference), but each
    /// of its values (`/F`, `/UF`) **is** an indirect reference to the stream
    /// object. The stream content is read and returned in the result.
    fn parse_file_spec(&self, name: &str, spec_id: ObjectId) -> Result<EmbeddedFile> {
        let spec_obj = self.document.get_object(spec_id)?;
        let spec_dict = spec_obj.as_dict().map_err(|_| {
            ExtractError::ExtractionError(name.into(), "file spec is not a dictionary".into())
        })?;

        // Resolve /EF — it is an inline dictionary, NOT an object reference.
        let ef_val = spec_dict
            .get(b"EF")
            .map_err(|_| ExtractError::ExtractionError(name.into(), "missing /EF entry".into()))?;

        let ef_dict = if let Ok(ef_id) = ef_val.as_reference() {
            // Some producers incorrectly store /EF as a reference — handle both.
            self.document
                .get_object(ef_id)?
                .as_dict()
                .map_err(|_| {
                    ExtractError::ExtractionError(name.into(), "/EF reference is not a dict".into())
                })?
                .clone()
        } else {
            ef_val
                .as_dict()
                .map_err(|_| {
                    ExtractError::ExtractionError(name.into(), "/EF is not a dictionary".into())
                })?
                .clone()
        };

        // /UF preferred over /F (unicode vs. ASCII path)
        let stream_ref = ef_dict
            .get(b"UF")
            .or_else(|_| ef_dict.get(b"F"))
            .map_err(|_| {
                ExtractError::ExtractionError(name.into(), "/EF has neither /F nor /UF".into())
            })?;

        let stream_id = stream_ref.as_reference().map_err(|_| {
            ExtractError::ExtractionError(name.into(), "/EF stream entry is not a reference".into())
        })?;

        let stream_obj = self.document.get_object(stream_id)?;
        let stream = stream_obj.as_stream().map_err(|_| {
            ExtractError::ExtractionError(
                name.into(),
                "embedded stream object is not a stream".into(),
            )
        })?;

        // Read and decompress the stream content
        let data = stream
            .decompressed_content()
            .unwrap_or_else(|_| stream.content.clone());

        let filename = Self::best_filename(spec_dict, name);
        let metadata = Self::read_metadata(spec_dict, &stream.dict);

        Ok(EmbeddedFile {
            filename,
            data,
            metadata,
        })
    }

    /// Return the best available filename: Unicode (/UF) > ASCII (/F) > fallback.
    fn best_filename(spec_dict: &lopdf::Dictionary, fallback: &str) -> String {
        for key in [b"UF" as &[u8], b"F"] {
            if let Ok(v) = spec_dict.get(key) {
                if let Ok(s) = v.as_str() {
                    let name = String::from_utf8_lossy(s).into_owned();
                    if !name.is_empty() {
                        return name;
                    }
                }
            }
        }
        fallback.into()
    }

    /// Read optional metadata from the file specification dictionary and the
    /// embedded stream's `/Params` sub-dictionary.
    fn read_metadata(
        spec_dict: &lopdf::Dictionary,
        stream_dict: &lopdf::Dictionary,
    ) -> EmbeddedFileMetadata {
        let mut m = EmbeddedFileMetadata::default();

        // /Desc — human-readable description
        if let Ok(v) = spec_dict.get(b"Desc") {
            if let Ok(s) = v.as_str() {
                m.description = Some(String::from_utf8_lossy(s).into_owned());
            }
        }

        // /Subtype — MIME type stored as a PDF name (e.g. /application#2Fxml)
        // lopdf decodes percent-encoded names automatically.
        if let Ok(v) = spec_dict.get(b"Subtype") {
            if let Ok(name_bytes) = v.as_name() {
                let s = String::from_utf8_lossy(name_bytes);
                // PDF names use '#2F' for '/' — lopdf gives us the raw string;
                // normalise the separator.
                m.mime_type = Some(s.replace('#', "").to_ascii_lowercase());
            }
        }

        // /Params — optional stream parameter dictionary
        if let Ok(params_val) = stream_dict.get(b"Params") {
            if let Ok(params) = params_val.as_dict() {
                if let Ok(v) = params.get(b"ModDate") {
                    if let Ok(s) = v.as_str() {
                        m.modification_date = Some(String::from_utf8_lossy(s).into_owned());
                    }
                }
                if let Ok(v) = params.get(b"CreationDate") {
                    if let Ok(s) = v.as_str() {
                        m.creation_date = Some(String::from_utf8_lossy(s).into_owned());
                    }
                }
                if let Ok(v) = params.get(b"Size") {
                    if let Ok(n) = v.as_i64() {
                        m.size = Some(n as usize);
                    }
                }
                if let Ok(v) = params.get(b"CheckSum") {
                    if let Ok(bytes) = v.as_str() {
                        m.checksum = Some(hex_encode(bytes));
                    }
                }
            }
        }

        m
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Encode raw bytes as a lowercase hex string (used for the MD5 checksum).
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
