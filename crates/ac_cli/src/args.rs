//! The command line.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

/// Runs API requests of a collection, e.g. in CI.
#[derive(Debug, Parser)]
#[command(name = "api-client-cli", version, about)]
pub struct Args {
    /// What to do.
    #[command(subcommand)]
    pub command: Command,
}

/// Subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run the requests of a collection one after another.
    Run(RunArgs),
}

/// Arguments of `run`.
#[derive(Debug, Clone, PartialEq, Eq, clap::Args)]
pub struct RunArgs {
    /// The collection folder.
    pub collection: PathBuf,
    /// Name of the environment to use.
    #[arg(short, long)]
    pub env: Option<String>,
    /// Only the folder or request at this path of names, e.g. `Auth/Login`.
    #[arg(long)]
    pub only: Option<String>,
    /// How to report.
    #[arg(long, value_enum, default_value_t = Format::Text)]
    pub format: Format,
    /// A status of 400 or more does not count as failed.
    #[arg(long)]
    pub allow_error_status: bool,
}

/// Report formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Format {
    /// For people.
    Text,
    /// For programs.
    Json,
    /// `JUnit` XML, read by most CI systems.
    Junit,
}
