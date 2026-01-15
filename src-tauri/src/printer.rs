use crate::protocol::PrinterInfo;
use base64::{engine::general_purpose::STANDARD, Engine};
use std::io::Write;
use std::process::Command;

/// Get list of available printers
pub fn get_printers() -> Vec<PrinterInfo> {
    let system_printers = printers::get_printers();

    system_printers
        .into_iter()
        .map(|p| {
            // Detect if it's likely a thermal printer based on name
            let printer_type = if is_thermal_printer(&p.name) {
                "thermal"
            } else {
                "standard"
            };

            // Use system_name for CUPS compatibility, fall back to name
            let printer_id = p.system_name.clone();

            log::debug!("Found printer: name='{}', system_name='{}', is_default={}",
                p.name, p.system_name, p.is_default);

            PrinterInfo {
                id: printer_id,
                name: p.name.clone(),
                printer_type: printer_type.to_string(),
                status: "ready".to_string(),
                is_default: p.is_default,
            }
        })
        .collect()
}

/// Check if a printer is likely a thermal label printer based on its name
fn is_thermal_printer(name: &str) -> bool {
    let thermal_keywords = [
        "rollo", "dymo", "zebra", "brother ql", "thermal",
        "label", "4x6", "shipping", "stamps.com"
    ];
    let name_lower = name.to_lowercase();
    thermal_keywords.iter().any(|kw| name_lower.contains(kw))
}

/// Create a safe ID from printer name
fn sanitize_printer_id(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Find a printer by ID (system_name)
pub fn find_printer(printer_id: &str) -> Option<String> {
    let printers = printers::get_printers();
    log::debug!("Looking for printer with id: {}", printer_id);
    printers
        .into_iter()
        .find(|p| p.system_name == printer_id)
        .map(|p| {
            log::debug!("Found printer: system_name='{}', name='{}'", p.system_name, p.name);
            p.system_name // Return system_name for CUPS
        })
}

/// Print a label to the specified printer (supports PDF and PNG)
pub fn print_label(printer_name: &str, data_base64: &str, format: &str, copies: u32) -> Result<(), String> {
    log::info!("Printing {} to '{}' ({} copies)", format, printer_name, copies);

    // Decode base64 data
    let data = STANDARD
        .decode(data_base64)
        .map_err(|e| format!("Failed to decode {}: {}", format, e))?;

    log::debug!("Decoded {}: {} bytes", format, data.len());

    // Determine file extension based on format
    let extension = match format.to_lowercase().as_str() {
        "png" => "png",
        "pdf" => "pdf",
        "jpg" | "jpeg" => "jpg",
        _ => "pdf", // Default to PDF
    };

    // Write to temp file
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(format!("limestack_label_{}.{}", std::process::id(), extension));

    log::debug!("Writing to temp file: {:?}", temp_path);

    let mut file = std::fs::File::create(&temp_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    file.write_all(&data)
        .map_err(|e| format!("Failed to write label: {}", e))?;

    // Ensure file is flushed and closed before printing
    drop(file);

    // Verify file exists
    if !temp_path.exists() {
        return Err("Temp file was not created".to_string());
    }

    // Print using OS-specific command
    let result = print_file(&temp_path, printer_name, copies);

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_path);

    result
}

#[cfg(target_os = "macos")]
fn print_file(path: &std::path::Path, printer_name: &str, copies: u32) -> Result<(), String> {
    log::info!("Running: lpr -P '{}' -# {} -o fit-to-page {:?}", printer_name, copies, path);

    let output = Command::new("lpr")
        .arg("-P")
        .arg(printer_name)
        .arg("-#")
        .arg(copies.to_string())
        .arg("-o")
        .arg("fit-to-page")
        .arg(path)
        .output()
        .map_err(|e| format!("Failed to execute lpr: {}", e))?;

    if output.status.success() {
        log::info!("Print job submitted successfully");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::error!("lpr failed: {}", stderr);
        Err(format!("lpr failed: {}", stderr))
    }
}

#[cfg(target_os = "windows")]
fn print_file(path: &std::path::Path, printer_name: &str, copies: u32) -> Result<(), String> {
    // Use SumatraPDF for silent printing if available, otherwise use default PDF handler
    let sumatra_paths = [
        r"C:\Program Files\SumatraPDF\SumatraPDF.exe",
        r"C:\Program Files (x86)\SumatraPDF\SumatraPDF.exe",
    ];

    for sumatra_path in sumatra_paths {
        if std::path::Path::new(sumatra_path).exists() {
            let output = Command::new(sumatra_path)
                .arg("-print-to")
                .arg(printer_name)
                .arg("-print-settings")
                .arg(format!("{}x", copies))
                .arg("-silent")
                .arg(path)
                .output()
                .map_err(|e| format!("Failed to execute SumatraPDF: {}", e))?;

            if output.status.success() {
                return Ok(());
            }
        }
    }

    // Fallback: use Windows print verb
    let path_str = path.to_string_lossy();
    let output = Command::new("cmd")
        .args(["/C", "start", "/wait", "", "/min", "mshta"])
        .arg(format!(
            "javascript:var ws=new ActiveXObject('WScript.Shell');ws.Run('print /d:\"{}\" \"{}\"',0,true);close();",
            printer_name, path_str
        ))
        .output()
        .map_err(|e| format!("Failed to print: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        // Try another fallback
        Command::new("rundll32")
            .arg("mshtml.dll,PrintHTML")
            .arg(path)
            .output()
            .map_err(|e| format!("Failed to print: {}", e))?;
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn print_file(path: &std::path::Path, printer_name: &str, copies: u32) -> Result<(), String> {
    let output = Command::new("lp")
        .arg("-d")
        .arg(printer_name)
        .arg("-n")
        .arg(copies.to_string())
        .arg(path)
        .output()
        .map_err(|e| format!("Failed to execute lp: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "lp failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}
