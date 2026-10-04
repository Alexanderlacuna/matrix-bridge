mod cli;
mod client;
mod config;
mod error;
mod format;
mod mcp;
mod trust;

use crate::client::MatrixBridgeClient;
use crate::config::{Config, Credentials};
use tracing_subscriber::EnvFilter;

#[cfg(feature = "cli")]
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("warn,matrix_bridge=info")),
        )
        .with_writer(std::io::stderr)
        .init();

    #[cfg(feature = "cli")]
    {
        let cli = cli::Cli::parse();
        run_command(cli).await?;
    }

    #[cfg(not(feature = "cli"))]
    {
        eprintln!("CLI feature not enabled. Use `matrix-bridge-mcp` for MCP server.");
    }

    Ok(())
}

#[cfg(feature = "cli")]
async fn run_command(cli: cli::Cli) -> anyhow::Result<()> {
    use cli::Commands;

    match cli.command {
        Commands::Setup => cmd_setup().await?,
        Commands::LoginToken { token } => cmd_login_token(&token).await?,
        Commands::RestoreToken {
            user_id,
            token,
            device_id,
        } => cmd_restore_token(&user_id, &token, &device_id).await?,
        Commands::Restore {
            recovery_key_file,
            version,
        } => cmd_restore(&recovery_key_file, &version).await?,
        Commands::Send {
            message,
            room,
            mention,
            no_mention,
        } => cmd_send(&message, room, mention, no_mention, cli.json).await?,
        Commands::Read { room, limit } => cmd_read(room, limit, cli.json).await?,
        Commands::Rooms => cmd_rooms(cli.json).await?,
        Commands::ImportExport { file, passphrase } => {
            cmd_import_export(file, passphrase).await?
        }
        Commands::VerifyWait { timeout } => cmd_verify_wait(timeout).await?,
        Commands::SendWait {
            message,
            room,
            mention,
            no_mention,
            timeout,
        } => cmd_send_wait(&message, room, mention, no_mention, timeout, cli.json).await?,
        Commands::Config { key, value } => cmd_config(key, value)?,
        #[cfg(feature = "mcp")]
        Commands::McpServer => mcp::run_server().await?,
    }

    Ok(())
}

#[cfg(feature = "cli")]
async fn cmd_setup() -> anyhow::Result<()> {
    use std::io::{self, Write};

    print!("Matrix user ID (e.g. @bot:matrix.org): ");
    io::stdout().flush()?;
    let mut user_id = String::new();
    io::stdin().read_line(&mut user_id)?;
    let user_id = user_id.trim().to_string();

    if !user_id.starts_with('@') || !user_id.contains(':') {
        anyhow::bail!("Invalid user ID format. Expected @user:server");
    }

    let server = user_id.split(':').nth(1).unwrap();
    let homeserver = format!("https://{}", server);

    let password = rpassword::prompt_password(format!("Password for {}: ", user_id))?;

    let config = Config {
        homeserver,
        user_id,
        device_name: "matrix-bridge".to_string(),
        store_path: config::default_dir()
            .join("store")
            .to_string_lossy()
            .into_owned(),
        trust_mode: config::TrustMode::Tofu,
        default_room: None,
        default_mention: None,
        notify_on_mention: None,
    };

    let client = MatrixBridgeClient::login_with_password(&config, &password).await?;

    config.save()?;
    println!("Setup complete. Session saved.");

    let rooms = client.get_rooms().await;
    if !rooms.is_empty() {
        println!("\nJoined rooms:");
        for r in &rooms {
            let name = r.name.as_deref().unwrap_or("(unnamed)");
            println!("  {} {}", r.room_id, name);
        }
        println!("\nSet a default room: matrix-bridge config default_room <room_id>");
    }

    Ok(())
}

