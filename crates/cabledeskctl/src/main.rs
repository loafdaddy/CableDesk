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
    /// Print the current CableDesk agent state.
    Status,
    /// Print the agent session snapshot (state, interface, peer).
    Session,
    /// List detected direct-link interfaces.
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
    /// Recreate the CableDesk NetworkManager profile and firewalld binding
    /// for the currently-detected direct-link interface.
    RepairNetwork,
    /// Reset CableDesk configuration to defaults (not yet implemented).
    Reset,
    /// Remove all CableDesk state, including trusted devices (not yet implemented).
    Purge,
    /// Development-only cable lifecycle simulation (requires `--features simulation`).
    #[cfg(feature = "simulation")]
    Simulate {
        #[command(subcommand)]
        event: SimulateEvent,
    },
}

#[cfg(feature = "simulation")]
#[derive(Subcommand, Clone, Copy)]
enum SimulateEvent {
    /// Run the full simulated cable-only lifecycle (insert → peer → stream → unplug).
    Demo,
    Usb4Controller,
    CableInserted,
    CableRemoved,
    DirectInterfaceAdded,
    DirectInterfaceRemoved,
    PeerDiscovered,
    /// Must reject: peer discovered on Wi-Fi.
    PeerOnWifi,
    PeerOnEthernet,
    PeerTrusted,
    IdentityMismatch,
    Charging,
    Discharging,
    StreamStarted,
    StreamFailed,
    /// Print current simulated state.
    Status,
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

/// Queries `cabledesk-agent`'s `GetState` over the session D-Bus — a thin
/// client for `data/dbus-1/interfaces/org.cabledesk.Agent1.xml`, kept
/// local to this CLI rather than a shared crate since it's the only
/// caller so far.
#[zbus::proxy(
    interface = "org.cabledesk.Agent1",
    default_service = "org.cabledesk.Agent1",
    default_path = "/org/cabledesk/Agent"
)]
trait Agent1 {
    async fn get_state(&self) -> zbus::Result<String>;
    async fn get_session_json(&self) -> zbus::Result<String>;
}

#[zbus::proxy(
    interface = "org.cabledesk.Helper1",
    default_service = "org.cabledesk.Helper1",
    default_path = "/org/cabledesk/Helper"
)]
trait Helper1 {
    async fn prepare_direct_link(&self, interface_name: &str) -> zbus::Result<()>;
}

async fn query_agent_state() -> zbus::Result<String> {
    let connection = zbus::Connection::session().await?;
    let proxy = Agent1Proxy::new(&connection).await?;
    proxy.get_state().await
}

async fn query_agent_session_json() -> zbus::Result<String> {
    let connection = zbus::Connection::session().await?;
    let proxy = Agent1Proxy::new(&connection).await?;
    proxy.get_session_json().await
}

async fn helper_prepare_direct_link(interface_name: &str) -> zbus::Result<()> {
    let connection = zbus::Connection::system().await?;
    let proxy = Helper1Proxy::new(&connection).await?;
    proxy.prepare_direct_link(interface_name).await
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
        Command::Status => {
            match query_agent_state().await {
                Ok(state) => {
                    if cli.json {
                        println!("{}", serde_json::json!({ "state": state }));
                    } else {
                        println!("{state}");
                    }
                }
                Err(e) => {
                    eprintln!("Failed to reach cabledesk-agent: {e}");
                    eprintln!("Is cabledesk-agent.service running? (systemctl --user status cabledesk-agent)");
                    return std::process::ExitCode::FAILURE;
                }
            }
        }
        Command::Session => match query_agent_session_json().await {
            Ok(json) => {
                if cli.json {
                    println!("{json}");
                } else if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json) {
                    println!("{}", serde_json::to_string_pretty(&value).unwrap_or(json));
                } else {
                    println!("{json}");
                }
            }
            Err(e) => {
                eprintln!("Failed to reach cabledesk-agent: {e}");
                eprintln!(
                    "Is cabledesk-agent.service running? (systemctl --user status cabledesk-agent)"
                );
                return std::process::ExitCode::FAILURE;
            }
        },
        Command::RepairNetwork => {
            match cabledesk_platform_fedora::detect_direct_link_interface_name().await {
                Ok(Some(interface_name)) => {
                    println!("Found direct-link interface: {interface_name}");
                    println!(
                        "Asking cabledesk-helper (Polkit) to recreate the NetworkManager profile..."
                    );
                    match helper_prepare_direct_link(&interface_name).await {
                        Ok(()) => println!("Done."),
                        Err(e) => {
                            eprintln!("Failed to repair via cabledesk-helper: {e}");
                            eprintln!(
                                "Is cabledesk-helper.service running? Polkit may prompt for auth."
                            );
                            return std::process::ExitCode::FAILURE;
                        }
                    }
                }
                Ok(None) => {
                    println!("No direct-link interface is currently present; nothing to repair.");
                }
                Err(e) => {
                    eprintln!("Failed to detect the direct-link interface: {e}");
                    return std::process::ExitCode::FAILURE;
                }
            }
        }
        Command::Devices => not_implemented("devices"),
        Command::Discover => not_implemented("discover"),
        Command::Pair => not_implemented("pair"),
        Command::Connect { .. } => not_implemented("connect"),
        Command::Disconnect => not_implemented("disconnect"),
        Command::Forget { .. } => not_implemented("forget"),
        Command::Logs => not_implemented("logs"),
        Command::Reset => not_implemented("reset"),
        Command::Purge => not_implemented("purge"),
        #[cfg(feature = "simulation")]
        Command::Simulate { event } => {
            return run_simulate(event, cli.json);
        }
    }

    std::process::ExitCode::SUCCESS
}

