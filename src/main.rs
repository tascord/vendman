use std::{fmt::format, io::Write, path::Path};

use clap::{Parser, Subcommand};
use toml::to_string;

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
}

fn main() {
    let args = Args::parse();
    let home = Path::new(&dirs::home_dir().expect("Can't find home directory")).join(".vendman");
    let config = home.join("config.toml");

    let enforce_config = || {
        if !config.exists() {
            eprintln!("Run `vendman init` first");
            std::process::exit(1);
        }
    };

    match args.command {
        Command::Init => {
            if !home.exists() {
                std::fs::create_dir(&home).expect("Can't create .vendman directory");
                std::fs::File::options()
                    .create(true)
                    .write(true)
                    .open(&config)
                    .expect("Can't create config file")
                    .write_all(
                        to_string(&types::Config {
                            version: "0.1.0".to_string(),
                            dependencies: Default::default(),
                        })
                        .unwrap()
                        .as_bytes(),
                    )
                    .expect("Can't write to config file");
            }

            println!("Initialized .vendman directory");
        }
        Command::Vend { repo, branch } => {
            enforce_config();
            let mut c = std::process::Command::new("git");
            c.arg("clone")
                .arg(repo.clone())
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());

            if branch.is_some() {
                c.arg("-b").arg(branch.clone().unwrap().clone());
            }

            let name = repo.clone();
            let name = name.split('/').last().unwrap();
            c.current_dir(home).arg(format!("./{name}"));
            c.spawn().expect("Can't clone repo");
            println!("Cloned {}", name);

            let mut config_file: types::Config =
                toml::from_str(&std::fs::read_to_string(&config).expect("Can't read config file"))
                    .expect("Can't parse config file");

            match branch {
                Some(b) => {
                    config_file
                        .dependencies
                        .insert(name.to_string(), types::Dependency::DepWithHash(repo, b));
                }
                None => {
                    config_file
                        .dependencies
                        .insert(name.to_string(), types::Dependency::Dep(repo));
                }
            }

            std::fs::write(config, to_string(&config_file).unwrap())
                .expect("Can't write to config file");
            println!("Added {} to config", name);
        }
        Command::Clean => {
            enforce_config();
            std::fs::remove_dir_all(&home).expect("Can't remove .vendman directory");
            println!("Removed .vendman directory");
        }
        Command::Update => {
            enforce_config();
            let config_file: types::Config =
                toml::from_str(&std::fs::read_to_string(&config).expect("Can't read config file"))
                    .expect("Can't parse config file");

            for (name, dep) in config_file.dependencies {
                match dep {
                    types::Dependency::Dep(_) => {
                        let mut c = std::process::Command::new("git");
                        c.arg("pull")
                            .stdin(std::process::Stdio::null())
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null());
                        c.current_dir(home.clone()).arg(format!("./{name}"));
                        c.spawn().expect("Can't pull repo");
                        println!("Pulled {}", name);
                    }
                    types::Dependency::DepWithHash(_, branch) => {
                        let mut c = std::process::Command::new("git");
                        c.arg("fetch")
                            .arg("origin")
                            .arg(branch)
                            .stdin(std::process::Stdio::null())
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null());
                        c.current_dir(home.clone()).arg(format!("./{name}"));
                        c.spawn().expect("Can't fetch repo");
                        println!("Fetched {}", name);
                    }
                }
            }
        }
    }
}
