// Integration tests for extractEmbedFilePDF.
//
// These tests work without real PDF fixtures by testing type behaviour and the
// public API surface directly.  Tests that require actual PDF files would live
// in a `tests/fixtures/` directory and are marked `#[ignore]` so the CI pass
// even without those files.

use extractembedfilepdf::{EmbeddedFile, EmbeddedFileMetadata, ExtractError, ExtractorConfig};

// ── ExtractorConfig ───────────────────────────────────────────────────────────

#[test]
fn default_config_is_permissive() {
    let cfg = ExtractorConfig::default();
    assert!(!cfg.strict_pdfa3_validation);
    assert!(cfg.max_embedded_file_size.is_none());
    assert!(!cfg.extract_to_disk);
    assert!(cfg.output_directory.is_none());
}

#[test]
fn custom_config_round_trips() {
    let cfg = ExtractorConfig {
        strict_pdfa3_validation: true,
        max_embedded_file_size: Some(1024),
        extract_to_disk: true,
        output_directory: Some("./out".into()),
    };
    assert!(cfg.strict_pdfa3_validation);
    assert_eq!(cfg.max_embedded_file_size, Some(1024));
    assert!(cfg.extract_to_disk);
    assert_eq!(cfg.output_directory.as_deref(), Some("./out"));
}

// ── EmbeddedFile helpers ──────────────────────────────────────────────────────

fn make_file(filename: &str, data: &[u8]) -> EmbeddedFile {
    EmbeddedFile {
        filename: filename.into(),
        data: data.to_vec(),
        metadata: EmbeddedFileMetadata::default(),
    }
}

#[test]
fn extension_none_when_no_dot() {
    assert_eq!(make_file("readme", b"").extension(), None);
}

#[test]
fn save_to_disk_creates_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = make_file("test.txt", b"hello world");
    file.save_to_disk(dir.path()).unwrap();

    let written = std::fs::read(dir.path().join("test.txt")).unwrap();
    assert_eq!(written, b"hello world");
}

// ── ExtractError display ──────────────────────────────────────────────────────

#[test]
fn error_display_is_non_empty() {
    let errors: &[ExtractError] = &[
        ExtractError::InvalidPdf("test".into()),
        ExtractError::NotPdfA3("test".into()),
        ExtractError::NoEmbeddedFiles,
        ExtractError::ExtractionError("f".into(), "reason".into()),
        ExtractError::FileSizeExceeded,
    ];
    for e in errors {
        assert!(!e.to_string().is_empty(), "empty display for {e:?}");
    }
}

// ── PdfAnalyzer with invalid input ───────────────────────────────────────────

#[test]
fn from_bytes_rejects_empty_slice() {
    use extractembedfilepdf::PdfAnalyzer;
    assert!(PdfAnalyzer::from_bytes(&[]).is_err());
}

#[test]
fn from_bytes_rejects_non_pdf() {
    use extractembedfilepdf::PdfAnalyzer;
    assert!(PdfAnalyzer::from_bytes(b"not a pdf").is_err());
}

// ── Fixture-based tests (ignored without real PDFs) ───────────────────────────

/// To run: place a valid PDF/A-3 with embedded files at
/// `tests/fixtures/sample_pdfa3.pdf` and run with `--include-ignored`.
#[test]
#[ignore]
fn fixture_pdfa3_roundtrip() {
    use extractembedfilepdf::PdfAnalyzer;

    let bytes = std::fs::read("tests/fixtures/sample_pdfa3.pdf")
        .expect("place tests/fixtures/sample_pdfa3.pdf to run this test");

    let analyzer = PdfAnalyzer::from_bytes(&bytes).unwrap();
    assert!(analyzer.is_pdf().unwrap());
    assert!(analyzer.is_pdfa3().unwrap());
    assert!(analyzer.has_embedded_files().unwrap());

    let files = analyzer.extract_embedded_files().unwrap();
    assert!(!files.is_empty());
    for f in &files {
        assert!(!f.filename.is_empty());
        assert!(!f.data.is_empty());
    }
}
