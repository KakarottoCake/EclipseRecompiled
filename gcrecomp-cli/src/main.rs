// CLI application
mod commands;
mod output;

use clap::Parser;
use commands::{analyze_dol, build_dol, prepare_disc, recompile_dol};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "gcrecomp")]
#[command(about = "GameCube static recompiler")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Analyze a DOL file using Ghidra
    Analyze {
        /// Path to the DOL file
        #[arg(short, long)]
        dol_file: PathBuf,

        /// Use ReOxide backend (default: headless CLI)
        #[arg(long)]
        use_reoxide: bool,

        /// Optional `name=0xADDRESS` symbol map
        #[arg(long)]
        symbol_map: Option<PathBuf>,
    },
    /// Recompile a DOL file to Rust code
    Recompile {
        /// Path to the DOL file
        #[arg(short, long)]
        dol_file: PathBuf,

        /// Output directory for generated Rust code
        #[arg(short, long)]
        output_dir: Option<PathBuf>,

        /// Use ReOxide backend (default: headless CLI)
        #[arg(long)]
        use_reoxide: bool,

        /// Optional `name=0xADDRESS` symbol map
        #[arg(long)]
        symbol_map: Option<PathBuf>,
    },
    /// Full pipeline: analyze, recompile, and build
    Build {
        /// Path to the DOL file
        #[arg(short, long)]
        dol_file: PathBuf,

        /// Output directory for generated Rust code
        #[arg(short, long)]
        output_dir: Option<PathBuf>,

        /// Use ReOxide backend (default: headless CLI)
        #[arg(long)]
        use_reoxide: bool,

        /// Optional `name=0xADDRESS` symbol map
        #[arg(long)]
        symbol_map: Option<PathBuf>,
    },
    /// Extract the DOL and build a local disc-asset archive from an ISO
    Prepare {
        /// Legally dumped GameCube ISO or GCM image
        #[arg(short, long)]
        disc_image: PathBuf,

        /// Directory for private extracted files
        #[arg(short, long, default_value = "eclipse")]
        output_dir: PathBuf,

        /// Extract only main.dol and skip the disc asset archive
        #[arg(long)]
        no_assets: bool,
    },
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze {
            dol_file,
            use_reoxide,
            symbol_map,
        } => {
            let pb = create_progress_bar("Analyzing DOL file...");
            analyze_dol(&dol_file, symbol_map.as_deref(), use_reoxide)?;
            pb.finish_with_message("Analysis complete");
        }
        Commands::Recompile {
            dol_file,
            output_dir,
            use_reoxide,
            symbol_map,
        } => {
            let pb = create_progress_bar("Recompiling DOL file...");
            recompile_dol(
                &dol_file,
                output_dir.as_deref(),
                symbol_map.as_deref(),
                use_reoxide,
            )?;
            pb.finish_with_message("Recompilation complete");
        }
        Commands::Build {
            dol_file,
            output_dir,
            use_reoxide,
            symbol_map,
        } => {
            let pb = create_progress_bar("Building recompiled game...");
            build_dol(
                &dol_file,
                output_dir.as_deref(),
                symbol_map.as_deref(),
                use_reoxide,
            )?;
            pb.finish_with_message("Build complete");
        }
        Commands::Prepare {
            disc_image,
            output_dir,
            no_assets,
        } => {
            let pb = create_progress_bar("Preparing private game files...");
            prepare_disc(&disc_image, &output_dir, !no_assets)?;
            pb.finish_with_message("Game files prepared");
        }
    }

    Ok(())
}

fn create_progress_bar(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(message.to_string());
    pb
}
