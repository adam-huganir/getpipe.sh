use anyhow::Result;
use clap::{Parser, Subcommand};
use std::process::Command;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Development tasks for termlib-server")]
struct Cli {
  #[command(subcommand)]
  command: Commands,
}

#[derive(Subcommand)]
enum Commands {
  /// Run cargo fmt on the entire workspace
  Format,
  /// Run clippy lints on the entire workspace
  Lint,
  /// Run all tests
  Test,
  /// Build the project in release mode
  Build,
  /// Run the development server
  Dev,
  /// Clean build artifacts
  Clean,
  /// Run a full CI check (format, lint, test, build)
  Ci,
}

fn main() -> Result<()> {
  let cli = Cli::parse();

  match cli.command {
    Commands::Format => format()?,
    Commands::Lint => lint()?,
    Commands::Test => test()?,
    Commands::Build => build()?,
    Commands::Dev => dev()?,
    Commands::Clean => clean()?,
    Commands::Ci => ci()?,
  }

  Ok(())
}

fn format() -> Result<()> {
  println!("🎨 Formatting code...");
  run_command("cargo", &["fmt", "--all"])?;
  println!("✅ Code formatted successfully");
  Ok(())
}

fn lint() -> Result<()> {
  println!("🔍 Running clippy lints...");
  run_command(
    "cargo",
    &[
      "clippy",
      "--all-targets",
      "--all-features",
      "--",
      "-D",
      "warnings",
    ],
  )?;
  println!("✅ Lints passed successfully");
  Ok(())
}

fn test() -> Result<()> {
  println!("🧪 Running tests...");
  run_command("cargo", &["test", "--all"])?;
  println!("✅ Tests passed successfully");
  Ok(())
}

fn build() -> Result<()> {
  println!("🔨 Building project in release mode...");
  run_command("cargo", &["build", "--release"])?;
  println!("✅ Build completed successfully");
  Ok(())
}

fn dev() -> Result<()> {
  println!("🚀 Starting development server...");
  std::env::set_var("LOG_LEVEL", "debug");
  std::env::set_var("LOG_REQUESTS", "true");
  run_command("cargo", &["run"])?;
  Ok(())
}

fn clean() -> Result<()> {
  println!("🧹 Cleaning build artifacts...");
  run_command("cargo", &["clean"])?;
  println!("✅ Clean completed successfully");
  Ok(())
}

fn ci() -> Result<()> {
  println!("🔄 Running full CI pipeline...");

  println!("\n1/4 Formatting...");
  format()?;

  println!("\n2/4 Linting...");
  lint()?;

  println!("\n3/4 Testing...");
  test()?;

  println!("\n4/4 Building...");
  build()?;

  println!("\n🎉 CI pipeline completed successfully!");
  Ok(())
}

fn run_command(cmd: &str, args: &[&str]) -> Result<()> {
  let status = Command::new(cmd).args(args).status()?;

  if !status.success() {
    anyhow::bail!("Command failed: {} {}", cmd, args.join(" "));
  }

  Ok(())
}
