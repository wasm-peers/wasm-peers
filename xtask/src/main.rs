#![allow(clippy::cargo_common_metadata)]

use std::process;

use clap::{Parser, Subcommand};
use color_eyre::Result;
use xshell::{cmd, Shell};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Fmt,
    Check,
    Clippy,
    Run,
    Test,
    Doc,
    PreCommit,
    PublishDocker { tag: String },
    PublishAws { tag: String },
}

const AWS_PUBLIC_ECR_ACCOUNT_URI: &str = "public.ecr.aws/2j7p7g8d";

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();
    let sh = Shell::new()?;

    match &cli.command {
        Command::Fmt => fmt(&sh)?,
        Command::Check => check(&sh)?,
        Command::Clippy => clippy(&sh)?,
        Command::Run => run(&sh)?,
        Command::Test => test(&sh)?,
        Command::Doc => doc(&sh)?,
        Command::PreCommit => pre_commit(&sh)?,
        Command::PublishDocker { tag } => publish_docker(&sh, tag)?,
        Command::PublishAws { tag } => publish_aws(&sh, tag)?,
    };

    Ok(())
}

fn fmt(sh: &Shell) -> Result<()> {
    Ok(cmd!(sh, "cargo +nightly fmt").run()?)
}

fn check(sh: &Shell) -> Result<()> {
    Ok(cmd!(sh, "cargo check --all-targets --all-features --workspace").run()?)
}

fn clippy(sh: &Shell) -> Result<()> {
    Ok(cmd!(sh, "cargo clippy --all-targets --all-features --workspace").run()?)
}

fn run(sh: &Shell) -> Result<()> {
    Ok(cmd!(sh, "cargo run --package wasm-peers-signaling-server").run()?)
}

fn test(sh: &Shell) -> Result<()> {
    cmd!(sh, "cargo build --package wasm-peers-signaling-server").run()?;
    let mut server = process::Command::new("./target/debug/wasm-peers-signaling-server")
        .current_dir(project_root::get_project_root()?)
        .spawn()?;

    let result = || -> Result<()> {
        let current_dir = sh.current_dir();
        sh.change_dir(project_root::get_project_root()?.join("library/"));
        cmd!(sh, "wasm-pack test --headless --firefox").run()?;
        cmd!(sh, "wasm-pack test --headless --chrome").run()?;
        sh.change_dir(current_dir);
        Ok(())
    }();

    server.kill()?;

    result
}

fn doc(sh: &Shell) -> Result<()> {
    Ok(cmd!(sh, "cargo doc --no-deps --all-features").run()?)
}

fn pre_commit(sh: &Shell) -> Result<()> {
    for cmd in [fmt, check, test, doc] {
        cmd(sh)?;
    }
    Ok(())
}

fn publish_docker(sh: &Shell, tag: &str) -> Result<()> {
    cmd!(sh, "docker login").run()?;
    cmd!(sh, "docker build -t wasm-peers/signaling-server .").run()?;
    cmd!(
        sh,
        "docker tag wasm-peers/signaling-server tomkarw/wasm-peers-signaling-server:{tag}"
    )
    .run()?;
    cmd!(
        sh,
        "docker tag wasm-peers/signaling-server tomkarw/wasm-peers-signaling-server:latest"
    )
    .run()?;
    cmd!(sh, "docker push tomkarw/wasm-peers-signaling-server:{tag}").run()?;
    cmd!(sh, "docker push tomkarw/wasm-peers-signaling-server:latest").run()?;
    Ok(())
}

fn publish_aws(sh: &Shell, tag: &str) -> Result<()> {
    #[rustfmt::skip]
    cmd!(
        sh,
        "aws ecr-public get-login-password --region us-east-1 | docker login --username AWS --password-stdin {AWS_PUBLIC_ECR_ACCOUNT_URI}"
    )
    .run()?;
    cmd!(sh, "docker build -t wasm-peers/signaling-server .").run()?;
    #[rustfmt::skip]
    cmd!(
        sh,
        "docker tag wasm-peers/signaling-server {AWS_PUBLIC_ECR_ACCOUNT_URI}/wasm-peers/signaling-server:{tag}"
    )
    .run()?;
    #[rustfmt::skip]
    cmd!(
        sh,
        "docker tag wasm-peers/signaling-server {AWS_PUBLIC_ECR_ACCOUNT_URI}/wasm-peers/signaling-server:latest"
    )
    .run()?;
    #[rustfmt::skip]
    cmd!(
        sh,
        "docker push {AWS_PUBLIC_ECR_ACCOUNT_URI}/wasm-peers/signaling-server:{tag}"
    )
    .run()?;
    cmd!(
        sh,
        "docker push {AWS_PUBLIC_ECR_ACCOUNT_URI}/wasm-peers/signaling-server:latest"
    )
    .run()?;
    Ok(())
}
