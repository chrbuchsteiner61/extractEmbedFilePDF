//! CLI that extracts only files matching a given extension or MIME type.
//!
//! Usage:
//!   cargo run --example filter_files -- invoice.pdf --ext xml
//!   cargo run --example filter_files -- invoice.pdf --mime application/xml
//!   cargo run --example filter_files -- invoice.pdf --xml   (shorthand for --ext xml)

use extractembedfilepdf::PdfAnalyzer;
use std::{env, process};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!(
            "Usage: {} <pdf> [--ext <ext>] [--mime <mime>] [--xml]",
            args[0]
        );
        process::exit(1);
    }

    let pdf_path = &args[1];

    // Parse optional filter flags
    let filter_ext: Option<String> = args
        .windows(2)
        .find(|w| w[0] == "--ext")
        .map(|w| w[1].to_ascii_lowercase());

    let filter_mime: Option<String> = args
        .windows(2)
        .find(|w| w[0] == "--mime")
        .map(|w| w[1].to_ascii_lowercase());

    let xml_shorthand = args.contains(&"--xml".to_string());

    let analyzer = PdfAnalyzer::from_path(pdf_path).unwrap_or_else(|e| {
        eprintln!("Cannot load PDF: {e}");
        process::exit(1);
    });

    let all_files = analyzer.extract_embedded_files().unwrap_or_else(|e| {
        eprintln!("Extraction failed: {e}");
        process::exit(1);
    });

    // Apply filter
    let matched: Vec<_> = all_files
        .iter()
        .filter(|f| {
            if xml_shorthand {
                return f.has_extension("xml") || f.metadata.is_xml();
            }
            if let Some(ref ext) = filter_ext {
                if !f.has_extension(ext) {
                    return false;
                }
            }
            if let Some(ref mime) = filter_mime {
                if !f.metadata.has_mime_type(mime) {
                    return false;
                }
            }
            true
        })
        .collect();

    println!(
        "{} of {} file(s) matched the filter:",
        matched.len(),
        all_files.len()
    );

    for file in matched {
        println!("\n  {}", file.filename);
        println!("  Size : {} bytes", file.data.len());
        if let Some(ref mime) = file.metadata.mime_type {
            println!("  MIME : {mime}");
        }

        // For XML files print a short content preview
        if file.has_extension("xml") || file.metadata.is_xml() {
            if let Ok(text) = std::str::from_utf8(&file.data) {
                let preview: String = text.chars().take(120).collect();
                println!("  Preview: {preview}…");
            }
        }

        file.save_to_disk(".").unwrap_or_else(|e| {
            eprintln!("  Save failed: {e}");
        });
        println!("  Saved.");
    }
}
