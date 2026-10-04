#[cfg(feature = "cli")]
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[cfg(feature = "cli")]
#[derive(Parser)]
#[command(name = "matrix-bridge", about = "E2EE Matrix bridge — CLI and MCP server")]
pub struct Cli {
    /// Output in JSON format
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[cfg(feature = "cli")]
#[derive(Subcommand)]
pub enum Commands {
    /// Interactive setup: login, create device, save config
    Setup,

    /// Login via an SSO login token (m.login.token) — for OAuth/Google accounts.
    /// Flow: open <homeserver>/_matrix/client/v3/login/sso/redirect?redirectUrl=http://localhost:8765/callback
    /// in a browser, authenticate, then copy the `loginToken` from the redirect URL.
    LoginToken {
        /// The loginToken from the SSO redirect URL
        token: String,
    },

    /// Restore a session from an existing access token.
    /// Useful for accounts created via OAuth/Google that don't have a password.
    RestoreToken {
        /// Matrix user ID (e.g. @user:matrix.org)
        user_id: String,

        /// Existing access token from Element or another Matrix client
        token: String,

        /// Existing device ID associated with the token
        device_id: String,
    },

    /// Restore Megolm room keys from the server-side key backup
    /// (requires the recovery key / "security key" from Element)
    Restore {
        /// Path to a file containing the base58 recovery key (starts with EsT)
        #[arg(long)]
        recovery_key_file: String,

        /// Backup version (see GET /_matrix/client/v3/room_keys/version)
        #[arg(long, default_value = "7")]
        version: String,
    },

    /// Send a message to a room
    Send {
        /// Message text
        message: String,

        /// Room ID (overrides default_room)
        #[arg(short, long)]
        room: Option<String>,

        /// User ID to @mention
        #[arg(short, long)]
        mention: Option<String>,

        /// Suppress default mention
        #[arg(long)]
        no_mention: bool,
    },

    /// Read recent messages from a room
    Read {
        /// Room ID (overrides default_room)
        #[arg(short, long)]
        room: Option<String>,

        /// Number of messages (1-100)
        #[arg(short, long, default_value = "10", value_parser = clap::value_parser!(u32).range(1..=100))]
        limit: u32,
    },

    /// List joined rooms
    Rooms,

    /// Import an Element "Export E2E room keys" file (unlocks old messages)
    ImportExport {
        /// Path to the export file
        #[arg(long)]
        file: PathBuf,
        /// Passphrase (prompted if omitted and a TTY is available)
        #[arg(long)]
        passphrase: Option<String>,
    },

    /// Arm auto-verify and keep syncing, so you can verify the bridge
    /// session from Element (Settings → Sessions → matrix-bridge → Verify).
    /// The bridge accepts and confirms the emoji comparison automatically.
    VerifyWait {
        /// How long to wait for the verification flow (seconds)
        #[arg(long, default_value = "180")]
        timeout: u64,
    },

    /// Send a message and wait for a reply
    SendWait {
        /// Message text
        message: String,

        /// Room ID (overrides default_room)
        #[arg(short, long)]
        room: Option<String>,

        /// User ID to @mention
        #[arg(short, long)]
        mention: Option<String>,

        /// Suppress default mention
        #[arg(long)]
        no_mention: bool,

        /// Timeout in seconds (1-300)
        #[arg(short, long, default_value = "30", value_parser = clap::value_parser!(u64).range(1..=300))]
        timeout: u64,
    },

    /// View or set config values
    Config {
        /// Config key to view or set
        key: Option<String>,

        /// Value to set
        value: Option<String>,
    },

    /// Start the MCP server (stdin/stdout)
    #[cfg(feature = "mcp")]
    McpServer,
}
