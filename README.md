# extractEmbedFilePDF

A Rust library for validating PDF/A-3 documents and extracting their embedded files.

[![CI](https://github.com/chrbuchsteiner61/extractEmbedFilePDF/actions/workflows/ci.yml/badge.svg)](https://github.com/chrbuchsteiner61/extractEmbedFilePDF/actions)
[![crates.io](https://img.shields.io/crates/v/extractembedfilepdf.svg)](https://crates.io/crates/extractembedfilepdf)
[![docs.rs](https://docs.rs/extractembedfilepdf/badge.svg)](https://docs.rs/extractembedfilepdf)

---

## What it does

| Step | Method | Description |
|------|--------|-------------|
| 1 | `is_pdf()` | Verifies the document has a valid PDF catalog, pages, and trailer |
| 2 | `is_pdfa3()` | Reads the XMP metadata stream and checks for `pdfaid:part=3` |
| 3 | `has_embedded_files()` | Walks the PDF name tree and page annotations |
| 4 | `extract_embedded_files()` | Decodes and returns every embedded file with metadata |

---

## Add to your project

```toml
[dependencies]
extractembedfilepdf = "0.1.0"
```

---

## Quick start

```rust
use extractembedfilepdf::PdfAnalyzer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let analyzer = PdfAnalyzer::from_path("invoice.pdf")?;

    println!("Valid PDF : {}", analyzer.is_pdf()?);
    println!("PDF/A-3   : {}", analyzer.is_pdfa3()?);

    if analyzer.has_embedded_files()? {
        for file in analyzer.extract_embedded_files()? {
            println!("  {} — {} bytes", file.filename, file.data.len());
            file.save_to_disk("./output")?;
        }
    }
    Ok(())
}
```

---

## Loading options

```rust
use extractembedfilepdf::{PdfAnalyzer, ExtractorConfig};

// From a file path
let a = PdfAnalyzer::from_path("invoice.pdf")?;

// From an in-memory buffer (e.g. received over HTTP)
let bytes: Vec<u8> = std::fs::read("invoice.pdf")?;
let a = PdfAnalyzer::from_bytes(&bytes)?;

// With custom configuration
let cfg = ExtractorConfig {
    strict_pdfa3_validation: true,          // Err instead of Ok(false) when not PDF/A-3
    max_embedded_file_size: Some(10 << 20), // reject files > 10 MiB
    extract_to_disk: true,
    output_directory: Some("./extracted".into()),
};
let a = PdfAnalyzer::with_config("invoice.pdf", cfg)?;
```

---

## Working with extracted files

```rust
let files = analyzer.extract_embedded_files()?;

for file in &files {
    println!("name : {}", file.filename);
    println!("size : {} bytes", file.data.len());
    println!("ext  : {}", file.extension().unwrap_or("—"));

    if let Some(ref mime) = file.metadata.mime_type {
        println!("mime : {mime}");
    }
    if let Some(ref date) = file.metadata.modification_date {
        println!("date : {date}");  // PDF date format: D:YYYYMMDDHHmmSSOHH'mm'
    }

    // Filter helpers
    if file.has_extension("xml") || file.metadata.is_xml() {
        let xml = std::str::from_utf8(&file.data)?;
        println!("--- XML preview ---");
        println!("{}", &xml[..xml.len().min(200)]);
    }
}
```

---

## Conformance level

```rust
// "PDF/A-3B", "PDF/A-3A", "PDF/A-3U", or None
if let Some(level) = analyzer.conformance_level() {
    println!("Conformance: {level}");
}
```

---

## Error handling

```rust
use extractembedfilepdf::ExtractError;

match analyzer.extract_embedded_files() {
    Ok(files) => { /* … */ }
    Err(ExtractError::NoEmbeddedFiles)      => eprintln!("nothing to extract"),
    Err(ExtractError::NotPdfA3(reason))     => eprintln!("not PDF/A-3: {reason}"),
    Err(ExtractError::FileSizeExceeded)     => eprintln!("file too large"),
    Err(ExtractError::ExtractionError(f,r)) => eprintln!("failed on '{f}': {r}"),
    Err(e)                                  => eprintln!("other error: {e}"),
}
```

---

## CLI examples

```bash
# Extract every embedded file into the current directory
cargo run --example extract_files -- invoice.pdf

# Extract into a specific output directory
cargo run --example extract_files -- invoice.pdf ./output

# Extract only XML files
cargo run --example filter_files -- invoice.pdf --xml

# Extract by extension
cargo run --example filter_files -- invoice.pdf --ext pdf

# Extract by MIME type
cargo run --example filter_files -- invoice.pdf --mime application/xml
```

---

## Relation to extractXMLeRechnung

`extractEmbedFilePDF` is the reusable library extracted from the
[extractXMLeRechnung](https://github.com/chrbuchsteiner61/extractXMLeRechnung)
application.  Where that application targets e-invoice XML specifically,
this crate is general-purpose: it extracts **any** file type embedded in
**any** PDF/A-3 document.

---

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE) at your option.
