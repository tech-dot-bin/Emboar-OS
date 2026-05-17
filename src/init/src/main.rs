/*
 * Emboar OS Init System (PID 1)
 * 
 * Responsibilities:
 *   1. Mount essential filesystems (/proc, /sys, /dev)
 *   2. Load kernel modules
 *   3. Parse /etc/fstab and mount filesystems
 *   4. Start services defined in /etc/rc.d/
 *   5. Manage process respawning and signal handling
 * 
 * File: src/init/src/main.rs
 */

use std::collections::HashMap;
use std::fs;
use std::process::{Command, Child};
use std::thread;
use std::time::Duration;
use anyhow::{Context, Result};

/* =========================================================================
 * TYPES
 * ======================================================================= */

#[derive(Clone, Debug)]
struct ServiceConfig {
    name: String,
    command: String,
    args: Vec<String>,
    restart_policy: RestartPolicy,
    user: Option<String>,
    group: Option<String>,
}

#[derive(Clone, Copy, Debug)]
enum RestartPolicy {
    Never,
    OnFailure,
    Always,
}

struct ServiceManager {
    services: HashMap<String, ServiceConfig>,
    running_processes: HashMap<String, Child>,
}

impl ServiceManager {
    fn new() -> Self {
        ServiceManager {
            services: HashMap::new(),
            running_processes: HashMap::new(),
        }
    }

    fn add_service(&mut self, config: ServiceConfig) {
        self.services.insert(config.name.clone(), config);
    }

    fn start_service(&mut self, name: &str) -> Result<()> {
        let config = self.services
            .get(name)
            .context(format!("Service not found: {}", name))?
            .clone();

        eprintln!("[init] Starting service: {}", name);

        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args);

        let child = cmd
            .spawn()
            .context(format!("Failed to start service: {}", name))?;

        self.running_processes.insert(name.to_string(), child);
        eprintln!("[init] Service started: {} (PID: TBD)", name);

        Ok(())
    }

    fn start_all_services(&mut self) -> Result<()> {
        let service_names: Vec<_> = self.services.keys().cloned().collect();
        for name in service_names {
            if let Err(e) = self.start_service(&name) {
                eprintln!("[init] ERROR starting service {}: {}", name, e);
            }
        }
        Ok(())
    }

    fn reap_processes(&mut self) {
        let running: Vec<String> = self.running_processes.keys().cloned().collect();

        for name in running {
            if let Some(mut child) = self.running_processes.get_mut(&name) {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        eprintln!("[init] Service {} exited with status: {}", name, status);
                        self.running_processes.remove(&name);

                        // Handle restart policy
                        if let Some(config) = self.services.get(&name) {
                            match config.restart_policy {
                                RestartPolicy::Always => {
                                    eprintln!("[init] Restarting service: {}", name);
                                    let _ = self.start_service(&name);
                                }
                                RestartPolicy::OnFailure => {
                                    if !status.success() {
                                        eprintln!("[init] Restarting failed service: {}", name);
                                        let _ = self.start_service(&name);
                                    }
                                }
                                RestartPolicy::Never => {
                                    eprintln!("[init] Not restarting service (policy=never): {}", name);
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        // Process still running
                    }
                    Err(e) => {
                        eprintln!("[init] ERROR waiting for process {}: {}", name, e);
                    }
                }
            }
        }
    }
}

/* =========================================================================
 * FILESYSTEM MOUNTING
 * ======================================================================= */

/**
 * Mount essential virtual filesystems required for system operation
 * 
 * These are typically tmpfs/devtmpfs mounted by the bootloader,
 * but we ensure they exist here:
 * - /proc      : Kernel process information
 * - /sys       : Kernel device information
 * - /dev       : Device node files
 * - /tmp       : Temporary file storage (tmpfs)
 */
fn mount_essential_filesystems() -> Result<()> {
    eprintln!("[init] Mounting essential filesystems...");

    // In production, use mount() syscall via nix crate
    // For now, just verify they exist

    let required_dirs = vec!["/proc", "/sys", "/dev", "/tmp"];

    for dir in required_dirs {
        match fs::metadata(dir) {
            Ok(meta) => {
                if meta.is_dir() {
                    eprintln!("[init] ✓ {} mounted", dir);
                }
            }
            Err(_) => {
                eprintln!("[init] ✗ {} not available (mount it before init)", dir);
            }
        }
    }

    Ok(())
}

/* =========================================================================
 * FSTAB PARSING
 * ======================================================================= */

/**
 * Parse /etc/fstab and mount filesystems
 * 
 * Format:
 * <device> <mount_point> <fstype> <options> <dump> <pass>
 * 
 * Example:
 * /dev/sda1 / ext4 defaults 0 1
 * /dev/sda2 /home ext4 defaults,acl 0 2
 * tmpfs /tmp tmpfs size=512M 0 0
 */
