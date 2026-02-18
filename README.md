# extractembedfilepdf

A Rust library for validating PDF/A-3 documents and extracting embedded files with support for various file formats including XML invoices

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
extractembedfilepdf = "0.2.0"
```

## Usage

```rust
use extractembedfilepdf::{PdfAnalyzer, ValidationConfig};

// Create a PDF analyzer with default configuration
let mut analyzer = PdfAnalyzer::new();

// Or use a custom configuration
let config = ValidationConfig::default();
let mut analyzer = PdfAnalyzer::with_config(config);

// Extract embedded files from a PDF
let pdf_bytes = std::fs::read("document.pdf")?;
let embedded_files = analyzer.extract_embedded_files(&pdf_bytes)?;

// Process extracted files
for file in embedded_files {
    println!("Found file: {}", file.name);
    
    // Save file to disk
    file.save_to_disk(&file.name)?;
    
    // Check file extension
    if let Some(ext) = file.extension() {
        println!("Extension: {}", ext);
    }
}
```

## Features

- Extract embedded files from PDF/A-3 documents
- Validate PDF structure and compliance
- Support for various file formats including XML invoices
- Type-safe error handling with custom error types
- Configurable validation rules

## Examples

Check the [examples](examples/) directory for complete usage examples:

- `extract_files.rs` - Basic file extraction
- `filter_files.rs` - Filtering files by type/extension

## License

This project is licensed under the MIT license.

## Repository

[https://github.com/chrbuchsteiner61/extractEmbedFilePDF](https://github.com/chrbuchsteiner61/extractEmbedFilePDF)
