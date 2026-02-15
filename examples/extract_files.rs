//! Minimal CLI that validates a PDF/A-3 file and writes out every embedded file.
//!
//! Usage:
//!   cargo run --example extract_files -- invoice.pdf
//!   cargo run --example extract_files -- invoice.pdf ./output

use extractembedfilepdf::{ExtractorConfig, PdfAnalyzer};
use std::{env, process};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <pdf_file> [output_dir]", args[0]);
        process::exit(1);
    }

    let pdf_path = &args[1];
    let output_dir = args.get(2).map(String::as_str);

    let config = ExtractorConfig {
        extract_to_disk: output_dir.is_some(),
        output_directory: output_dir.map(str::to_owned),
        ..Default::default()
    };

    println!("Analysing: {pdf_path}");

    let analyzer = PdfAnalyzer::with_config(pdf_path, config).unwrap_or_else(|e| {
        eprintln!("Error loading PDF: {e}");
        process::exit(1);
    });

    // 1. Is it a valid PDF?
    match analyzer.is_pdf() {
        Ok(true) => println!("✓ Valid PDF"),
        Ok(false) | Err(_) => {
            eprintln!("✗ Not a valid PDF");
            process::exit(1);
        }
    }

    // 2. Is it PDF/A-3?
    match analyzer.is_pdfa3() {
        Ok(true) => {
            let level = analyzer.conformance_level().unwrap_or_else(|| "PDF/A-3".into());
            println!("✓ {level}");
        }
        Ok(false) => println!("⚠ Not PDF/A-3 (proceeding anyway)"),
        Err(e) => println!("⚠ PDF/A-3 check failed: {e}"),
    }

    // 3. Are there embedded files?
    let count = analyzer.count_embedded_files().unwrap_or(0);
    if count == 0 {
        println!("  No embedded files found.");
        process::exit(0);
    }
    println!("✓ {count} embedded file(s)");

    // 4. Extract them.
    let files = analyzer.extract_embedded_files().unwrap_or_else(|e| {
        eprintln!("Extraction error: {e}");
        process::exit(1);
    });

    let save_dir = output_dir.unwrap_or(".");
    for (i, file) in files.iter().enumerate() {
        println!("\n  File #{}", i + 1);
        println!("    Name : {}", file.filename);
        println!("    Size : {} bytes", file.data.len());
        if let Some(ref mime) = file.metadata.mime_type {
            println!("    MIME : {mime}");
        }
        if let Some(ref date) = file.metadata.modification_date {
            println!("    Date : {date}");
        }
        if output_dir.is_none() {
            // config.extract_to_disk was false, so save manually
            match file.save_to_disk(save_dir) {
                Ok(_) => println!("    ✓ Saved to {save_dir}/{}", file.filename),
                Err(e) => eprintln!("    ✗ Save failed: {e}"),
            }
        } else {
            println!("    ✓ Saved to {save_dir}/{}", file.filename);
        }
    }
}