#[cfg(feature = "cli")]
async fn cmd_restore_token(user_id: &str, token: &str, device_id: &str) -> anyhow::Result<()> {
    if !user_id.starts_with('@') || !user_id.contains(':') {
        anyhow::bail!("Invalid user ID format. Expected @user:server");
    }

    let server = user_id.split(':').nth(1).unwrap();
    let homeserver = format!("https://{}", server);

    let config = Config {
        homeserver,
        user_id: user_id.to_string(),
        device_name: "matrix-bridge".to_string(),
        store_path: config::default_dir()
            .join("store")
            .to_string_lossy()
            .into_owned(),
        trust_mode: config::TrustMode::Tofu,
        default_room: None,
        default_mention: None,
        notify_on_mention: None,
    };

    config.ensure_store_dir()?;
    config.save()?;

    let creds = Credentials {
        access_token: token.to_string(),
        user_id: user_id.to_string(),
        device_id: device_id.to_string(),
    };
    creds.save(&config)?;

    let mut client = MatrixBridgeClient::restore(&config).await?;
    client.sync_once().await?;

    println!("Session restored from token. Setup complete.");
    println!("User: {}", config.user_id);

    let rooms = client.get_rooms().await;
    if !rooms.is_empty() {
        println!("\nJoined rooms:");
        for r in &rooms {
            let name = r.name.as_deref().unwrap_or("(unnamed)");
            println!("  {} {}", r.room_id, name);
        }
        println!("\nSet a default room: matrix-bridge config default_room <room_id>");
    }

    Ok(())
}

#[cfg(feature = "cli")]
async fn cmd_restore(recovery_key_file: &str, version: &str) -> anyhow::Result<()> {
    use matrix_sdk::encryption::RoomKeyImportResult;

    let key = std::fs::read_to_string(recovery_key_file)
        .map_err(|e| anyhow::anyhow!("cannot read recovery key file: {e}"))?;

    let config = Config::load()?;
    let mut client = MatrixBridgeClient::restore(&config).await?;
    client.sync_once().await?;

    let result: RoomKeyImportResult = client.restore_from_backup(key.trim(), version).await?;

    println!("Restore from backup version {version} complete.");
    println!("  Sessions imported: {}", result.imported_count);
    println!("  Sessions total:    {}", result.total_count);
    println!("  Rooms affected:    {}", result.keys.len());
    println!("\nOld messages should now decrypt on the next `matrix-bridge read`.");

    Ok(())
}

#[cfg(feature = "cli")]
async fn cmd_import_export(
    file: std::path::PathBuf,
    passphrase: Option<String>,
) -> anyhow::Result<()> {
    let passphrase = match passphrase {
        Some(p) => p,
        None => rpassword::prompt_password("Export passphrase: ")?,
    };
    let config = Config::load()?;
    let client = MatrixBridgeClient::restore(&config).await?;
    client.import_export_file(&file, &passphrase).await?;
    Ok(())
}

#[cfg(feature = "cli")]
async fn cmd_verify_wait(timeout: u64) -> anyhow::Result<()> {
    let config = Config::load()?;
    let mut client = MatrixBridgeClient::restore(&config).await?;
    client.enable_auto_verify().await;

    println!("Auto-verify armed for {timeout}s.");
    println!("Now in Element: Settings → Security & Privacy → Sessions → matrix-bridge → Verify.");
    println!("Pick emoji verification — the bridge will accept and confirm automatically.");
    println!("Waiting...\n");

    client.sync_for(std::time::Duration::from_secs(timeout)).await?;

    println!("Verify-wait finished.");
    Ok(())
}

#[cfg(feature = "cli")]
async fn cmd_login_token(token: &str) -> anyhow::Result<()> {
    let config = Config::load()?;
    let mut client = MatrixBridgeClient::login_with_token(&config, token).await?;

    println!("Login successful. Setup complete.");
    println!("User: {}", config.user_id);
    println!("Device: {}", client.device_id()?);

    let rooms = client.get_rooms().await;
    if !rooms.is_empty() {
        println!("\nJoined rooms:");
        for r in &rooms {
            let name = r.name.as_deref().unwrap_or("(unnamed)");
            println!("  {} {}", r.room_id, name);
        }
    }

    Ok(())
}

