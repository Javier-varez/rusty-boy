use std::io::Write;
use std::path::{Path, PathBuf};

use clap::Parser;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use xshell::{cmd, Shell};

mod stack;

use stack::check_stack_sizes;

#[derive(Parser)]
struct BuildArgs {
    /// Build in release mode
    #[arg(short, long)]
    release: bool,
}

#[derive(Parser)]
enum Commands {
    Build(BuildArgs),
    Clean,
    Check,
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const NIGHTLY_TOOLCHAIN: &str = "nightly-2025-03-15";

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}

fn rusty_date_path() -> PathBuf {
    let mut path = project_root();
    path.push("rusty-date");
    path
}

fn rusty_date_elf_path(release: bool) -> PathBuf {
    let mut elf = rusty_date_path();
    elf.push("target");
    elf.push("thumbv7em-none-eabihf");
    elf.push(if release { "release" } else { "debug" });
    elf.push("rusty-date");
    elf
}

fn print_header(message: &str) -> Result<()> {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))?;
    write!(&mut stdout, "{}", message)?;
    stdout.reset()?;
    writeln!(&mut stdout)?;
    Ok(())
}

fn run_build_native(args: &BuildArgs) -> Result<()> {
    print_header("Native build")?;
    let sh = Shell::new()?;
    let mut cmd = cmd!(sh, "cargo build");
    if args.release {
        cmd = cmd.arg("--release");
    }
    cmd.run()?;

    Ok(())
}

fn run_build_playdate(args: &BuildArgs) -> Result<()> {
    print_header("Playdate build")?;

    let sh = Shell::new()?;

    let rusty_date_path = rusty_date_path();
    {
        let _dir = sh.push_dir(&rusty_date_path);

        let mut cmd = cmd!(sh, "cargo +{NIGHTLY_TOOLCHAIN} build");
        if args.release {
            cmd = cmd.arg("--release");
        }
        cmd.run()?;
    }

    let elf = rusty_date_elf_path(args.release);

    let mut pdxinfo = rusty_date_path.clone();
    pdxinfo.push("pdxinfo");

    let mut rusty_date_target = rusty_date_path.clone();
    rusty_date_target.push("build");
    rusty_date_target.push("rusty_date");

    let mut target_elf = rusty_date_target.clone();
    target_elf.push("pdex.elf");

    let mut target_pdxinfo = rusty_date_target.clone();
    target_pdxinfo.push("pdxinfo");

    cmd!(sh, "mkdir -p {rusty_date_target}").run()?;
    cmd!(sh, "cp {elf} {target_elf}").run()?;
    cmd!(sh, "cp {pdxinfo} {target_pdxinfo}").run()?;

    let _env_guard = std::env::var("PLAYDATE_SDK_PATH").ok().and_then(|sdk| {
        let path = std::env::var("PATH").ok()?;
        let mut p = String::new();
        p.push_str(&sdk);
        p.push_str("/bin:");
        p.push_str(&path);
        Some(sh.push_env("PATH", p))
    });

    cmd!(sh, "pdc {rusty_date_target}").run()?;

    Ok(())
}

fn run_build(args: &BuildArgs) -> Result<()> {
    run_build_native(args)?;
    run_build_playdate(args)?;

    Ok(())
}

fn run_clean_native() -> Result<()> {
    print_header("Native clean")?;
    let sh = Shell::new()?;
    cmd!(sh, "cargo clean").run()?;
    Ok(())
}

fn run_clean_playdate() -> Result<()> {
    print_header("Playdate clean")?;

    let sh = Shell::new()?;
    let rusty_date_path = rusty_date_path();
    {
        let _dir = sh.push_dir(&rusty_date_path);
        cmd!(sh, "cargo clean").run()?;
    }

    let mut build_dir = rusty_date_path.clone();
    build_dir.push("build");
    cmd!(sh, "rm -rf {build_dir}").run()?;
    Ok(())
}

fn run_clean() -> Result<()> {
    run_clean_native()?;
    run_clean_playdate()?;
    Ok(())
}

fn run_check() -> Result<()> {
    let release = true;
    run_build(&BuildArgs { release })?;
    let rusty_date = rusty_date_elf_path(release);

    let stack_sizes = check_stack_sizes(&rusty_date)?;

    const NUM_FUNCTIONS: usize = 10;

    let max_name_len = stack_sizes
        .iter()
        .take(NUM_FUNCTIONS)
        .map(|(name, _)| name.len())
        .max()
        .unwrap_or(0);
    for (name, size) in stack_sizes.iter().take(NUM_FUNCTIONS) {
        println!("{name:width$}: {size}", width = max_name_len + 2);
    }

    Ok(())
}

fn main() -> Result<()> {
    let command = Commands::parse();

    match command {
        Commands::Build(args) => {
            run_build(&args)?;
        }
        Commands::Clean => {
            run_clean()?;
        }
        Commands::Check => {
            run_check()?;
        }
    }

    Ok(())
}
