mod auth;
mod config;
mod crypto;
mod storage;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use rand::Rng;
use std::io::Write;

/// Command line arguments for AxKeyStore
#[derive(Parser)]
#[command(name = "axkeystore")]
#[command(about = "A secure, GitHub-backed keystore CLI", long_about = None)]
struct Cli {
    /// Use a specific profile
    #[arg(short, long, global = true)]
    profile: Option<String>,

    /// Command to execute
    #[command(subcommand)]
    command: Commands,
}

/// Available subcommands for AxKeyStore
#[derive(Subcommand)]
enum Commands {
    /// Authenticate with GitHub
    Login,
    /// Store a key-value pair securely
    Store {
        /// The name of the key
        #[arg(short, long)]
        key: String,
        /// The value to store (if not provided, a random alphanumeric value will be generated)
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
        /// Optional version (SHA) to retrieve
        #[arg(short, long)]
        version: Option<String>,
    },
    /// View the version history of a key
    History {
        /// The name of the key
        #[arg(index = 1)]
        key: String,
        /// Optional category path
        #[arg(short, long)]
        category: Option<String>,
    },
    /// Initialize the AxKeyStore repository on GitHub
    Init {
        /// Name of the repository to use
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
    /// Manage profiles
    Profile {
        #[command(subcommand)]
        command: ProfileCommands,
    },
    /// Reset your master password
    ResetPassword,
}

/// Profile management subcommands
#[derive(Subcommand)]
enum ProfileCommands {
    /// List all profiles
    List,
    /// Switch to a specific profile
    Switch {
        /// The name of the profile to switch to (omit to switch to default root)
        #[arg(index = 1)]
        name: Option<String>,
    },
    /// Delete a profile
    Delete {
        /// The name of the profile to delete
        #[arg(index = 1)]
        name: String,
    },
    /// Show current profile
    Current,
    /// Create a new profile
    Create {
        /// The name of the profile to create
        #[arg(index = 1)]
        name: String,
    },
}

/// Prompts the user for a password via stdin without echo
fn prompt_password(message: &str) -> Result<String> {
    print!("{}: ", message);
    std::io::stdout().flush()?;
    rpassword::read_password().context("Failed to read password")
}

/// Retrieves the master key from GitHub or initializes it if it doesn't exist
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
            println!("Master key initialized and saved to GitHub.");
            Ok(master_key)
        }
    }
}

/// Prompts the user for a yes/no confirmation via stdin
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

/// Displays the AxKeyStore application banner
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

