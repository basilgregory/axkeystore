mod auth;
mod config;
mod crypto;
mod storage;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use rand::Rng;
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
        /// The value to store (if not provided, a random alphabetic value will be generated)
        #[arg(short, long)]
        value: Option<String>,
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

fn prompt_password(message: &str) -> Result<String> {
    print!("{}: ", message);
    std::io::stdout().flush()?;
    rpassword::read_password().context("Failed to read password")
}

async fn get_or_init_master_key(storage: &storage::Storage, password: &str) -> Result<String> {
    match storage.get_master_key_blob().await? {
        Some(data) => {
            // Master key exists, try to decrypt it with the provided password
            let encrypted: crypto::EncryptedBlob = serde_json::from_slice(&data)
                .context("Failed to parse master key blob from GitHub")?;

            match crypto::CryptoHandler::decrypt(&encrypted, password) {
                Ok(decrypted) => {
                    return String::from_utf8(decrypted).context("Master key is not valid UTF-8");
                }
                Err(_) => {
                    return Err(anyhow::anyhow!(
                        "Incorrect master password. Please verify your credentials."
                    ));
                }
            }
        }
        None => {
            // Master key doesn't exist, we use the provided password to initialize it
            let master_key = crypto::CryptoHandler::generate_master_key();
            let encrypted = crypto::CryptoHandler::encrypt(master_key.as_bytes(), password)?;
            let json_blob = serde_json::to_vec(&encrypted)?;

            storage.save_master_key_blob(&json_blob).await?;
            println!("✅ Master key initialized and saved to GitHub.");
            Ok(master_key)
        }
    }
}

fn prompt_yes_no(message: &str) -> Result<bool> {
    print!("{} (y/n): ", message);
    std::io::stdout().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();
    Ok(input == "y" || input == "yes")
}

/// Generate a random alphanumeric string with length between 6 and 36 characters
fn generate_random_alphanumeric() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    let length = rng.gen_range(6..=36);

    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
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
            if auth::is_logged_in() {
                let reauth =
                    prompt_yes_no("You are already logged in. Do you want to re-authenticate?")?;
                if !reauth {
                    println!("Login cancelled.");
                    return Ok(());
                }
            }

            let token = match auth::authenticate().await {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Authentication failed: {:#}", e);
                    std::process::exit(1);
                }
            };

            println!("Setting up master password to secure your token locally...");
            let password = loop {
                let p1 = prompt_password("Set master password")?;
                if p1.len() < 8 {
                    eprintln!("❌ Password must be at least 8 characters long.");
                    continue;
                }
                let p2 = prompt_password("Confirm master password")?;
                if p1 == p2 {
                    break p1;
                }
                eprintln!("❌ Passwords do not match. Please try again.");
            };

            auth::save_token(&token, &password)?;
            println!("✅ Successfully authenticated and secured token.");
        }
        Commands::Init { repo } => {
            let password = prompt_password("Enter master password")?;
            let storage = storage::Storage::new(repo, &password).await?;
            storage.init_repo().await?;

            config::Config::set_repo_name(repo, &password)?;
            println!("Configuration saved.");
        }
        Commands::Store {
            key,
            value,
            category,
        } => {
            let password = prompt_password("Enter master password")?;
            let repo_name = config::Config::get_repo_name(&password)?;
            let storage = storage::Storage::new(&repo_name, &password).await?;
            let master_key = get_or_init_master_key(&storage, &password).await?;

            let display_path = match &category {
                Some(cat) => format!("{}/{}", cat.trim_matches('/'), key),
                None => key.clone(),
            };

            // Check if key already exists
            if let Ok(Some((_, _))) = storage.get_blob(key, category.as_deref()).await {
                let should_update = prompt_yes_no(&format!(
                    "Key '{}' already exists. Do you want to update it?",
                    display_path
                ))?;

                if !should_update {
                    println!("Update cancelled.");
                    return Ok(());
                }
            }

            // Determine the value to store
            let final_value = match value {
                Some(v) => v.clone(),
                None => {
                    // Generate a random alphabetic value
                    let generated = generate_random_alphanumeric();
                    println!("\n🔑 Generated value: {}", generated);
                    println!("   (Length: {} characters)\n", generated.len());

                    let confirmed = prompt_yes_no("Do you want to use this generated value?")?;

                    if !confirmed {
                        println!("Operation cancelled.");
                        return Ok(());
                    }
                    generated
                }
            };

            let encrypted = crypto::CryptoHandler::encrypt(final_value.as_bytes(), &master_key)?;
            let json_blob = serde_json::to_vec(&encrypted)?;

            storage
                .save_blob(key, &json_blob, category.as_deref())
                .await?;

            println!("Key '{}' stored successfully.", display_path);
        }
        Commands::Get { key, category } => {
            let password = prompt_password("Enter master password")?;
            let repo_name = config::Config::get_repo_name(&password)?;
            let storage = storage::Storage::new(&repo_name, &password).await?;
            let master_key = get_or_init_master_key(&storage, &password).await?;

            let display_path = match &category {
                Some(cat) => format!("{}/{}", cat.trim_matches('/'), key),
                None => key.clone(),
            };

            if let Some((data, _)) = storage.get_blob(key, category.as_deref()).await? {
                let encrypted: crypto::EncryptedBlob = serde_json::from_slice(&data)?;
                let decrypted = crypto::CryptoHandler::decrypt(&encrypted, &master_key)?;
                let value =
                    String::from_utf8(decrypted).context("Decrypted data is not valid UTF-8")?;
                println!("{}", value);
            } else {
                eprintln!("Key '{}' not found.", display_path);
                std::process::exit(1);
            }
        }
        Commands::Delete { key, category } => {
            let password = prompt_password("Enter master password")?;
            let repo_name = config::Config::get_repo_name(&password)?;
            let storage = storage::Storage::new(&repo_name, &password).await?;
            let _master_key = get_or_init_master_key(&storage, &password).await?;

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
