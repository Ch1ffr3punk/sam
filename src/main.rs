mod merge;
mod split;
use anyhow::{bail, Context, Result};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "sam", about = "Split And Merge files (c) 2026 Ch1ffr3punk")]
struct Cli {
    #[arg(short = 's', long = "split", help = "Split file into chunks of X MiB")]
    split_size_mib: Option<u64>,
    
    #[arg(short = 'm', long = "merge", num_args = 1..)]
    merge_files: Vec<String>,
    
    #[arg(short = 'o', long = "out", default_value = ".")]
    output_path: String,
    
    #[arg(value_name = "FILE")]
    file: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if !cli.merge_files.is_empty() {
        return merge::merge_files(&cli.merge_files, &cli.output_path);
    }

    if let Some(size_mib) = cli.split_size_mib {
        if size_mib == 0 {
            bail!("Split size must be greater than 0");
        }
        let file_path = cli
            .file
            .as_ref()
            .context("File must be specified for splitting")?;
        return split::split_file(file_path, size_mib, &cli.output_path);
    }

    bail!("Either -s <MiB> or -m <part> must be specified");
}
