use anyhow::{bail, Context, Result};
use ripemd::{Digest, Ripemd160};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

fn hash_to_hex(hash: &[u8; 20]) -> String {
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

fn compute_file_hash(path: &Path) -> Result<([u8; 20], u64)> {
    let mut file = File::open(path)
        .with_context(|| format!("Cannot open file for hashing: {}", path.display()))?;
    let mut hasher = Ripemd160::new();
    let mut buffer = vec![0u8; 64 * 1024];
    let mut total_size: u64 = 0;
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
        total_size += n as u64;
    }
    let result = hasher.finalize();
    let mut hash = [0u8; 20];
    hash.copy_from_slice(&result);
    Ok((hash, total_size))
}

fn write_hash_report(
    original_name: &str,
    original_size: u64,
    original_hash: &[u8; 20],
    parts: &[(String, [u8; 20], u64)],
    operation: &str,
) -> Result<()> {
    let report_path = PathBuf::from("ripemd-160.txt");
    let mut file = File::create(&report_path).with_context(|| {
        format!("Cannot create hash report: {}", report_path.display())
    })?;
    writeln!(file, "RIPEMD-160 Hash Report")?;
    writeln!(file, "======================")?;
    writeln!(file, "Operation: {}", operation)?;
    writeln!(file)?;
    writeln!(file, "Merged file: {}", original_name)?;
    writeln!(file, "Merged size: {} bytes", original_size)?;
    writeln!(file, "Merged hash: {}", hash_to_hex(original_hash))?;
    writeln!(file)?;
    writeln!(file, "Parts ({}):", parts.len())?;
    writeln!(file, "----------")?;
    for (name, hash, size) in parts {
        writeln!(file, "{} ({} bytes): {}", name, size, hash_to_hex(hash))?;
    }
    Ok(())
}

// Erkennt das neue Format: <name>.part<NNN><ext>
fn parse_part_filename(file_name: &str) -> Option<(String, u32)> {
    let part_pos = file_name.rfind(".part")?;
    let prefix = &file_name[..part_pos];
    let rest = &file_name[part_pos + 5..]; // Überspringt ".part"
    
    if rest.len() < 3 { return None; }
    
    let (index_str, ext) = rest.split_at(3);
    let index: u32 = index_str.parse().ok()?;
    
    let original_name = format!("{}{}", prefix, ext);
    Some((original_name, index))
}

fn effective_parent(path: &Path) -> &Path {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}

pub fn merge_files(parts: &[String], output_path: &str) -> Result<()> {
    if parts.is_empty() {
        bail!("No part files specified");
    }

    let mut all_parts: Vec<String> = Vec::new();

    if parts.len() == 1 {
        let single_path = &parts[0];
        let path = Path::new(single_path);
        let parent = effective_parent(path);
        let file_name = path
            .file_name()
            .context("Invalid file path")?
            .to_string_lossy()
            .to_string();

        if let Some((original_name, _)) = parse_part_filename(&file_name) {
            println!("Auto-discovering parts in {:?}...", parent);
            for entry in fs::read_dir(parent)? {
                let entry = entry?;
                let entry_name = entry.file_name().to_string_lossy().to_string();
                if let Some((oname, _)) = parse_part_filename(&entry_name) {
                    if oname == original_name {
                        all_parts.push(entry.path().to_string_lossy().to_string());
                    }
                }
            }
            if all_parts.is_empty() {
                bail!("No matching parts found for: {}", file_name);
            }
            println!("Found {} parts", all_parts.len());
        } else {
            all_parts.push(single_path.clone());
        }
    } else {
        all_parts.extend(parts.iter().cloned());
    }

    let mut indexed: Vec<(u32, String)> = Vec::new();
    for p in &all_parts {
        let path = Path::new(p);
        let file_name = path
            .file_name()
            .context("Invalid file path")?
            .to_string_lossy()
            .to_string();
            
        let (_original_name, index) = parse_part_filename(&file_name)
            .with_context(|| {
                format!(
                    "Cannot parse part info from '{}'. Expected pattern: <name>.part<NNN><ext>",
                    p
                )
            })?;
        indexed.push((index, p.clone()));
    }

    indexed.sort_by_key(|(idx, _)| *idx);

    // Prüfen, ob die Teile lückenlos von 1 bis N nummeriert sind
    for (expected_zero_based, (idx, p)) in indexed.iter().enumerate() {
        let expected_idx = (expected_zero_based + 1) as u32;
        if *idx != expected_idx {
            bail!("Missing part {} (found {} at '{}')", expected_idx, idx, p);
        }
    }

    let first_path = Path::new(&indexed[0].1);
    let first_file_name = first_path
        .file_name()
        .context("Invalid file path")?
        .to_string_lossy()
        .to_string();
        
    let (original_name, _) = parse_part_filename(&first_file_name)
        .context("Cannot derive original filename")?;

    let output_dir = PathBuf::from(output_path);
    fs::create_dir_all(&output_dir).with_context(|| {
        format!("Cannot create output directory: {}", output_dir.display())
    })?;

    let output_file_path = output_dir.join(&original_name);
    println!("Output file: {}", output_file_path.display());
    println!("Will join {} parts", indexed.len());
    println!("Computing hashes of parts...");

    let mut parts_info: Vec<(String, [u8; 20], u64)> = Vec::new();
    for (_i, (_, part_path)) in indexed.iter().enumerate() {
        let (hash, size) = compute_file_hash(Path::new(part_path))?;
        let part_name = Path::new(part_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        println!("  {} [{}]", part_name, hash_to_hex(&hash));
        parts_info.push((part_name, hash, size));
    }

    let mut out_file = File::create(&output_file_path).with_context(|| {
        format!("Failed to create output file: {}", output_file_path.display())
    })?;
    
    let mut merged_hasher = Ripemd160::new();
    let mut merged_size: u64 = 0;
    let mut buffer = vec![0u8; 64 * 1024];

    for (_i, (_, part_path)) in indexed.iter().enumerate() {
        let mut part_file = File::open(part_path)
            .with_context(|| format!("Failed to open part: {}", part_path))?;
        loop {
            let n = part_file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            out_file.write_all(&buffer[..n])?;
            merged_hasher.update(&buffer[..n]);
            merged_size += n as u64;
        }
        out_file.flush()?;
        println!("Processing file: {}", part_path);
    }

    let merged_result = merged_hasher.finalize();
    let mut merged_hash = [0u8; 20];
    merged_hash.copy_from_slice(&merged_result);

    write_hash_report(
        &original_name,
        merged_size,
        &merged_hash,
        &parts_info,
        "merge",
    )?;

    println!("Combined successfully!");
    println!("Merged hash: {}", hash_to_hex(&merged_hash));
    println!(
        "Hash report written to: {}",
        std::env::current_dir()?.join("ripemd-160.txt").display()
    );
    Ok(())
}
