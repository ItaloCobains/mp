#![allow(unused)]

use clap::Parser;
use git2::build::RepoBuilder;
use git2::Error;
use serde::{Deserialize, Serialize};
use shlex::split;
use std::ffi::OsString;
use std::io::{self, Write};
use std::{env, fs, path::Path};
use std::process::Command;

#[derive(Debug, Serialize, Deserialize)]
struct Dependency {
    folder_name: String,
    install_command: String,
}

#[derive(Parser)]
struct Cli {
    pattern: String,
    url: Option<String>,
}

fn main() {
    let args = Cli::parse();

    match args.pattern.as_str() {
        "init" => init(),
        "add" => add(args.url.unwrap().as_str()),
        "list" => list(),
        "rinstall" => rinstall(),
        "runAll" => run_all(),
        _ => println!("default"),
    }
}

fn run_all() {
    if !verify_mp_folder_exists() {
        eprintln!("Diretório .mp não encontrado.");
        return;
    }

    let rt = root_path();

    let mp_path = Path::new(&rt).join(".mp");
    
    let json_file_path = "dependencies.json";

    let json_data = fs::read_to_string(json_file_path).expect("Erro ao ler o arquivo JSON");

    let dependencies: Vec<Dependency> = serde_json::from_str(&json_data).expect("Erro ao desserializar o JSON");

    for dependency in dependencies {
        println!("Instalando dependência na pasta '{}'", dependency.folder_name);

        let final_path = mp_path.join(&dependency.folder_name).display().to_string();

        let mut command = Command::new("sh");

        if cfg!(target_os = "windows") {
            command.arg("/C");
            let full_command = format!("{} && {}", final_path, dependency.install_command);
            if let Some(args) = split(&full_command) {
                command.args(args);
            } else {
                eprintln!("Erro ao dividir o comando em argumentos.");
                continue;
            }
        } else {
            command.arg("-c").arg(format!("cd {} && {}", final_path, dependency.install_command));
        }

        match command.output() {
            Ok(output) => {
                if output.status.success() {
                    println!("Comando executado com sucesso na pasta '{}'", dependency.folder_name);
                } else {
                    eprintln!(
                        "Erro ao executar o comando na pasta '{}': {}",
                        dependency.folder_name,
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "Erro ao executar o comando na pasta '{}': {}",
                    dependency.folder_name,
                    e
                );
            }
        }
    }
}

fn rinstall() {
    if !verify_mp_folder_exists() {
        eprintln!("Diretório .mp não encontrado.");
        return;
    }

    let rt = root_path();

    let mp_path = Path::new(&rt).join(".mp");

    let folders = list_folders_in_directory(mp_path.to_str().unwrap());

    if folders.is_empty() {
        println!("Nenhum repositório encontrado.");
        return;
    }

    let mut dependencies: Vec<Dependency> = Vec::new();

    for folder in folders {
        print!("Digite o comando de instalação para a pasta '{}': ", folder);
        io::stdout().flush().unwrap();

        let mut install_command = String::new();
        io::stdin().read_line(&mut install_command).unwrap();

        let dependency = Dependency {
            folder_name: folder,
            install_command: install_command.trim().to_string(),
        };

        dependencies.push(dependency);
    }

    let json_data  = serde_json::to_string(&dependencies).unwrap();

    let json_file_path = "dependencies.json";

    if let Err(err) = fs::write(json_file_path, json_data) {
        eprintln!("Erro ao salvar o arquivo JSON: {}", err);
    } else {
        println!("Informações salvas no arquivo '{}'", json_file_path);
    }
}

fn list() {
    if !verify_mp_folder_exists() {
        eprintln!("Diretório .mp não encontrado.");
        return;
    }

    let rt = root_path();

    let mp_path = Path::new(&rt).join(".mp");

    let folders = list_folders_in_directory(mp_path.to_str().unwrap());

    if folders.is_empty() {
        println!("Nenhum repositório encontrado.");
        return;
    }

    println!("Repositórios encontrados:");

    for folder in folders {
        println!("{}", folder);
    }
}

fn list_folders_in_directory(directory_path: &str) -> Vec<String> {
    let dir = match fs::read_dir(directory_path) {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("Erro ao acessar o diretório: {}", err);
            return Vec::new();
        }
    };

    let mut folders = Vec::new();

    for entry in dir {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_dir() {
                if let Some(folder_name) = path.file_name().and_then(|n| n.to_str()) {
                    folders.push(folder_name.to_string());
                }
            }
        }
    }

    folders
}

fn add(url: &str) {
    if !verify_mp_folder_exists() {
        eprintln!("Diretório .mp não encontrado.");
        return;
    }

    let rt = root_path();

    let mp_path = Path::new(&rt).join(".mp");

    let repo_name = url.split("/").last().unwrap();

    let repo_path = mp_path.join(repo_name);

    if repo_path.exists() {
        eprintln!("Repositório já existe.");
        return;
    }

    if let Err(err) = git_clone(url, &repo_path) {
        eprintln!("Erro ao clonar o repositório: {:?}", err);
        return;
    }

    println!(
        "Repositório clonado com sucesso em: {}",
        repo_path.to_str().unwrap()
    );
}

fn git_clone(url: &str, destination: &Path) -> Result<(), Error> {
    let repo = RepoBuilder::new().clone(url, destination)?;

    println!(
        "Repositório clonado com sucesso em: {}",
        destination.to_str().unwrap()
    );
    Ok(())
}

fn root_path() -> OsString {
    let root_path = match env::var_os("HOME") {
        Some(path) => path,
        None => match env::var_os("USERPROFILE") {
            Some(path) => path,
            None => {
                eprintln!("Não foi possível encontrar o diretório raiz.");
                "".into()
            }
        },
    };

    root_path
}

fn verify_mp_folder_exists() -> bool {
    let rt = root_path();

    let mp_path = Path::new(&rt).join(".mp");

    mp_path.exists()
}

fn init() {
    let rt = root_path();
    let mp_path = Path::new(&rt).join(".mp");

    if let Err(err) = fs::create_dir_all(&mp_path) {
        eprintln!("Erro ao criar o diretório: {:?}", err);
        return;
    }

    println!("Diretório criado com sucesso: {:?}", mp_path);
}
