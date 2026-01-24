mod auth;
mod config;
mod crypto;
mod storage;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::io::Write;

#[derive(Parser)]
#[command(name = "axkeystore")]
#[command(about = "A secure, GitHub-backed keystore CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with GitHub
    Login,
    /// Store a key-value pair securely
    Store {
        /// The name of the key
        #[arg(short, long)]
        key: String,
        /// The value to store
        #[arg(short, long)]
        value: String,
    },
    /// Retrieve a stored value
    Get {
        /// The name of the key to retrieve
        #[arg(index = 1)]
        key: String,
    },
    /// Initialize the AxKeyStore repository on GitHub
    Init {
        /// Name of the repository to use/create
        #[arg(short, long, default_value = "axkeystore-storage")]
        repo: String,
    },
}

fn prompt_password() -> Result<String> {
    print!("Enter master password: ");
    std::io::stdout().flush()?;
    rpassword::read_password().context("Failed to read password")
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok(); // Load .env file if it exists
    let cli = Cli::parse();

    match &cli.command {
        Commands::Login => {
            if let Err(e) = auth::authenticate().await {
                eprintln!("Authentication failed: {:#}", e);
                std::process::exit(1);
            }
        }
        Commands::Init { repo } => {
            let storage = storage::Storage::new(repo).await?;
            storage.init_repo().await?;
            config::Config::set_repo_name(repo)?;
            println!("Configuration saved.");
        }
        Commands::Store { key, value } => {
            let repo_name = config::Config::get_repo_name()?;

            let password = prompt_password()?;
            let encrypted = crypto::CryptoHandler::encrypt(value.as_bytes(), &password)?;
            let json_blob = serde_json::to_vec(&encrypted)?;

            let storage = storage::Storage::new(&repo_name).await?;
            storage.save_blob(key, &json_blob).await?;
            println!("Key '{}' stored successfully.", key);
        }
        Commands::Get { key } => {
            let repo_name = config::Config::get_repo_name()?;
            let storage = storage::Storage::new(&repo_name).await?;

            if let Some((data, _)) = storage.get_blob(key).await? {
                let encrypted: crypto::EncryptedBlob = serde_json::from_slice(&data)?;
                let password = prompt_password()?;
                let decrypted = crypto::CryptoHandler::decrypt(&encrypted, &password)?;
                let value =
                    String::from_utf8(decrypted).context("Decrypted data is not valid UTF-8")?;
                println!("{}", value);
            } else {
                eprintln!("Key '{}' not found.", key);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