#[cfg(feature = "cli")]
async fn cmd_send(
    message: &str,
    room: Option<String>,
    mention: Option<String>,
    no_mention: bool,
    json: bool,
) -> anyhow::Result<()> {
    let config = Config::load()?;
    let mut client = MatrixBridgeClient::restore(&config).await?;
    client.sync_once().await?;

    let room_id = resolve_room(&config, room)?;
    let mention = resolve_mention(&config, mention, no_mention);

    let event_id = client
        .send_message(&room_id, message, mention.as_deref())
        .await?;

    if json {
        println!("{}", serde_json::json!({ "event_id": event_id }));
    } else {
        println!("Sent (event: {})", event_id);
    }

    Ok(())
}

#[cfg(feature = "cli")]
async fn cmd_read(room: Option<String>, limit: u32, json: bool) -> anyhow::Result<()> {
    let config = Config::load()?;
    let mut client = MatrixBridgeClient::restore(&config).await?;
    client.sync_once().await?;

    let room_id = resolve_room(&config, room)?;
    let messages = client.read_messages(&room_id, limit).await?;

    println!("{}", format::format_messages(&messages, json));
    Ok(())
}

#[cfg(feature = "cli")]
async fn cmd_rooms(json: bool) -> anyhow::Result<()> {
    let config = Config::load()?;
    let mut client = MatrixBridgeClient::restore(&config).await?;
    client.sync_once().await?;

    let rooms = client.get_rooms().await;
    println!("{}", format::format_rooms(&rooms, json));
    Ok(())
}

#[cfg(feature = "cli")]
async fn cmd_send_wait(
    message: &str,
    room: Option<String>,
    mention: Option<String>,
    no_mention: bool,
    timeout: u64,
    json: bool,
) -> anyhow::Result<()> {
    let config = Config::load()?;
    let mut client = MatrixBridgeClient::restore(&config).await?;
    client.sync_once().await?;

    let room_id = resolve_room(&config, room)?;
    let mention = resolve_mention(&config, mention, no_mention);

    match client
        .send_and_wait(&room_id, message, mention.as_deref(), timeout)
        .await?
    {
        Some(reply) => {
            if json {
                println!("{}", serde_json::to_string_pretty(&reply)?);
            } else {
                let sender = reply
                    .sender
                    .strip_prefix('@')
                    .and_then(|s| s.split(':').next())
                    .unwrap_or(&reply.sender);
                println!("[{}] {}: {}", reply.timestamp, sender, reply.body);
            }
        }
        None => {
            if json {
                println!("{}", serde_json::json!({ "reply": null, "timeout": true }));
            } else {
                eprintln!("No reply within {}s", timeout);
            }
        }
    }

    Ok(())
}

#[cfg(feature = "cli")]
fn cmd_config(key: Option<String>, value: Option<String>) -> anyhow::Result<()> {
    match (key, value) {
        (None, None) => {
            let config = Config::load()?;
            println!("{}", serde_json::to_string_pretty(&config)?);
        }
        (Some(key), None) => {
            let config = Config::load()?;
            let val = serde_json::to_value(&config)?;
            match val.get(&key) {
                Some(v) => println!("{}", serde_json::to_string_pretty(v)?),
                None => anyhow::bail!("Unknown config key: {}", key),
            }
        }
        (Some(key), Some(value)) => {
            let mut config = Config::load()?;
            let mut val = serde_json::to_value(&config)?;
            let obj = val.as_object_mut().unwrap();
            obj.insert(key.clone(), serde_json::Value::String(value));
            config = serde_json::from_value(val)?;
            config.save()?;
            println!("Set {} ✓", key);
        }
        _ => unreachable!(),
    }
    Ok(())
}

#[cfg(feature = "cli")]
fn resolve_room(config: &Config, room: Option<String>) -> anyhow::Result<String> {
    room.or_else(|| config.default_room.clone())
        .ok_or_else(|| anyhow::anyhow!("No room specified and no default_room in config"))
}

#[cfg(feature = "cli")]
fn resolve_mention(
    config: &Config,
    mention: Option<String>,
    no_mention: bool,
) -> Option<String> {
    if no_mention {
        None
    } else {
        mention.or_else(|| config.default_mention.clone())
    }
}
