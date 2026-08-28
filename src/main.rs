use std::{collections::HashMap, path::PathBuf, sync::LazyLock};

use clap::{Args, Parser, Subcommand};
use ptip_ffi::{generate, initialize};

static DEFAULT_LIBRARY_ENTRY_POINTS: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| HashMap::from([("typescript", "index.ts"), ("python", "index.py"), ("cpp", "index.hpp")]));

#[derive(Parser)]
#[command(name = "ptip-ffi")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Generate(GenerateArgs),
}

#[derive(Args)]
struct GenerateArgs {
    #[arg(long, required = true)]
    library_root: PathBuf,
    #[arg(long, required = true)]
    library_language: String,
    #[arg(long)]
    library_entry_point: Option<PathBuf>,
    #[arg(long, required = true)]
    output_directory: PathBuf,
    #[arg(long, required = true)]
    output_language: String,
}

fn default_library_entry_point(language: &str) -> Option<&'static str> {
    let normalized = language.trim();
    DEFAULT_LIBRARY_ENTRY_POINTS
        .get(&normalized.to_ascii_lowercase()[..])
        .copied()
}

fn canonical_language_name(registry: &[ptip_ffi::LanguageConfig], language: &str) -> String {
    registry
        .iter()
        .find(|config| config.name.eq_ignore_ascii_case(language))
        .map(|config| config.name.to_string())
        .unwrap_or_else(|| language.to_string())
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate(args) => {
            let registry = initialize();
            let library_language = canonical_language_name(&registry, &args.library_language);
            let output_language = canonical_language_name(&registry, &args.output_language);
            let library_entry_point = match args.library_entry_point {
                Some(path) => path,
                None => match default_library_entry_point(&library_language) {
                    Some(entry) => args.library_root.join(entry),
                    None => {
                        eprintln!(
                            "No default library entry point configured for language '{}'. Please pass --library-entry-point explicitly.",
                            library_language
                        );
                        std::process::exit(1);
                    }
                },
            };

            let callee_output_root = args.output_directory.join("callee");
            let caller_output_root = args.output_directory.join("caller");

            if let Err(e) = generate(
                &registry,
                args.library_root.as_path(),
                library_entry_point.as_path(),
                &library_language,
                &output_language,
                callee_output_root.as_path(),
                caller_output_root.as_path(),
            ) {
                eprintln!("Error: {:?}", e);
                std::process::exit(1);
            }
        }
    }
}
