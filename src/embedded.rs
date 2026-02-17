use std::path::Path;

// ── EmbeddedFile ─────────────────────────────────────────────────────────────

/// A file that was embedded inside a PDF document.
///
/// Returned by [`crate::PdfAnalyzer::extract_embedded_files`].
#[derive(Debug, Clone)]
pub struct EmbeddedFile {
    /// The filename as declared in the PDF file specification object
    /// (Unicode name preferred over ASCII name when both are present).
    pub filename: String,

    /// The raw, decompressed file content.
    pub data: Vec<u8>,

    /// Optional metadata read from the PDF file specification and stream
    /// parameter dictionaries.
    pub metadata: EmbeddedFileMetadata,
}

impl EmbeddedFile {
    /// Write this file into `output_dir`, creating the directory if necessary.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use extractembedfilepdf::PdfAnalyzer;
    ///
    /// let analyzer = PdfAnalyzer::from_path("invoice.pdf").unwrap();
    /// for file in analyzer.extract_embedded_files().unwrap() {
    ///     file.save_to_disk("./extracted").unwrap();
    /// }
    /// ```
    pub fn save_to_disk<P: AsRef<Path>>(&self, output_dir: P) -> std::io::Result<()> {
        let dir = output_dir.as_ref();
        std::fs::create_dir_all(dir)?;
        std::fs::write(dir.join(&self.filename), &self.data)
    }

    /// Returns the file extension (lowercase), or `None` if the filename has
    /// no extension.
    ///
    /// ```
    /// # use extractembedfilepdf::{EmbeddedFile, EmbeddedFileMetadata};
    /// # let file = EmbeddedFile { filename: "factur-x.xml".into(), data: vec![], metadata: Default::default() };
    /// assert_eq!(file.extension(), Some("xml"));
    /// ```
    pub fn extension(&self) -> Option<&str> {
        Path::new(&self.filename)
            .extension()
            .and_then(|e| e.to_str())
    }

    /// Returns `true` when the file's extension matches `ext`
    /// (case-insensitive comparison).
    ///
    /// ```
    /// # use extractembedfilepdf::{EmbeddedFile, EmbeddedFileMetadata};
    /// # let file = EmbeddedFile { filename: "Factur-X.XML".into(), data: vec![], metadata: Default::default() };
    /// assert!(file.has_extension("xml"));
    /// ```
    pub fn has_extension(&self, ext: &str) -> bool {
        self.extension()
            .map(|e| e.eq_ignore_ascii_case(ext))
            .unwrap_or(false)
    }
}

// ── EmbeddedFileMetadata ──────────────────────────────────────────────────────

/// Metadata associated with an [`EmbeddedFile`], sourced from the PDF file
/// specification dictionary and the embedded stream's `/Params` sub-dictionary.
///
/// All fields are optional: a conforming PDF need not populate them.
#[derive(Debug, Clone, Default)]
pub struct EmbeddedFileMetadata {
    /// MIME type declared in the file specification's `/Subtype` entry
    /// (e.g. `"application/xml"`).
    pub mime_type: Option<String>,

    /// Human-readable description from the `/Desc` entry.
    pub description: Option<String>,

    /// Modification date from `/Params/ModDate` in PDF date format
    /// (`D:YYYYMMDDHHmmSSOHH'mm'`).
    pub modification_date: Option<String>,

    /// Creation date from `/Params/CreationDate`.
    pub creation_date: Option<String>,

    /// Uncompressed file size in bytes, from `/Params/Size`.
    pub size: Option<usize>,

    /// MD5 checksum hex string from `/Params/CheckSum`, if present.
    pub checksum: Option<String>,
}

impl EmbeddedFileMetadata {
    /// Returns `true` when the declared MIME type contains the string `"xml"`.
    pub fn is_xml(&self) -> bool {
        self.mime_type
            .as_deref()
            .map(|m| m.to_ascii_lowercase().contains("xml"))
            .unwrap_or(false)
    }

    /// Returns `true` when the declared MIME type matches `mime_type`
    /// (case-insensitive).
    pub fn has_mime_type(&self, mime_type: &str) -> bool {
        self.mime_type
            .as_deref()
            .map(|m| m.eq_ignore_ascii_case(mime_type))
            .unwrap_or(false)
    }
}
