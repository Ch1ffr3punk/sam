use anyhow::{Context, Result};
use ripemd::{Digest, Ripemd160};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

fn hash_to_hex(hash: &[u8; 20]) -> String {
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

fn compute_file_hash(path: &Path) -> Result<[u8; 20]> {
    let mut file = File::open(path)
        .with_context(|| format!("Cannot open file for hashing: {}", path.display()))?;
    let mut hasher = Ripemd160::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    let result = hasher.finalize();
    let mut hash = [0u8; 20];
    hash.copy_from_slice(&result);
    Ok(hash)
}

fn compute_data_hash(data: &[u8]) -> [u8; 20] {
    let mut hasher = Ripemd160::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 20];
    hash.copy_from_slice(&result);
    hash
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
    writeln!(file, "Original file: {}", original_name)?;
    writeln!(file, "Original size: {} bytes", original_size)?;
    writeln!(file, "Original hash: {}", hash_to_hex(original_hash))?;
    writeln!(file)?;
    writeln!(file, "Parts ({}):", parts.len())?;
    writeln!(file, "----------")?;
    for (name, hash, size) in parts {
        writeln!(file, "{} ({} bytes): {}", name, size, hash_to_hex(hash))?;
    }

    Ok(())
}

pub fn split_file(file_path: &str, num_parts: u32, output_path: &str) -> Result<()> {
    let metadata = std::fs::metadata(file_path)
        .with_context(|| format!("Cannot stat file: {}", file_path))?;
    let file_size = metadata.len();
    let part_size = (file_size + num_parts as u64 - 1) / num_parts as u64;

    let path = Path::new(file_path);
    let base_name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let name_without_ext = base_name.trim_end_matches(&ext);

    let output_dir = PathBuf::from(output_path);
    fs::create_dir_all(&output_dir).with_context(|| {
        format!("Cannot create output directory: {}", output_dir.display())
    })?;

    println!(
        "Splitting {} into {} parts of up to {} bytes each",
        file_path, num_parts, part_size
    );
    println!("Output directory: {}", output_dir.display());

    print!("Computing hash of original file... ");
    std::io::Write::flush(&mut std::io::stdout()).ok();
    let original_hash = compute_file_hash(path)?;
    println!("{}", hash_to_hex(&original_hash));

    let mut source = File::open(file_path)
        .with_context(|| format!("Cannot open file: {}", file_path))?;
    let mut buffer = vec![0u8; part_size as usize];
    let mut parts_info: Vec<(String, [u8; 20], u64)> = Vec::new();

    for i in 1..=num_parts {
        let part_name = format!("{}_{}_{}{}", name_without_ext, i, num_parts, ext);
        let part_path = output_dir.join(&part_name);

        let mut bytes_read: usize = 0;
        while bytes_read < part_size as usize {
            let n = source.read(&mut buffer[bytes_read..])?;
            if n == 0 {
                break;
            }
            bytes_read += n;
        }
        if bytes_read == 0 {
            break;
        }

        let part_hash = compute_data_hash(&buffer[..bytes_read]);
        parts_info.push((part_name.clone(), part_hash, bytes_read as u64));

        let mut part_file = File::create(&part_path)
            .with_context(|| format!("Cannot create part file: {}", part_path.display()))?;
        part_file.write_all(&buffer[..bytes_read])?;

        println!(
            "Created file: {} ({} bytes) [{}]",
            part_path.display(),
            bytes_read,
            hash_to_hex(&part_hash)
        );
    }

    write_hash_report(
        &base_name,
        file_size,
        &original_hash,
        &parts_info,
        "split",
    )?;
    println!(
        "Hash report written to: {}",
        std::env::current_dir()?.join("ripemd-160.txt").display()
    );
    println!("Splitted successfully!");
    Ok(())
}
