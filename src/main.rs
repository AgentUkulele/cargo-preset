use std::path::PathBuf;
use std::fs;

use clap::{Subcommand, Parser, Args};
use fs_extra::dir::CopyOptions;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {

    #[arg(short, long, default_value_t = false)]
    debug: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {

    /// Use preset
    Apply { name: String },

    /// List available presets
    List,

    /// Add preset
    Add {
        /// Name of new preset
        name: String,

        #[command(flatten)]
        paths: AddEntry,
    },

    /// Remove preset
    Remove { name: String },

    /// View details of preset
    Inspect { name: String }
}

#[derive(Args)]
#[group(required = true)]
struct AddEntry {

    /// Files to have with preset
    #[arg(long)]
    files: Vec<PathBuf>,

    /// Directories to have with preset
    #[arg(long)]
    directories: Vec<PathBuf>,
}


fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let mut config = if let Some(path) = std::env::var_os("HOME") {
        let mut p: PathBuf = path.into();
        p.push(".config");
        if !p.exists() {
            anyhow::bail!("$HOME/.config directory not found");
        }
        p.push("cargo_preset");
        if !p.exists() {
            if args.debug {
                println!("Creating cargo_preset configuration directory");
            }
            let _ = fs::create_dir(&p)?;
        }
        p
    } else {
        anyhow::bail!("Home directory not found");
    };

    let current_presets: Vec<_> = config.read_dir()?
        .map(|dir| {
            dir.expect("Could not get directory entry")
                .file_name()
                .to_str()
                .expect("Could not convert filename into string")
                .to_owned()
        })
        .collect();


    match args.command {
        Command::Apply { name } => {
            config.push(&name);
            if !current_presets.contains(&name) {
                anyhow::bail!(format!("Could not find preset with name {name}"));
            }

            let curr_dir: PathBuf = std::str::from_utf8(
                &std::process::Command::new("sh")
                    .arg("-c")
                    .arg("pwd")
                    .output()?
                    .stdout
                )?
                .trim()
                .into();

            fs_extra::dir::copy(&config, curr_dir, &CopyOptions::new().copy_inside(true).content_only(true))?;
            config.pop();
        }
        Command::List => {
            println!("Available presets: ");
            for dir in current_presets {
                println!("\t{}", dir);
            }
        }
        Command::Add { name, paths } => {
            if current_presets.contains(&name) {
                anyhow::bail!(format!("Preset with name {name} already exists"));
            }

            config.push(name);
            fs::create_dir(&config)?;
            for file in paths.files {
                eprintln!("file: {file:#?}, config: {config:#?}");
                config.push(&file);
                let r = fs::copy(file, &config)?;
                eprintln!("wrote {r} bytes");
                config.pop();
            }

            let opts = CopyOptions::new()
                .copy_inside(true);

            for directory in paths.directories {
                fs_extra::dir::copy(directory, &config, &opts)?;
            }
        }
        Command::Remove { name } => {
            config.push(name);
            fs::remove_dir_all(config)?;
        }
        Command::Inspect { name } => {
            config.push(&name);
            println!("Contents of {name}: ");
            for dir in fs::read_dir(&config)? {
                let dir = dir.unwrap();
                if dir.metadata()?.is_dir() {
                    println!("- {}/", dir.file_name().to_str().unwrap());
                    print_dir(1, &dir.path())?;
                } else {
                    println!("- {}", dir.file_name().to_str().unwrap());
                }
            }
        }
    }

    Ok(())
}

fn print_dir(level: usize, path: &PathBuf) -> anyhow::Result<()> {
    for file in fs::read_dir(path)? {
        let file = file.unwrap();
        if file.metadata()?.is_dir() {
            println!("{} {}/", 
            "-".repeat(level),
            &file.file_name().to_str().unwrap()
        );
            print_dir(level + 1, &file.path())?;
        } else {
            println!(
            "{} {}",
            "-".repeat(level),
            file.file_name().to_str().unwrap()
        );
        }
    }

    Ok(())
}
