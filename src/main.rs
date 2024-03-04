use std::{io::Write, path::Path};

use clap::{Parser, Subcommand};
use git2::Repository;
use termimad::MadSkin;
use toml::to_string;
use types::Config;

pub mod types;

#[derive(Parser, Debug)]
#[command(author = "RMHedge", version = "0.1.0")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    #[command(name = "init", about = "Initialize .vendman directory")]
    Init,
    #[command(name = "vend", about = "Clone a repo and add it to the config")]
    Vend {
        #[arg(short = 'r', help = "GitHub repo to clone")]
        repo: String,

        #[arg(short = 'b', help = "Branch to checkout")]
        branch: Option<String>,
    },
    #[command(name = "clean", about = "Remove .vendman directory")]
    Clean,
    #[command(name = "update", about = "Update dependencies")]
    Update,
    #[command(name = "ls", about = "List current versions of dependencies")]
    List,
}

fn main() {
    let args = Args::parse();

    let mut erroneous = MadSkin::default();
    erroneous
        .bold
        .set_fg(termimad::crossterm::style::Color::Red);

    let error = |text: &str| {
        eprint!("{}", erroneous.inline(&format!("**Error**: {}", text)));
        std::process::exit(1);
    };

    let output = process(args).map_err(|e| error(&e)).unwrap();
    println!("{}", output);
}

fn process(args: Args) -> Result<String, String> {
    let home = Path::new(&dirs::home_dir().expect("Can't find home directory")).join(".vendman");
    let config = home.join("config.toml");

    let enforce_config = || -> Result<Config, String> {
        if !config.exists() {
            return Err(
                "No .vendman directory found. Run `vendman init` to initialize it.".to_string(),
            );
        }

        toml::from_str(
            &std::fs::read_to_string(&config).map_err(|_| "Can't read config file".to_string())?,
        )
        .map_err(|_| "Can't parse config file".to_string())
    };

    match args.command {
        Command::Init => {
            if !home.exists() {
                std::fs::create_dir(&home).map_err(|_| "Can't create .vendman directory")?;
                std::fs::File::options()
                    .create(true)
                    .write(true)
                    .open(&config)
                    .map_err(|_| "Can't create config file")?
                    .write_all(
                        to_string(&types::Config {
                            version: "0.1.0".to_string(),
                            dependencies: Default::default(),
                        })
                        .unwrap()
                        .as_bytes(),
                    )
                    .map_err(|_| "Can't write to config file")?;
            }

            return Ok(termimad::inline("**.vendman directory initialized**").to_string());
        }
        Command::Vend { repo, branch } => {
            let mut config_file = enforce_config()?;
            let name = repo.split('/').last().unwrap();

            let repo = Repository::clone(&repo, home.join(name))
                .map_err(|e| format!("Can't clone repository: {e:?}"))?;

            let dep = match branch {
                Some(branch) => types::Dependency::DepWithHash(
                    repo.path().to_str().unwrap().to_string(),
                    branch,
                ),
                None => types::Dependency::Dep(repo.path().to_str().unwrap().to_string()),
            };

            config_file.dependencies.insert(name.to_string(), dep);
            std::fs::write(config, to_string(&config_file).unwrap())
                .map_err(|_| "Can't write to config file")?;

            return Ok(termimad::inline(&format!("**Cloned**: *{}*", name)).to_string());
        }
        Command::Clean => {
            enforce_config()?;
            std::fs::remove_dir_all(&home).map_err(|_| "Can't remove .vendman directory")?;
            return Ok(termimad::inline("**.vendman directory removed**").to_string());
        }
        Command::Update => {
            let config_file = enforce_config()?;
            let mut updated = Vec::<String>::new();

            for (name, dep) in config_file.dependencies {
                match dep {
                    types::Dependency::Dep(_) => {
                        Repository::open(home.join(name.clone()))
                            .map_err(|_| format!("[{name}] Can't open repository"))?
                            .find_remote("origin")
                            .map_err(|_| format!("[{name}] Can't find remote"))?
                            .fetch(&["origin"], None, None)
                            .map_err(|_| format!("[{name}] Can't fetch repo"))?;

                        updated.push(format!("{name}"));
                    }
                    types::Dependency::DepWithHash(_, branch) => {
                        Repository::open(home.join(name.clone()))
                            .map_err(|_| format!("[{name}] Can't open repository"))?
                            .find_remote("origin")
                            .map_err(|_| format!("[{name}] Can't find remote"))?
                            .fetch(&["origin"], None, None)
                            .map_err(|_| format!("[{name}] Can't fetch repo"))?;

                        Repository::open(home.join(name.clone()))
                            .map_err(|_| format!("[{name}] Can't open repository"))?
                            .checkout_head(None)
                            .map_err(|_| format!("[{name}] Can't checkout branch"))?;

                        updated.push(format!("{name}/*{branch}*"));
                    }
                }
            }

            return Ok(termimad::inline("**Dependencies updated**").to_string());
        }
        Command::List => {
            let config_file = enforce_config()?;
            let mut table = Vec::<(String, String, String)>::new();

            for (name, _) in config_file.dependencies {
                let repo = Repository::open(home.join(name.clone()))
                    .map_err(|_| "Can't open repository")?;
                let head = repo.head().map_err(|_| "Can't get head")?;
                let commit = head.peel_to_commit().map_err(|_| "Can't peel to commit")?;

                let branch = head.shorthand().unwrap_or("HEAD").to_string();
                let hash = commit.id().to_string();

                table.push((name, branch, hash));
            }

            let mut table = table.iter().collect::<Vec<_>>();
            table.sort_by(|a, b| a.0.cmp(&b.0));

            termimad::print_text(&format!(
                "\n|**Name**|**Branch**|**Hash**|\n|:-:|:-:|:-:|\n{}\n",
                table
                    .iter()
                    .map(|(name, branch, hash)| format!(
                        "| **{}** | {} | *{}* |",
                        name, branch, hash
                    ))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));

            Ok(String::new())
        }
    }
}
