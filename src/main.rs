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
        /// Optional category path (e.g., 'api/production/internal')
        #[arg(short, long)]
        category: Option<String>,
    },
    /// Retrieve a stored value
    Get {
        /// The name of the key to retrieve
        #[arg(index = 1)]
        key: String,
        /// Optional category path (e.g., 'api/production/internal')
        #[arg(short, long)]
        category: Option<String>,
    },
    /// Initialize the AxKeyStore repository on GitHub
    Init {
        /// Name of the repository to use/create
        #[arg(short, long, default_value = "axkeystore-storage")]
        repo: String,
    },
    /// Delete a stored key
    Delete {
        /// The name of the key to delete
        #[arg(index = 1)]
        key: String,
        /// Optional category path (e.g., 'api/production/internal')
        #[arg(short, long)]
        category: Option<String>,
    },
}

fn prompt_password() -> Result<String> {
    print!("Enter master password: ");
    std::io::stdout().flush()?;
    rpassword::read_password().context("Failed to read password")
}

fn prompt_yes_no(message: &str) -> Result<bool> {
    print!("{} (y/n): ", message);
    std::io::stdout().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();
    Ok(input == "y" || input == "yes")
}

fn display_banner() {
    // ANSI color codes
    const CYAN: &str = "\x1b[36m";
    const GREEN: &str = "\x1b[32m";
    const MAGENTA: &str = "\x1b[35m";
    const RESET: &str = "\x1b[0m";
    const BOLD: &str = "\x1b[1m";
    const DIM: &str = "\x1b[2m";

    println!();
    println!("{CYAN}{BOLD}  ╠═══════════════════════════════════════════════════════════════════╣{RESET}");
    println!(
        "{CYAN}{BOLD}  {RESET}  {GREEN}★{RESET} {BOLD}AxKeyStore{RESET} is an {MAGENTA}Open Source Project{RESET} built by {BOLD}Appxiom Team{RESET}"
    );
    println!(
        "{CYAN}{BOLD}  {RESET}                                                                   {RESET}"
    );
    println!(
        "{CYAN}{BOLD}  {RESET}  {DIM}Visit{RESET} {CYAN}{BOLD}https://www.appxiom.com{RESET} {DIM}to know more about us.{RESET}"
    );
    println!(
        "{CYAN}{BOLD}  {RESET}  {DIM}You will love our product if you are into software engineering!{RESET}"
    );
    println!("{CYAN}{BOLD}  ╚═══════════════════════════════════════════════════════════════════╝{RESET}");
    println!();
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok(); // Load .env file if it exists
    let cli = Cli::parse();

    // Display the banner
    display_banner();

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
        Commands::Store {
            key,
            value,
            category,
        } => {
            let repo_name = config::Config::get_repo_name()?;
            let storage = storage::Storage::new(&repo_name).await?;

            let display_path = match &category {
                Some(cat) => format!("{}/{}", cat.trim_matches('/'), key),
                None => key.clone(),
            };

            // Check if key already exists
            if let Some((_, _)) = storage.get_blob(key, category.as_deref()).await? {
                let should_update = prompt_yes_no(&format!(
                    "Key '{}' already exists. Do you want to update it?",
                    display_path
                ))?;

                if !should_update {
                    println!("Update cancelled.");
                    return Ok(());
                }
            }

            let password = prompt_password()?;
            let encrypted = crypto::CryptoHandler::encrypt(value.as_bytes(), &password)?;
            let json_blob = serde_json::to_vec(&encrypted)?;

            storage
                .save_blob(key, &json_blob, category.as_deref())
                .await?;

            println!("Key '{}' stored successfully.", display_path);
        }
        Commands::Get { key, category } => {
            let repo_name = config::Config::get_repo_name()?;
            let storage = storage::Storage::new(&repo_name).await?;

            let display_path = match &category {
                Some(cat) => format!("{}/{}", cat.trim_matches('/'), key),
                None => key.clone(),
            };

            if let Some((data, _)) = storage.get_blob(key, category.as_deref()).await? {
                let encrypted: crypto::EncryptedBlob = serde_json::from_slice(&data)?;
                let password = prompt_password()?;
                let decrypted = crypto::CryptoHandler::decrypt(&encrypted, &password)?;
                let value =
                    String::from_utf8(decrypted).context("Decrypted data is not valid UTF-8")?;
                println!("{}", value);
            } else {
                eprintln!("Key '{}' not found.", display_path);
                std::process::exit(1);
            }
        }
        Commands::Delete { key, category } => {
            let repo_name = config::Config::get_repo_name()?;
            let storage = storage::Storage::new(&repo_name).await?;

            let display_path = match &category {
                Some(cat) => format!("{}/{}", cat.trim_matches('/'), key),
                None => key.clone(),
            };

            // Check if key exists first
            if storage.get_blob(key, category.as_deref()).await?.is_none() {
                eprintln!("Key '{}' not found.", display_path);
                std::process::exit(1);
            }

            // Confirm deletion
            let should_delete = prompt_yes_no(&format!(
                "Are you sure you want to delete key '{}'?",
                display_path
            ))?;

            if !should_delete {
                println!("Deletion cancelled.");
                return Ok(());
            }

            if storage.delete_blob(key, category.as_deref()).await? {
                println!("Key '{}' deleted successfully.", display_path);
            } else {
                eprintln!("Failed to delete key '{}'.", display_path);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
