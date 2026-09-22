use anyhow::{Context, Result};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
#[cfg(unix)]
use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
    thread,
    time::Duration,
};

pub struct VmResources {
    pub memory_mb: u32,
    pub cpus: u8,
}

pub struct VmLaunch {
    pub name: String,
    pub firecracker: PathBuf,
    pub kernel: PathBuf,
    pub rootfs: PathBuf,
    pub resources: VmResources,
    pub guest_command: Vec<String>,
    pub app_dir: PathBuf,
}

pub struct RunningVm {
    pub pid: u32,
}

pub struct FirecrackerRuntime;

impl FirecrackerRuntime {
    pub fn launch(launch: &VmLaunch) -> Result<RunningVm> {
        fs::create_dir_all(&launch.app_dir)?;
        let socket = launch.app_dir.join("firecracker.sock");
        let config_path = launch.app_dir.join("firecracker.json");
        let stdout = fs::File::create(launch.app_dir.join("stdout.log"))?;
        let stderr = fs::File::create(launch.app_dir.join("stderr.log"))?;
        let config = json!({
            "boot-source": {
                "kernel_image_path": launch.kernel,
                "boot_args": "console=ttyS0 reboot=k panic=1 pci=off"
            },
            "drives": [{
                "drive_id": "rootfs",
                "path_on_host": launch.rootfs,
                "is_root_device": true,
                "is_read_only": false
            }],
            "machine-config": {
                "vcpu_count": launch.resources.cpus,
                "mem_size_mib": launch.resources.memory_mb,
                "smt": false
            }
        });
        fs::write(&config_path, serde_json::to_vec_pretty(&config)?)?;
        let mut child = Command::new(&launch.firecracker)
            .arg("--api-sock")
            .arg(&socket)
            .arg("--config-file")
            .arg(&config_path)
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
            .with_context(|| {
                format!(
                    "failed to start Firecracker at {}",
                    launch.firecracker.display()
                )
            })?;
        if let Err(error) = Self::start_instance(&socket) {
            let _ = child.kill();
            return Err(error);
        }
        Ok(RunningVm { pid: child.id() })
    }

    fn start_instance(socket: &Path) -> Result<()> {
        #[cfg(unix)]
        {
            for _ in 0..50 {
                if let Ok(mut stream) = UnixStream::connect(socket) {
                    let request = concat!(
                        "PUT /actions HTTP/1.1\r\n",
                        "Host: localhost\r\n",
                        "Content-Type: application/json\r\n",
                        "Content-Length: 31\r\n",
                        "Connection: close\r\n\r\n",
                        "{\"action_type\":\"InstanceStart\"}"
                    );
                    stream.write_all(request.as_bytes())?;
                    let mut response = String::new();
                    stream.read_to_string(&mut response)?;
                    if response.starts_with("HTTP/1.1 204") {
                        return Ok(());
                    }
                    anyhow::bail!("Firecracker rejected VM start: {response}");
                }
                thread::sleep(Duration::from_millis(100));
            }
            anyhow::bail!("timed out waiting for Firecracker API socket")
        }
        #[cfg(not(unix))]
        {
            let _ = socket;
            anyhow::bail!("Firecracker runtime requires a Unix host")
        }
    }

    pub fn stop(pid: u32) -> Result<()> {
        #[cfg(unix)]
        unsafe {
            if libc::kill(pid as i32, libc::SIGTERM) != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
        }
        #[cfg(windows)]
        {
            Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .status()?;
        }
        Ok(())
    }
}
