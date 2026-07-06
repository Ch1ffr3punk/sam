mod merge;
mod split;
use anyhow::{bail, Context, Result};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "sam", about = "Split And Merge files")]
struct Cli {
    #[arg(short = 'p', long = "parts")]
    split_parts: Option<u32>,
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

    if let Some(num_parts) = cli.split_parts {
        if num_parts == 0 {
            bail!("Number of parts must be greater than 0");
        }
        let file_path = cli
            .file
            .as_ref()
            .context("File must be specified for splitting")?;
        return split::split_file(file_path, num_parts, &cli.output_path);
    }

    bail!("Either -p <parts> or -m <part> must be specified");
}