/// Entry point for the AxKeyStore CLI
#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok(); // Load .env file if it exists
    let cli = Cli::parse();

    // Display the banner
    display_banner();

    // Determine the effective profile
    let effective_profile = match (&cli.profile, config::GlobalConfig::get_active_profile()?) {
        (Some(p), _) => {
            config::Config::validate_profile_name(p)?;
            Some(p.clone())
        }
        (None, Some(p)) => Some(p),
        (None, None) => None,
    };

    let profile_str = effective_profile.as_deref().unwrap_or("default");

    match &cli.command {
        Commands::Login => {
            if auth::is_logged_in_with_profile(effective_profile.as_deref()) {
                let reauth = prompt_yes_no(
                    "You are already logged in for this profile. Do you want to re-authenticate?",
                )?;
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

            // Check if LMK already exists for this profile
            let config = config::Config::load_with_profile(effective_profile.as_deref())?;
            let lmk_exists = config.encrypted_lmk.is_some();

            println!("Setting up master password to secure your token locally...");
            let password = if lmk_exists {
                println!("A master password is already set for this profile.");
                let p = prompt_password("Enter master password")?;

                // Verify the password by trying to decrypt the LMK
                match config::Config::get_or_create_lmk_with_profile(
                    effective_profile.as_deref(),
                    &p,
                ) {
                    Ok(_) => p,
                    Err(_) => {
                        eprintln!("Incorrect master password.");
                        std::process::exit(1);
                    }
                }
            } else {
                loop {
                    let p1 = prompt_password("Set master password")?;
                    if p1.len() < 8 {
                        eprintln!("Password must be at least 8 characters long.");
                        continue;
                    }
                    let p2 = prompt_password("Confirm master password")?;
                    if p1 == p2 {
                        break p1;
                    }
                    eprintln!("Passwords do not match. Please try again.");
                }
            };

            auth::save_token_with_profile(effective_profile.as_deref(), &token, &password)?;
            println!(
                "Successfully authenticated and secured token for profile '{}'.",
                effective_profile.as_deref().unwrap_or("default")
            );
            println!("\nNext step: If you haven't already, ensure your repository exists on GitHub, then run 'axkeystore init --repo <YOUR_REPO>' to set up your vault.");
        }
        Commands::Init { repo } => {
            let password = prompt_password("Enter master password")?;
            let storage =
                storage::Storage::new_with_profile(effective_profile.as_deref(), repo, &password)
                    .await?;
            storage.init_repo().await?;

            // Verify if the password matches the remote master key (if it exists)
            if let Some(blob) = storage.get_master_key_blob().await? {
                let encrypted: crypto::EncryptedBlob = serde_json::from_slice(&blob)
                    .context("Failed to parse master key blob from GitHub")?;

                if crypto::CryptoHandler::decrypt(&encrypted, &password).is_err() {
                    eprintln!("\nError: The provided password is incorrect for this repository.");
                    eprintln!("   This repository already has a master key encrypted with a different password.");
                    eprintln!(
                        "   Please provide the correct password to sync with this repository.\n"
                    );
                    std::process::exit(1);
                }
                println!("Master password verified against existing repository.");
            }

            config::Config::set_repo_name_with_profile(
                effective_profile.as_deref(),
                repo,
                &password,
            )?;
            println!(
                "Configuration saved for profile '{}'.",
                effective_profile.as_deref().unwrap_or("default")
            );
        }
        Commands::Store {
            key,
            value,
            category,
        } => {
            let password = prompt_password("Enter master password")?;
            let repo_name = config::Config::get_repo_name_with_profile(
                effective_profile.as_deref(),
                &password,
            )?;
            let storage = storage::Storage::new_with_profile(
                effective_profile.as_deref(),
                &repo_name,
                &password,
            )
            .await?;
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
                    println!("\nGenerated value: {}", generated);
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
        Commands::Get {
            key,
            category,
            version,
        } => {
            let password = prompt_password("Enter master password")?;
            let repo_name = config::Config::get_repo_name_with_profile(
                effective_profile.as_deref(),
                &password,
            )?;
            let storage = storage::Storage::new_with_profile(
                effective_profile.as_deref(),
                &repo_name,
                &password,
            )
            .await?;
            let master_key = get_or_init_master_key(&storage, &password).await?;

            let display_path = match &category {
                Some(cat) => format!("{}/{}", cat.trim_matches('/'), key),
                None => key.clone(),
            };

            let data = if let Some(sha) = version {
                storage
                    .get_blob_at_version(key, category.as_deref(), sha)
                    .await?
            } else {
                storage
                    .get_blob(key, category.as_deref())
                    .await?
                    .map(|(d, _)| d)
            };

            if let Some(data) = data {
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
        Commands::History { key, category } => {
            let password = prompt_password("Enter master password")?;
            let repo_name = config::Config::get_repo_name_with_profile(
                effective_profile.as_deref(),
                &password,
            )?;
            let storage = storage::Storage::new_with_profile(
                effective_profile.as_deref(),
                &repo_name,
                &password,
            )
            .await?;

            let mut page = 1;
            loop {
                let versions = storage
                    .get_key_history(key, category.as_deref(), page, 10)
                    .await?;
                if versions.is_empty() {
                    if page == 1 {
                        println!("No history found for key '{}'.", key);
                    } else {
                        println!("No more versions found.");
                    }
                    break;
                }

                println!("\nVersion History for '{}':", key);
                println!("{:<40} | {:<25} | {}", "SHA", "Date", "Message");
                println!("{:-<40}-+-{:-<25}-+-{:-<20}", "", "", "");

                for v in &versions {
                    println!("{:<40} | {:<25} | {}", v.sha, v.date, v.message);
                }

                if versions.len() < 10 {
                    break;
                }

                if !prompt_yes_no("\nShow more versions?")? {
                    break;
                }
                page += 1;
            }
        }
        Commands::Delete { key, category } => {
            let password = prompt_password("Enter master password")?;
            let repo_name = config::Config::get_repo_name_with_profile(
                effective_profile.as_deref(),
                &password,
            )?;
            let storage = storage::Storage::new_with_profile(
                effective_profile.as_deref(),
                &repo_name,
                &password,
            )
            .await?;
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
        Commands::Profile { command } => match command {
            ProfileCommands::List => {
                let profiles = config::GlobalConfig::list_profiles()?;
                let active = config::GlobalConfig::get_active_profile()?;
                println!("\nProfiles:");
                if profiles.is_empty() {
                    println!("  (No profiles created)");
                } else {
                    for p in profiles {
                        let indicator = if Some(&p) == active.as_ref() {
                            "*"
                        } else {
                            " "
                        };
                        println!(" {} {}", indicator, p);
                    }
                }
                println!("\n* Active profile");
            }
            ProfileCommands::Switch { name } => {
                config::GlobalConfig::set_active_profile(name.clone())?;
                match name {
                    Some(n) => println!("Switched to profile '{}'.", n),
                    None => println!("Switched to default root profile."),
                }
            }
            ProfileCommands::Delete { name } => {
                if prompt_yes_no(&format!(
                    "Are you sure you want to delete profile '{}'?",
                    name
                ))? {
                    config::GlobalConfig::delete_profile(name)?;
                    println!("Profile '{}' deleted.", name);
                }
            }
            ProfileCommands::Current => {
                let active = config::GlobalConfig::get_active_profile()?;
                println!(
                    "Current active profile: {}",
                    active.unwrap_or_else(|| "default".to_string())
                );
            }
            ProfileCommands::Create { name } => {
                config::Config::get_config_dir(Some(&name))?;
                println!("Profile '{}' created.", name);
            }
        },
        Commands::ResetPassword => {
            let old_password = prompt_password("Enter current master password")?;

            // 1. Verify old password and retrieve LMK
            let lmk = match config::Config::get_or_create_lmk_with_profile(
                effective_profile.as_deref(),
                &old_password,
            ) {
                Ok(k) => k,
                Err(_) => {
                    eprintln!("Incorrect old master password.");
                    std::process::exit(1);
                }
            };

            // 2. Try to retrieve RMK if storage is configured
            let mut rmk_data: Option<(String, storage::Storage)> = None;
            if let Ok(repo_name) = config::Config::get_repo_name_with_profile(
                effective_profile.as_deref(),
                &old_password,
            ) {
                if let Ok(storage) = storage::Storage::new_with_profile(
                    effective_profile.as_deref(),
                    &repo_name,
                    &old_password,
                )
                .await
                {
                    if let Ok(Some(data)) = storage.get_master_key_blob().await {
                        let encrypted: crypto::EncryptedBlob = serde_json::from_slice(&data)?;
                        if let Ok(decrypted) =
                            crypto::CryptoHandler::decrypt(&encrypted, &old_password)
                        {
                            let rmk = String::from_utf8(decrypted)?;
                            rmk_data = Some((rmk, storage));
                        }
                    }
                }
            }

            // 3. Prompt for new password
            println!("\nEnter your new master password:");
            let new_password = loop {
                let p1 = prompt_password("New master password")?;
                if p1.len() < 8 {
                    eprintln!("Password must be at least 8 characters long.");
                    continue;
                }
                let p2 = prompt_password("Confirm new master password")?;
                if p1 == p2 {
                    if p1 == old_password {
                        eprintln!("New password must be different from the old one.");
                        continue;
                    }
                    break p1;
                }
                eprintln!("Passwords do not match. Please try again.");
            };

            // 4. Update RMK remotely if it exists
            if let Some((rmk, storage)) = rmk_data {
                let encrypted_rmk = crypto::CryptoHandler::encrypt(rmk.as_bytes(), &new_password)?;
                let json_blob = serde_json::to_vec(&encrypted_rmk)?;
                if let Err(e) = storage.save_master_key_blob(&json_blob).await {
                    eprintln!("Failed to update remote master key on GitHub: {}", e);
                    eprintln!("   Password reset aborted. Your current password is still active.");
                    std::process::exit(1);
                }
                println!("Remote master key updated on GitHub.");
            }

            // 5. Update LMK locally
            let encrypted_lmk = crypto::CryptoHandler::encrypt(lmk.as_bytes(), &new_password)?;
            let mut cfg = config::Config::load_with_profile(effective_profile.as_deref())?;
            cfg.encrypted_lmk = Some(encrypted_lmk);
            cfg.save_with_profile(effective_profile.as_deref())?;

            println!(
                "Master password successfully reset for profile '{}'.",
                profile_str
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_random_alphanumeric() {
        for _ in 0..100 {
            let s = generate_random_alphanumeric();
            assert!(s.len() >= 6 && s.len() <= 36);
            assert!(s.chars().all(|c| c.is_alphanumeric()));
        }
    }
}