#[cfg(feature = "simulation")]
fn run_simulate(event: SimulateEvent, json: bool) -> std::process::ExitCode {
    use cabledesk_core::error::UserFacingError;
    use cabledesk_network::simulation::{SimulateCommand, SimulatedWorld};

    let path = std::env::temp_dir().join("cabledesk-sim-world.json");
    let mut world = load_sim_world(&path);

    let result = match event {
        SimulateEvent::Demo => {
            world = SimulatedWorld::new();
            let steps = [
                SimulateCommand::Usb4Controller,
                SimulateCommand::CableInserted,
                SimulateCommand::DirectInterfaceAdded,
                SimulateCommand::PeerDiscovered,
                SimulateCommand::PeerTrusted,
                SimulateCommand::StreamStarted,
                SimulateCommand::CableRemoved,
            ];
            for step in steps {
                match world.apply(step) {
                    Ok(msg) => println!("ok  {step:?}: {msg}"),
                    Err(e) => {
                        eprintln!("err {step:?}: {}", UserFacingError::from(&e).headline);
                        return std::process::ExitCode::FAILURE;
                    }
                }
            }
            let mut mid = SimulatedWorld::new();
            let _ = mid.apply(SimulateCommand::Usb4Controller);
            let _ = mid.apply(SimulateCommand::CableInserted);
            let _ = mid.apply(SimulateCommand::DirectInterfaceAdded);
            match mid.apply(SimulateCommand::PeerOnWifi) {
                Err(e) => {
                    println!(
                        "ok  PeerOnWifi rejected: {}",
                        UserFacingError::from(&e).headline
                    );
                }
                Ok(_) => {
                    eprintln!("err PeerOnWifi was incorrectly accepted");
                    return std::process::ExitCode::FAILURE;
                }
            }
            println!(
                "demo complete — final state {:?} (mock_stream_active={})",
                world.state.current(),
                world.mock_stream_active
            );
            save_sim_world(&path, &world);
            return std::process::ExitCode::SUCCESS;
        }
        SimulateEvent::Status => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&world.to_snapshot()).unwrap()
                );
            } else {
                println!("state: {:?}", world.state.current());
                println!("cable_inserted: {}", world.cable_inserted);
                println!("interface: {:?}", world.interface_name);
                println!("mock_stream_active: {}", world.mock_stream_active);
            }
            return std::process::ExitCode::SUCCESS;
        }
        SimulateEvent::Usb4Controller => world.apply(SimulateCommand::Usb4Controller),
        SimulateEvent::CableInserted => world.apply(SimulateCommand::CableInserted),
        SimulateEvent::CableRemoved => world.apply(SimulateCommand::CableRemoved),
        SimulateEvent::DirectInterfaceAdded => world.apply(SimulateCommand::DirectInterfaceAdded),
        SimulateEvent::DirectInterfaceRemoved => {
            world.apply(SimulateCommand::DirectInterfaceRemoved)
        }
        SimulateEvent::PeerDiscovered => world.apply(SimulateCommand::PeerDiscovered),
        SimulateEvent::PeerOnWifi => world.apply(SimulateCommand::PeerOnWifi),
        SimulateEvent::PeerOnEthernet => world.apply(SimulateCommand::PeerOnEthernet),
        SimulateEvent::PeerTrusted => world.apply(SimulateCommand::PeerTrusted),
        SimulateEvent::IdentityMismatch => world.apply(SimulateCommand::IdentityMismatch),
        SimulateEvent::Charging => world.apply(SimulateCommand::Charging),
        SimulateEvent::Discharging => world.apply(SimulateCommand::Discharging),
        SimulateEvent::StreamStarted => world.apply(SimulateCommand::StreamStarted),
        SimulateEvent::StreamFailed => world.apply(SimulateCommand::StreamFailed),
    };

    match result {
        Ok(msg) => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "ok": true,
                        "message": msg,
                        "snapshot": world.to_snapshot(),
                    })
                );
            } else {
                println!("{msg}");
                println!("state: {:?}", world.state.current());
            }
            save_sim_world(&path, &world);
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            let uf = UserFacingError::from(&e);
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "ok": false,
                        "headline": uf.headline,
                        "detail": uf.detail,
                    })
                );
            } else {
                eprintln!("{}", uf.headline);
                if let Some(detail) = uf.detail {
                    eprintln!("{detail}");
                }
            }
            save_sim_world(&path, &world);
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(feature = "simulation")]
fn load_sim_world(path: &std::path::Path) -> cabledesk_network::simulation::SimulatedWorld {
    use cabledesk_network::simulation::{SimSnapshot, SimulatedWorld};
    match std::fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str::<SimSnapshot>(&text) {
            Ok(snap) => SimulatedWorld::from_snapshot(snap),
            Err(_) => SimulatedWorld::new(),
        },
        Err(_) => SimulatedWorld::new(),
    }
}

#[cfg(feature = "simulation")]
fn save_sim_world(path: &std::path::Path, world: &cabledesk_network::simulation::SimulatedWorld) {
    if let Ok(text) = serde_json::to_string_pretty(&world.to_snapshot()) {
        let _ = std::fs::write(path, text);
    }
}
