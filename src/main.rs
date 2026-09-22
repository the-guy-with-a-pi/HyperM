mod runtime;
mod state;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use runtime::{FirecrackerRuntime, VmLaunch, VmResources};
use state::{AppRecord, StateStore};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "hyperm", version, about = "PM2-style Firecracker microVM manager")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Start {
        name: String,
        #[arg(long, default_value = "firecracker")]
        firecracker: PathBuf,
        #[arg(long)]
        kernel: PathBuf,
        #[arg(long)]
        rootfs: PathBuf,
        #[arg(long, default_value_t = 512)]
        memory: u32,
        #[arg(long, default_value_t = 1)]
        cpus: u8,
        #[arg(last = true)]
        guest_command: Vec<String>,
    },
    Run {
        program: PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, alias = "memory", default_value_t = 512)]
        ram: u32,
        #[arg(long, default_value_t = 1)]
        cpus: u8,
        #[arg(long, default_value = "firecracker")]
        firecracker: PathBuf,
        #[arg(long, default_value = "/var/lib/hyperm/vmlinux")]
        kernel: PathBuf,
        #[arg(long, default_value = "/var/lib/hyperm/rootfs.ext4")]
        rootfs: PathBuf,
        #[arg(last = true)]
        args: Vec<String>,
    },
    List,
    Stop { name: String },
    Delete { name: String },
    Logs { name: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let home = std::env::var_os("HYPERM_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".hyperm"));
    let store = StateStore::new(home)?;

    match cli.command {
        Command::Start { name, firecracker, kernel, rootfs, memory, cpus, guest_command } => {
            if guest_command.is_empty() {
                bail!("guest command is required after --")
            }
            launch_app(&store, name, firecracker, kernel, rootfs, memory, cpus, guest_command)?;
        }
        Command::Run { program, name, ram, cpus, firecracker, kernel, rootfs, args } => {
            let inferred_name = program
                .file_stem()
                .and_then(|stem| stem.to_str())
                .filter(|stem| !stem.is_empty())
                .unwrap_or("app")
                .to_string();
            let app_name = name.unwrap_or(inferred_name);
            let mut guest_command = vec![program.to_string_lossy().into_owned()];
            guest_command.extend(args);
            launch_app(&store, app_name, firecracker, kernel, rootfs, ram, cpus, guest_command)?;
        }
        Command::List => {
            for app in store.list()? {
                println!("{:<16} {:<8} vm-pid={} guest={}", app.name, app.status, app.pid, app.guest_command.join(" "));
            }
        }
        Command::Stop { name } => {
            let mut app = store.require(&name)?;
            FirecrackerRuntime::stop(app.pid)?;
            app.status = "stopped".into();
            store.update(app)?;
            println!("stopped {name}");
        }
        Command::Delete { name } => {
            let app = store.require(&name)?;
            if app.status == "online" {
                bail!("stop the app before deleting it")
            }
            store.delete(&name)?;
            println!("deleted {name}");
        }
        Command::Logs { name } => {
            let app = store.require(&name)?;
            let output = std::fs::read_to_string(&app.stdout_log).unwrap_or_default();
            let errors = std::fs::read_to_string(&app.stderr_log).unwrap_or_default();
            println!("--- stdout ---\n{output}--- stderr ---\n{errors}");
        }
    }
    Ok(())
}

fn launch_app(
    store: &StateStore,
    name: String,
    firecracker: PathBuf,
    kernel: PathBuf,
    rootfs: PathBuf,
    memory: u32,
    cpus: u8,
    guest_command: Vec<String>,
) -> Result<()> {
    if store.get(&name)?.is_some() {
        bail!("app '{name}' already exists; stop and delete it first")
    }
    let app_dir = store.app_dir(&name)?;
    let launch = VmLaunch {
        name: name.clone(),
        firecracker,
        kernel,
        rootfs,
        resources: VmResources { memory_mb: memory, cpus },
        guest_command,
        app_dir,
    };
    let runtime = FirecrackerRuntime::launch(&launch)?;
    let record = AppRecord::from_launch(&launch, runtime.pid);
    store.insert(record)?;
    println!("started {name} (vm pid {})", runtime.pid);
    Ok(())
}
