# extractembedfilepdf

## extractEmbedFilePDF

A Rust library for validating PDF/A-3 documents and extracting their embedded files.

### What this crate does

1. **Validate PDF** — checks that the bytes form a structurally valid PDF document.
2. **Validate PDF/A-3** — reads the XMP metadata stream and confirms the document
   declares PDF/A-3 conformance (part 3, level A, B, or U).
3. **Detect embedded files** — walks the PDF name tree and page annotations to find
   every embedded-file specification.
4. **Extract embedded files** — reads each embedded stream and returns the
   raw bytes together with filename and metadata.

### Quick example

```rust
use extractembedfilepdf::{PdfAnalyzer, ExtractorConfig};

let analyzer = PdfAnalyzer::from_path("invoice.pdf")?;

println!("Valid PDF : {}", analyzer.is_pdf()?);
println!("PDF/A-3   : {}", analyzer.is_pdfa3()?);

if analyzer.has_embedded_files()? {
    for file in analyzer.extract_embedded_files()? {
        println!("  {} — {} bytes", file.filename, file.data.len());
    }
}
```

License: MIT
