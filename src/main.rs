//! CLI tool for extracting embedded files from PDF/A-3 documents.
//!
//! This binary demonstrates the capabilities of the extractembedfilepdf crate
//! and provides a command-line interface for PDF analysis and file extraction.

use extractembedfilepdf::{ExtractorConfig, PdfAnalyzer, Result};
use std::{env, fs, process};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_usage(&args[0]);
        process::exit(if args.len() < 2 { 1 } else { 0 });
    }

    let pdf_path = &args[1];
    let output_dir = if args.len() > 2 { Some(args[2].as_str()) } else { None };

    // Determine output directory and create it if necessary  
    let final_output_dir = output_dir.unwrap_or("extracted_files");
    if let Some(dir) = output_dir {
        if let Err(e) = fs::create_dir_all(dir) {
            eprintln!("❌ Failed to create output directory '{}': {}", dir, e);
            process::exit(1);
        }
    }

    match run_analysis(pdf_path, final_output_dir) {
        Ok(()) => println!("\n✅ Analysis completed successfully!"),
        Err(e) => {
            eprintln!("\n❌ Error: {}", e);
            process::exit(1);
        }
    }
}

fn print_usage(program_name: &str) {
    println!("📄 extractEmbedFilePDF - PDF/A-3 Analysis & File Extraction Tool");
    println!();
    println!("USAGE:");
    println!("    {} <pdf_file> [output_dir]", program_name);
    println!();
    println!("ARGUMENTS:");
    println!("    <pdf_file>     Path to the PDF file to analyze");
    println!("    [output_dir]   Directory to extract files to (default: 'extracted_files')");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help     Show this help message");
    println!();
    println!("EXAMPLES:");
    println!("    {} invoice.pdf", program_name);
    println!("    {} document.pdf ./output", program_name);
    println!();
    println!("This tool will:");
    println!("  • Validate the PDF structure");
    println!("  • Check for PDF/A-3 conformance");
    println!("  • List all embedded files with metadata");
    println!("  • Extract embedded files to the output directory");
}

fn run_analysis(pdf_path: &str, output_dir: &str) -> Result<()> {
    println!("🔍 Analyzing PDF: {}", pdf_path);
    println!("📁 Output directory: {}", output_dir);
    println!("{}", "─".repeat(60));

    // Create analyzer with configuration
    let config = ExtractorConfig {
        extract_to_disk: true,
        output_directory: Some(output_dir.to_string()),
        max_embedded_file_size: Some(100 * 1024 * 1024), // 100MB limit
        strict_pdfa3_validation: false,
    };

    let analyzer = PdfAnalyzer::with_config(pdf_path, config)?;

    // Step 1: Validate PDF structure
    print!("📋 Checking PDF structure... ");
    match analyzer.is_pdf() {
        Ok(true) => {
            println!("✅ Valid PDF");
        }
        Ok(false) => {
            println!("❌ Invalid PDF structure");
            return Ok(());
        }
        Err(e) => {
            println!("❌ Validation failed: {}", e);
            return Err(e);
        }
    }

    // Step 2: Check PDF/A-3 conformance
    print!("🔖 Checking PDF/A-3 conformance... ");
    match analyzer.is_pdfa3() {
        Ok(true) => {
            let level = analyzer
                .conformance_level()
                .unwrap_or_else(|| "PDF/A-3".to_string());
            println!("✅ {}", level);
        }
        Ok(false) => {
            println!("⚠️  Not PDF/A-3 compliant (will proceed anyway)");
        }
        Err(e) => {
            println!("⚠️  PDF/A-3 check failed: {} (will proceed anyway)", e);
        }
    }

    // Step 3: Check for embedded files
    print!("📎 Scanning for embedded files... ");
    let count = analyzer.count_embedded_files().unwrap_or(0);
    
    if count == 0 {
        println!("ℹ️  No embedded files found");
        return Ok(());
    }
    
    println!("✅ Found {} embedded file(s)", count);

    // Step 4: Extract files
    println!("\n🚀 Extracting embedded files:");
    println!("{}", "─".repeat(60));

    let files = analyzer.extract_embedded_files()?;
    
    // Ensure output directory exists
    fs::create_dir_all(output_dir)?;

    for (i, file) in files.iter().enumerate() {
        println!("\n📄 File #{}: {}", i + 1, file.filename);
        println!("   📏 Size: {} bytes", format_bytes(file.data.len()));
        
        if let Some(ref description) = file.metadata.description {
            println!("   📝 Description: {}", description);
        }
        
        if let Some(ref mime_type) = file.metadata.mime_type {
            println!("   🏷️  MIME Type: {}", mime_type);
        }
        
        if let Some(ref creation_date) = file.metadata.creation_date {
            println!("   📅 Created: {}", creation_date);
        }
        
        if let Some(ref modification_date) = file.metadata.modification_date {
            println!("   📅 Modified: {}", modification_date);
        }

        // Files are automatically saved because extract_to_disk is true
        let file_path = format!("{}/{}", output_dir, file.filename);
        println!("   💾 Saved to: {}", file_path);
    }

    println!("\n{}", "─".repeat(60));
    println!("📊 Summary:");
    println!("   • {} file(s) extracted successfully", files.len());
    
    let total_size: usize = files.iter().map(|f| f.data.len()).sum();
    println!("   • Total size: {}", format_bytes(total_size));
    println!("   • Output directory: {}", output_dir);

    Ok(())
}

fn format_bytes(bytes: usize) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}
