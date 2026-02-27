use crate::{pdf_utils, EmbeddedFile, EmbeddedFileMetadata, ExtractError, Result};
use lopdf::{Document, ObjectId};

/// Handles parsing of file specifications and extraction of embedded file data.
///
/// This module contains logic to:
/// - Parse file specification objects
/// - Extract and decompress stream content
/// - Read metadata from file specifications
pub struct FileSpecParser<'a> {
    document: &'a Document,
}

impl<'a> FileSpecParser<'a> {
    pub fn new(document: &'a Document) -> Self {
        Self { document }
    }

    /// Create an extraction error with consistent formatting.
    fn extraction_error(&self, name: &str, message: &str) -> ExtractError {
        ExtractError::ExtractionError(name.into(), message.into())
    }

    /// Get object from document and convert to dictionary with error context.
    fn get_dict_object(&self, obj_id: ObjectId, name: &str, context: &str) -> Result<lopdf::Dictionary> {
        let obj = self.document.get_object(obj_id)?;
        obj.as_dict()
            .map_err(|_| self.extraction_error(name, context))
            .cloned()
    }

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
    pub fn parse_file_spec(&self, name: &str, spec_id: ObjectId) -> Result<EmbeddedFile> {
        let spec_dict = self.get_dict_object(spec_id, name, "file spec is not a dictionary")?;
        let ef_dict = self.resolve_ef_dictionary(&spec_dict, name)?;
        let stream = self.extract_embedded_stream(&ef_dict, name)?;
        
        let data = stream
            .decompressed_content()
            .unwrap_or_else(|_| stream.content.clone());

        let filename = Self::best_filename(&spec_dict, name);
        let metadata = Self::read_metadata(&spec_dict, &stream.dict);

        Ok(EmbeddedFile {
            filename,
            data,
            metadata,
        })
    }

    /// Resolve the /EF dictionary, handling both inline and reference cases.
    fn resolve_ef_dictionary(&self, spec_dict: &lopdf::Dictionary, name: &str) -> Result<lopdf::Dictionary> {
        let ef_val = spec_dict
            .get(b"EF")
            .map_err(|_| self.extraction_error(name, "missing /EF entry"))?;

        if let Ok(ef_id) = ef_val.as_reference() {
            // Some producers incorrectly store /EF as a reference — handle both.
            self.get_dict_object(ef_id, name, "/EF reference is not a dict")
        } else {
            ef_val
                .as_dict()
                .map_err(|_| self.extraction_error(name, "/EF is not a dictionary"))
                .cloned()
        }
    }

    /// Extract the embedded file stream from the EF dictionary.
    fn extract_embedded_stream(&self, ef_dict: &lopdf::Dictionary, name: &str) -> Result<lopdf::Stream> {
        // /UF preferred over /F (unicode vs. ASCII path)
        let stream_ref = ef_dict
            .get(b"UF")
            .or_else(|_| ef_dict.get(b"F"))
            .map_err(|_| self.extraction_error(name, "/EF has neither /F nor /UF"))?;

        let stream_id = stream_ref
            .as_reference()
            .map_err(|_| self.extraction_error(name, "/EF stream entry is not a reference"))?;

        let stream_obj = self.document.get_object(stream_id)?;
        stream_obj
            .as_stream()
            .map_err(|_| self.extraction_error(name, "embedded stream object is not a stream"))
            .cloned()
    }

    /// Return the best available filename: Unicode (/UF) > ASCII (/F) > fallback.
    fn best_filename(spec_dict: &lopdf::Dictionary, fallback: &str) -> String {
        for key in [b"UF" as &[u8], b"F"] {
            if let Some(name) = pdf_utils::extract_string_from_dict(spec_dict, key) {
                return name;
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
        let mut metadata = EmbeddedFileMetadata::default();
        
        Self::read_spec_metadata(spec_dict, &mut metadata);
        Self::read_stream_params(stream_dict, &mut metadata);
        
        metadata
    }

    /// Read metadata from the file specification dictionary.
    fn read_spec_metadata(spec_dict: &lopdf::Dictionary, metadata: &mut EmbeddedFileMetadata) {
        // /Desc — human-readable description
        metadata.description = pdf_utils::extract_string_from_dict(spec_dict, b"Desc");

        // /Subtype — MIME type stored as a PDF name (e.g. /application#2Fxml)
        if let Ok(v) = spec_dict.get(b"Subtype") {
            if let Ok(name_bytes) = v.as_name() {
                let s = String::from_utf8_lossy(name_bytes);
                // PDF names use '#2F' for '/' — lopdf gives us the raw string;
                // normalise the separator.
                metadata.mime_type = Some(s.replace('#', "").to_ascii_lowercase());
            }
        }
    }

    /// Read metadata from the stream's /Params sub-dictionary.
    fn read_stream_params(stream_dict: &lopdf::Dictionary, metadata: &mut EmbeddedFileMetadata) {
        if let Ok(params_val) = stream_dict.get(b"Params") {
            if let Ok(params) = params_val.as_dict() {
                Self::read_date_params(params, metadata);
                Self::read_numeric_params(params, metadata);
                Self::read_checksum_param(params, metadata);
            }
        }
    }

    /// Read date-related parameters from the /Params dictionary.
    fn read_date_params(params: &lopdf::Dictionary, metadata: &mut EmbeddedFileMetadata) {
        metadata.modification_date = pdf_utils::extract_string_from_dict(params, b"ModDate");
        metadata.creation_date = pdf_utils::extract_string_from_dict(params, b"CreationDate");
    }

    /// Read numeric parameters from the /Params dictionary.
    fn read_numeric_params(params: &lopdf::Dictionary, metadata: &mut EmbeddedFileMetadata) {
        if let Ok(v) = params.get(b"Size") {
            if let Ok(n) = v.as_i64() {
                metadata.size = Some(n as usize);
            }
        }
    }

    /// Read checksum parameter from the /Params dictionary.
    fn read_checksum_param(params: &lopdf::Dictionary, metadata: &mut EmbeddedFileMetadata) {
        if let Ok(v) = params.get(b"CheckSum") {
            if let Ok(bytes) = v.as_str() {
                metadata.checksum = Some(hex_encode(bytes));
            }
        }
    }
}

/// Encode raw bytes as a lowercase hex string (used for the MD5 checksum).
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}