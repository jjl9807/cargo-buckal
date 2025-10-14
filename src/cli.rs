use clap::Parser;

use crate::build_version;

#[derive(Parser, Debug)]
#[command(name = "buckal", version = build_version(), about = "A cargo plugin for Buck2", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Parser, Debug)]
pub enum Commands {
    Buckal(BuckalArgs),
}

#[derive(Parser, Debug)]
pub struct BuckalArgs {
    /// Use verbose output
    #[command(subcommand)]
    pub subcommands: BuckalSubCommands,
}

#[derive(Parser, Debug)]
pub enum BuckalSubCommands {
    /// Compile the current package
    Build(crate::commands::build::BuildArgs),

    /// Create a new package in an existing directory
    Init(crate::commands::init::InitArgs),

    /// Create a new package
    New(crate::commands::new::NewArgs),

    /// Clean up the buck-out directory
    Clean(crate::commands::clean::CleanArgs),

    /// Add dependencies to a manifest file
    Add(crate::commands::add::AddArgs),

    /// Migrate existing Cargo packages to Buck2
    Migrate(crate::commands::migrate::MigrateArgs),

    /// Update dependencies in a manifest file
    Update(crate::commands::update::UpdateArgs),

    /// Remove dependencies from a manifest file
    Remove(crate::commands::remove::RemoveArgs),

    /// Print version information
    Version(crate::commands::version::VersionArgs),
}

impl Cli {
    pub fn run(&self) {
        match &self.command {
            Commands::Buckal(args) => match &args.subcommands {
                BuckalSubCommands::Build(args) => crate::commands::build::execute(args),
                BuckalSubCommands::Init(args) => crate::commands::init::execute(args),
                BuckalSubCommands::New(args) => crate::commands::new::execute(args),
                BuckalSubCommands::Clean(args) => crate::commands::clean::execute(args),
                BuckalSubCommands::Add(args) => crate::commands::add::execute(args),
                BuckalSubCommands::Migrate(args) => crate::commands::migrate::execute(args),
                BuckalSubCommands::Update(args) => crate::commands::update::execute(args),
                BuckalSubCommands::Remove(args) => crate::commands::remove::execute(args),
                BuckalSubCommands::Version(args) => crate::commands::version::execute(args),
            },
        }
    }
}
