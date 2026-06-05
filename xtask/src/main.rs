use anyhow::Result;
use clap::{Parser, Subcommand};
use std::process::Command;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Development tasks for getpipe.sh")]
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
    &["clippy", "--all-targets", "--", "-D", "warnings"],
  )?;
  println!("✅ Lints passed successfully");
  Ok(())
}

fn test() -> Result<()> {
  println!("🧪 Running tests...");
  run_command("cargo", &["test", "--workspace"])?;
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
  println!("🚀 Starting development server (hot-reload enabled)...");
  let secrets_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .join("secrets");
  let read_secret = |name: &str| -> Option<String> {
    std::fs::read_to_string(secrets_dir.join(name))
      .ok()
      .map(|s| s.trim().to_string())
  };
  let app_id = read_secret("GITHUB_APP_ID");
  let installation_id = read_secret("GITHUB_APP_INSTALLATION_ID");
  let app_key = read_secret("GITHUB_APP_KEY");

  let mut env_vars: Vec<(&str, String)> = vec![
    ("LOG_LEVEL", "debug".to_string()),
    ("LOG_REQUESTS", "true".to_string()),
  ];
  if let (Some(id), Some(inst), Some(key)) = (app_id, installation_id, app_key) {
    env_vars.push(("GITHUB_APP_ID", id));
    env_vars.push(("GITHUB_APP_INSTALLATION_ID", inst));
    env_vars.push(("GITHUB_APP_KEY", key));
  }

  let env_refs: Vec<(&str, &str)> = env_vars.iter().map(|(k, v)| (*k, v.as_str())).collect();
  run_command_with_env("cargo", &["run", "--features", "hot-reload"], &env_refs)?;
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
  run_command_with_env(cmd, args, &[])
}

fn run_command_with_env(cmd: &str, args: &[&str], env: &[(&str, &str)]) -> Result<()> {
  let mut command = Command::new(cmd);
  command.args(args);
  for (key, val) in env {
    command.env(key, val);
  }
  let status = command.status()?;

  if !status.success() {
    anyhow::bail!(
      "Command failed (exit {}): {} {}",
      status.code().unwrap_or(-1),
      cmd,
      args.join(" ")
    );
  }

  Ok(())
}
