/*
 * Emboar OS - Device Manager Daemon (udevd)
 * 
 * Responsibilities:
 *   1. Monitor device hotplug events from kernel
 *   2. Create/remove device nodes in /dev/
 *   3. Apply device permissions and ownership
 *   4. Load device drivers as needed
 *   5. Manage device naming and symlinks
 * 
 * File: src/services/src/udevd.rs
 */

use anyhow::Result;
use std::collections::HashMap;
use chrono::Local;

/* =========================================================================
 * DEVICE TYPES
 * ======================================================================= */

#[derive(Debug, Clone)]
enum DeviceType {
    BlockDevice,
    CharacterDevice,
    NetworkInterface,
    InputDevice,
    Other,
}

impl DeviceType {
    fn name(&self) -> &'static str {
        match self {
            DeviceType::BlockDevice => "block",
            DeviceType::CharacterDevice => "char",
            DeviceType::NetworkInterface => "net",
            DeviceType::InputDevice => "input",
            DeviceType::Other => "other",
        }
    }
}

/* =========================================================================
 * DEVICE RULE
 * ======================================================================= */

#[derive(Debug, Clone)]
struct DeviceRule {
    device_type: DeviceType,
    subsystem: String,
    match_pattern: String,     /* e.g., "KERNEL=*sda*" */
    name: Option<String>,      /* Device node name */
    symlink: Vec<String>,      /* Additional symlinks */
    permissions: Option<u32>,  /* File mode */
    owner: Option<String>,     /* Owner UID:GID */
    run_command: Option<String>, /* Command to run on event */
}

/* =========================================================================
 * UDEV DAEMON
 * ======================================================================= */

struct UdevDaemon {
    rules: Vec<DeviceRule>,
    devices: HashMap<String, DeviceInfo>,
}

#[derive(Debug, Clone)]
struct DeviceInfo {
    name: String,
    device_type: DeviceType,
    major: u32,
    minor: u32,
    node_path: String,
    owner_uid: u32,
    owner_gid: u32,
    permissions: u32,
}

impl UdevDaemon {
    /**
     * Create new udev daemon
     */
    fn new() -> Self {
        eprintln!("[udevd] Initializing udev daemon");

        let mut daemon = UdevDaemon {
            rules: Vec::new(),
            devices: HashMap::new(),
        };

        daemon.load_rules();
        eprintln!("[udevd] ✓ Device rules loaded");

        daemon
    }

    /**
     * Load device rules from /etc/udev/rules.d/
     * 
     * Rules format:
     * ACTION=="add", SUBSYSTEM=="block", KERNEL=="sda*", NAME="block/%k"
     * ACTION=="add", SUBSYSTEM=="usb", NAME="usb/%k", MODE="0666"
     */
    fn load_rules(&mut self) {
        eprintln!("[udevd] Loading device rules...");

        // Default rules
        self.rules.push(DeviceRule {
            device_type: DeviceType::BlockDevice,
            subsystem: "block".to_string(),
            match_pattern: "sda*".to_string(),
            name: Some("sda".to_string()),
            symlink: vec![],
            permissions: Some(0o660),
            owner: Some("0:0".to_string()),
            run_command: None,
        });

        self.rules.push(DeviceRule {
            device_type: DeviceType::CharacterDevice,
            subsystem: "input".to_string(),
            match_pattern: "mouse*".to_string(),
            name: Some("input/mouse".to_string()),
            symlink: vec!["mouse".to_string()],
            permissions: Some(0o644),
            owner: Some("0:0".to_string()),
            run_command: None,
        });

        self.rules.push(DeviceRule {
            device_type: DeviceType::NetworkInterface,
            subsystem: "net".to_string(),
            match_pattern: "eth*".to_string(),
            name: Some("eth0".to_string()),
            symlink: vec![],
            permissions: Some(0o644),
            owner: Some("0:0".to_string()),
            run_command: None,
        });

        eprintln!("[udevd] ✓ Loaded {} device rules", self.rules.len());
    }

    /**
     * Start udev daemon
     */
    fn start(&mut self) -> Result<()> {  // Change to &mut self
        eprintln!("[udevd] Starting udev daemon");
        eprintln!("[udevd] Scanning existing devices...");

        self.scan_devices()?;

        eprintln!("[udevd] ✓ Device scan complete");
        eprintln!("[udevd] Listening for hotplug events...");
        eprintln!("[udevd] Ready");

        Ok(())
    }