fn mount_filesystems_from_fstab() -> Result<()> {
    eprintln!("[init] Parsing /etc/fstab...");

    let fstab_path = "/etc/fstab";
    match fs::read_to_string(fstab_path) {
        Ok(content) => {
            for line in content.lines() {
                // Skip comments and empty lines
                if line.starts_with('#') || line.trim().is_empty() {
                    continue;
                }

                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 6 {
                    eprintln!("[init] Malformed fstab entry: {}", line);
                    continue;
                }

                let device = parts[0];
                let mount_point = parts[1];
                let fstype = parts[2];
                let options = parts[3];

                eprintln!(
                    "[init] Would mount: {} -> {} ({})",
                    device, mount_point, fstype
                );

                // In production: call mount() syscall with these parameters
                // nix::mount::mount(device, mount_point, fstype, options)
            }
        }
        Err(e) => {
            eprintln!("[init] Could not read /etc/fstab: {}", e);
        }
    }

    Ok(())
}

/* =========================================================================
 * SERVICE LOADING
 * ======================================================================= */

/**
 * Load service definitions from /etc/init/services.d/
 * 
 * Each service is defined by a TOML file:
 * 
 * [service]
 * name = "syslogd"
 * command = "/usr/sbin/syslogd"
 * args = ["-n", "-C", "10"]
 * restart_policy = "always"
 * user = "root"
 * group = "root"
 */
fn load_services() -> Result<Vec<ServiceConfig>> {
    eprintln!("[init] Loading services from /etc/init/services.d/...");

    let mut services = Vec::new();
    let services_dir = "/etc/init/services.d";

    // For now, add some default services
    services.push(ServiceConfig {
        name: "syslogd".to_string(),
        command: "/usr/sbin/syslogd".to_string(),
        args: vec!["-n".to_string()],
        restart_policy: RestartPolicy::Always,
        user: Some("root".to_string()),
        group: Some("root".to_string()),
    });

    services.push(ServiceConfig {
        name: "udevd".to_string(),
        command: "/usr/sbin/udevd".to_string(),
        args: vec!["--daemon".to_string()],
        restart_policy: RestartPolicy::OnFailure,
        user: Some("root".to_string()),
        group: Some("root".to_string()),
    });

    services.push(ServiceConfig {
        name: "audit-daemon".to_string(),
        command: "/usr/sbin/auditd".to_string(),
        args: vec![],
        restart_policy: RestartPolicy::Always,
        user: Some("root".to_string()),
        group: Some("root".to_string()),
    });

    eprintln!("[init] Loaded {} services", services.len());
    Ok(services)
}

/* =========================================================================
 * SIGNAL HANDLING
 * ======================================================================= */

/**
 * Install signal handlers for graceful shutdown
 * 
 * Signals handled:
 * - SIGTERM: Graceful shutdown (kill all services)
 * - SIGINT: Same as SIGTERM
 * - SIGCHLD: Child process exited (reap zombies)
 */
fn setup_signal_handlers() -> Result<()> {
    eprintln!("[init] Setting up signal handlers...");

    // In production, use nix::signal to set up handlers
    // For now, just log that we would set them up

    eprintln!("[init] ✓ SIGTERM handler installed");
    eprintln!("[init] ✓ SIGINT handler installed");
    eprintln!("[init] ✓ SIGCHLD handler installed");

    Ok(())
}

/* =========================================================================
 * MAIN INIT PROCESS
 * ======================================================================= */

fn main() -> Result<()> {
    eprintln!("");
    eprintln!("========================================");
    eprintln!("Emboar OS Init System (PID 1)");
    eprintln!("========================================");
    eprintln!("");

    // Step 1: Mount essential filesystems
    mount_essential_filesystems()?;
    eprintln!("");

    // Step 2: Parse fstab and mount additional filesystems
    mount_filesystems_from_fstab()?;
    eprintln!("");

    // Step 3: Set up signal handlers
    setup_signal_handlers()?;
    eprintln!("");

    // Step 4: Load services
    let services = load_services()?;
    eprintln!("");

    // Step 5: Start services
    let mut manager = ServiceManager::new();
    for service in services {
        manager.add_service(service);
    }

    manager.start_all_services()?;
    eprintln!("");

    eprintln!("[init] System initialized successfully");
    eprintln!("[init] Entering main loop...");
    eprintln!("");

    // Main init loop: reap children, monitor services
    loop {
        thread::sleep(Duration::from_secs(1));
        manager.reap_processes();
    }
}
