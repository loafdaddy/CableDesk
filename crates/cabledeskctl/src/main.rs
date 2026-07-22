//! `cabledeskctl` — diagnostics and development CLI for CableDesk.
//!
//! This is a development/diagnostics tool, not the primary user interface;
//! normal users are expected to use the GTK application after installation
//! (see docs/ARCHITECTURE.md, "CLI").

use cabledesk_platform::{CheckStatus, CompatibilityCheck, PlatformBackend};
use cabledesk_platform_fedora::FedoraBackend;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "cabledeskctl",
    about = "CableDesk diagnostics and development CLI"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Print machine-readable JSON instead of formatted text.
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Run the first-run compatibility checks and print a report.
    Compatibility,
    /// Print the full sanitised diagnostics bundle.
    Diagnostics,
    /// Print the current CableDesk agent state (not yet implemented).
    Status,
    /// List detected direct-link interfaces (not yet implemented).
    Links,
    /// Print USB-C Power Delivery and battery status.
    Power,
    /// List trusted devices (not yet implemented).
    Devices,
    /// Discover unpaired peers on the direct link (not yet implemented).
    Discover,
    /// Pair with a discovered peer (not yet implemented).
    Pair,
    /// Connect to a trusted device (not yet implemented).
    Connect { device: String },
    /// Disconnect the active session (not yet implemented).
    Disconnect,
    /// Remove a trusted device (not yet implemented).
    Forget { device: String },
    /// Print recent CableDesk logs (not yet implemented).
    Logs,
    /// Recreate the CableDesk NetworkManager profile (not yet implemented).
    RepairNetwork,
    /// Reset CableDesk configuration to defaults (not yet implemented).
    Reset,
    /// Remove all CableDesk state, including trusted devices (not yet implemented).
    Purge,
}

fn print_checks(title: &str, checks: &[CompatibilityCheck], json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(checks).unwrap());
        return;
    }
    println!("{title}");
    for check in checks {
        let symbol = match check.status {
            CheckStatus::Available => "[ ok ]",
            CheckStatus::NeedsSetup => "[setup]",
            CheckStatus::Warning => "[warn]",
            CheckStatus::Unsupported => "[ no ]",
            CheckStatus::Unknown => "[ ?? ]",
        };
        print!("  {symbol} {}", check.name);
        if let Some(detail) = &check.detail {
            print!(" — {detail}");
        }
        println!();
    }
}

fn not_implemented(command: &str) {
    eprintln!(
        "`cabledeskctl {command}` is not implemented in this milestone.\n\
         See docs/ROADMAP.md for the phase in which it lands."
    );
    std::process::exit(1);
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .init();

    let cli = Cli::parse();
    let backend = FedoraBackend::new();

    match cli.command {
        Command::Compatibility => {
            let system = match backend.detect_system().await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to detect system: {e}");
                    return std::process::ExitCode::FAILURE;
                }
            };

            let dependencies = backend.check_dependencies().await;
            let security = backend.inspect_security_system().await;
            let power = backend.inspect_power_delivery().await;
            let direct_link = cabledesk_platform_fedora::inspect_direct_link_report().await;

            if cli.json {
                // Emitted as a single JSON object rather than one array per
                // category, so `cabledeskctl compatibility --json` is one
                // parseable document, not several concatenated ones.
                let report = serde_json::json!({
                    "system": system,
                    "dependencies": dependencies.as_ref().ok().map(|r| &r.checks),
                    "security": security.as_ref().ok().map(|r| &r.checks),
                    "direct_link": direct_link.as_ref().ok(),
                    "power": power.as_ref().ok().map(|r| &r.checks),
                });
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
            } else {
                println!(
                    "{} — {} ({:?} session)\n",
                    system.distro_name,
                    system
                        .desktop_environment
                        .as_deref()
                        .unwrap_or("unknown desktop"),
                    system.session_type
                );
            }

            // Text-mode-only from here: the JSON case was already fully
            // emitted as one combined object above.
            if !cli.json {
                match dependencies {
                    Ok(r) => print_checks("Dependencies", &r.checks, false),
                    Err(e) => eprintln!("Failed to check dependencies: {e}"),
                }
                match security {
                    Ok(r) => print_checks("Security", &r.checks, false),
                    Err(e) => eprintln!("Failed to inspect security system: {e}"),
                }
                match direct_link {
                    Ok(checks) => print_checks("Direct link", &checks, false),
                    Err(e) => eprintln!("Failed to inspect direct link: {e}"),
                }
                match power {
                    Ok(r) => print_checks("Power", &r.checks, false),
                    Err(e) => eprintln!("Failed to inspect power delivery: {e}"),
                }
            }
        }
        Command::Diagnostics => match backend.collect_diagnostics().await {
            Ok(diagnostics) => {
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&diagnostics).unwrap());
                } else {
                    println!("{diagnostics:#?}");
                }
            }
            Err(e) => {
                eprintln!("Failed to collect diagnostics: {e}");
                return std::process::ExitCode::FAILURE;
            }
        },
        Command::Power => match backend.inspect_power_delivery().await {
            Ok(r) => print_checks("Power", &r.checks, cli.json),
            Err(e) => {
                eprintln!("Failed to inspect power delivery: {e}");
                return std::process::ExitCode::FAILURE;
            }
        },
        Command::Links => match cabledesk_platform_fedora::inspect_direct_link_report().await {
            Ok(checks) => print_checks("Direct link", &checks, cli.json),
            Err(e) => {
                eprintln!("Failed to inspect direct link: {e}");
                return std::process::ExitCode::FAILURE;
            }
        },
        Command::Status => not_implemented("status"),
        Command::Devices => not_implemented("devices"),
        Command::Discover => not_implemented("discover"),
        Command::Pair => not_implemented("pair"),
        Command::Connect { .. } => not_implemented("connect"),
        Command::Disconnect => not_implemented("disconnect"),
        Command::Forget { .. } => not_implemented("forget"),
        Command::Logs => not_implemented("logs"),
        Command::RepairNetwork => not_implemented("repair-network"),
        Command::Reset => not_implemented("reset"),
        Command::Purge => not_implemented("purge"),
    }

    std::process::ExitCode::SUCCESS
}