    /**
     * Scan /sys for existing devices
     */
    fn scan_devices(&mut self) -> Result<()> {
        let device_files = vec![
            ("/dev/null", DeviceType::CharacterDevice, 1, 3),
            ("/dev/zero", DeviceType::CharacterDevice, 1, 5),
            ("/dev/random", DeviceType::CharacterDevice, 1, 8),
            ("/dev/urandom", DeviceType::CharacterDevice, 1, 9),
            ("/dev/tty", DeviceType::CharacterDevice, 5, 0),
            ("/dev/console", DeviceType::CharacterDevice, 5, 1),
            ("/dev/sda", DeviceType::BlockDevice, 8, 0),
            ("/dev/sda1", DeviceType::BlockDevice, 8, 1),
        ];

        for (path, dev_type, major, minor) in device_files {
            eprintln!("[udevd] ✓ Device node: {} ({}/{})", path, major, minor);
            
            // Store the device
            let device_info = DeviceInfo {
                name: path.to_string(),
                device_type: dev_type.clone(),
                major,
                minor,
                node_path: path.to_string(),
                owner_uid: 0,
                owner_gid: 0,
                permissions: 0o644,
            };
            
            self.devices.insert(path.to_string(), device_info);
        }

        Ok(())
    }

    /**
     * Handle device hotplug event
     */
    fn on_device_event(&mut self, action: &str, device_path: &str) -> Result<()> {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");

        eprintln!("[udevd] {} Device: {} at {}", action, device_path, timestamp);

        // Find matching rules
        let matching_rules: Vec<_> = self.rules.iter()
            .filter(|r| self.matches_rule(device_path, r))
            .cloned()
            .collect();

        if matching_rules.is_empty() {
            eprintln!("[udevd] ⚠ No rule matched for device: {}", device_path);
            return Ok(());
        }

        // Now handle each rule (mutable operations)
        for rule in matching_rules {
            match action {
                "add" => self.handle_device_add(device_path, &rule)?,
                "remove" => self.handle_device_remove(device_path, &rule)?,
                _ => eprintln!("[udevd] Unknown action: {}", action),
            }
        }

        Ok(())
    }

    /**
     * Handle device addition
     */
    fn handle_device_add(&mut self, device_path: &str, rule: &DeviceRule) -> Result<()> {
        eprintln!("[udevd] Adding device: {}", device_path);
        eprintln!("[udevd] Creating device node at /dev/{:?}", rule.name);

        if let Some(owner) = &rule.owner {
            eprintln!("[udevd] Setting ownership: {}", owner);
        }

        if let Some(perms) = rule.permissions {
            eprintln!("[udevd] Setting permissions: {:o}", perms);
        }

        if let Some(cmd) = &rule.run_command {
            eprintln!("[udevd] Running command: {}", cmd);
        }

        // Store device info
        let device_info = DeviceInfo {
            name: rule.name.clone().unwrap_or_default(),
            device_type: rule.device_type.clone(),
            major: 0,
            minor: 0,
            node_path: format!("/dev/{}", rule.name.as_ref().unwrap_or(&"unknown".to_string())),
            owner_uid: 0,
            owner_gid: 0,
            permissions: rule.permissions.unwrap_or(0o644),
        };

        self.devices.insert(device_path.to_string(), device_info);
        eprintln!("[udevd] ✓ Device added");

        Ok(())
    }

    /**
     * Handle device removal
     */
    fn handle_device_remove(&mut self, device_path: &str, rule: &DeviceRule) -> Result<()> {
        eprintln!("[udevd] Removing device: {}", device_path);
        eprintln!("[udevd] Deleting device node: /dev/{:?}", rule.name);

        self.devices.remove(device_path);
        eprintln!("[udevd] ✓ Device removed");

        Ok(())
    }

    /**
     * Check if device matches rule
     */
    fn matches_rule(&self, device_path: &str, rule: &DeviceRule) -> bool {
        // Simple pattern matching
        device_path.contains(&rule.match_pattern)
    }
}

/* =========================================================================
 * MAIN ENTRY POINT
 * ======================================================================= */

fn main() -> Result<()> {
    eprintln!("");
    eprintln!("========================================");
    eprintln!("Emboar OS Device Manager Daemon");
    eprintln!("========================================");
    eprintln!("");

    let mut daemon = UdevDaemon::new();
    eprintln!("");

    daemon.start()?;

    // Simulate hotplug
    eprintln!("");
    let _ = daemon.on_device_event("add", "pci0000:00/0000:00:01.0");
    let _ = daemon.on_device_event("add", "pci0000:00/0000:00:02.0");

    eprintln!("");
    eprintln!("[udevd] Active devices: {}", daemon.devices.len());
    eprintln!("[udevd] Daemon running...");

    // In production, would continue running and listening
    // use std::thread;
    // use std::time::Duration;
    // loop {
    //     thread::sleep(Duration::from_secs(1));
    // }

    Ok(())
}
