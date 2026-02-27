use crate::file_discovery::FileSpecDiscovery;
use crate::file_parsing::FileSpecParser;
use crate::{EmbeddedFile, ExtractError, ExtractorConfig, Result};
use lopdf::{Document, ObjectId};
use std::path::Path;

/// Central extraction engine that orchestrates the complete file extraction process.
pub struct ExtractionEngine<'a> {
    document: &'a Document,
    config: &'a ExtractorConfig,
}

impl<'a> ExtractionEngine<'a> {
    pub fn new(document: &'a Document, config: &'a ExtractorConfig) -> Self {
        Self { document, config }
    }

    /// Extract all embedded files from the document.
    pub fn extract_all_files(&self) -> Result<Vec<EmbeddedFile>> {
        let specs = self.discover_file_specs()?;
        let files = self.parse_and_process_files(specs);
        
        if files.is_empty() {
            return Err(ExtractError::NoEmbeddedFiles);
        }
        
        Ok(files)
    }

    /// Count embedded files in the document.
    pub fn count_files(&self) -> Result<usize> {
        let specs = FileSpecDiscovery::new(self.document).collect_file_specs()?;
        Ok(specs.len())
    }

    /// Check if document has embedded files.
    pub fn has_files(&self) -> Result<bool> {
        let specs = FileSpecDiscovery::new(self.document).collect_file_specs()?;
        Ok(!specs.is_empty())
    }

    /// Discover all file specifications in the document.
    fn discover_file_specs(&self) -> Result<Vec<(String, ObjectId)>> {
        let discovery = FileSpecDiscovery::new(self.document);
        let specs = discovery.collect_file_specs()?;

        if specs.is_empty() {
            return Err(ExtractError::NoEmbeddedFiles);
        }

        Ok(specs)
    }

    /// Parse file specifications and return successfully processed files.
    fn parse_and_process_files(&self, specs: Vec<(String, ObjectId)>) -> Vec<EmbeddedFile> {
        let parser = FileSpecParser::new(self.document);
        let mut results = Vec::new();

        for (name, spec_id) in specs {
            match self.process_single_file(&parser, &name, spec_id) {
                Some(file) => results.push(file),
                None => continue, // Error already logged
            }
        }

        results
    }

    /// Process a single file specification with validation and optional disk writing.
    fn process_single_file(
        &self,
        parser: &FileSpecParser,
        name: &str,
        spec_id: ObjectId,
    ) -> Option<EmbeddedFile> {
        // Parse the file
        let file = match parser.parse_file_spec(name, spec_id) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("extractEmbedFilePDF: warning: skipping '{}': {}", name, e);
                return None;
            }
        };

        // Validate and process
        if let Err(e) = self.validate_and_write_file(&file) {
            eprintln!("extractEmbedFilePDF: error processing '{}': {}", name, e);
            return None;
        }

        Some(file)
    }

    /// Validate file and optionally write to disk based on configuration.
    fn validate_and_write_file(&self, file: &EmbeddedFile) -> Result<()> {
        self.validate_file_size(file)?;
        self.write_file_if_configured(file)?;
        Ok(())
    }

    /// Validate that the file size doesn't exceed the configured maximum.
    fn validate_file_size(&self, file: &EmbeddedFile) -> Result<()> {
        if let Some(max_size) = self.config.max_embedded_file_size {
            if file.data.len() > max_size {
                return Err(ExtractError::FileSizeExceeded);
            }
        }
        Ok(())
    }

    /// Write the file to disk if extract_to_disk is enabled and output_directory is set.
    fn write_file_if_configured(&self, file: &EmbeddedFile) -> Result<()> {
        if !self.config.extract_to_disk {
            return Ok(());
        }

        let output_dir = match &self.config.output_directory {
            Some(dir) => dir,
            None => return Ok(()),
        };

        let dest = Path::new(output_dir).join(&file.filename);
        std::fs::create_dir_all(output_dir)?;
        std::fs::write(&dest, &file.data)?;
        
        Ok(())
    }
}