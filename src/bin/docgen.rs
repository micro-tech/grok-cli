use clap::CommandFactory;
use clap_markdown::MarkdownOptions;
use grok_cli::cli::app::Cli;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from("docs");
    if !out_dir.exists() {
        fs::create_dir_all(&out_dir).expect("Failed to create docs directory");
    }

    let markdown = clap_markdown::help_markdown::<Cli>();
    let out_path = out_dir.join("CLI_REFERENCE.md");

    fs::write(&out_path, markdown).expect("Failed to write CLI_REFERENCE.md");
    println!("Generated CLI documentation at {:?}", out_path);
}
