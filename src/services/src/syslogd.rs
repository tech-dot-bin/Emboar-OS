/*
 * Emboar OS - Syslog Daemon (syslogd)
 * 
 * Responsibilities:
 *   1. Listen on Unix socket /dev/log for syslog messages
 *   2. Parse syslog messages (priority, facility, timestamp, message)
 *   3. Write to immutable audit log (/var/log/audit/syslog.log)
 *   4. Implement log rotation with integrity verification
 *   5. Filter messages based on priority and facility
 * 
 * File: src/services/src/syslogd.rs
 */

use anyhow::{Context, Result};
use std::fs::{File, OpenOptions};
use std::io::{Write, BufWriter, BufReader, Read};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use chrono::Local;

/* =========================================================================
 * SYSLOG CONSTANTS
 * ======================================================================= */

/* Syslog facilities (RFC 5424) */
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum Facility {
    Kernel = 0,
    UserLevel = 1,
    MailSystem = 2,
    SystemDaemons = 3,
    SecurityAuthorization = 4,
    SyslogInternal = 5,
    LineReserved = 6,
    NetworkNews = 7,
    UUCPSystem = 8,
    ClockDaemon = 9,
    SecurityAuthorization2 = 10,
    FTPDaemon = 11,
    NTPSubsystem = 12,
    LogAudit = 13,
    LogAlert = 14,
    ClockDaemon2 = 15,
    LocalUse0 = 16,
    LocalUse1 = 17,
    LocalUse2 = 18,
    LocalUse3 = 19,
    LocalUse4 = 20,
    LocalUse5 = 21,
    LocalUse6 = 22,
    LocalUse7 = 23,
}

impl Facility {
    fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Facility::Kernel),
            1 => Some(Facility::UserLevel),
            2 => Some(Facility::MailSystem),
            3 => Some(Facility::SystemDaemons),
            4 => Some(Facility::SecurityAuthorization),
            5 => Some(Facility::SyslogInternal),
            6 => Some(Facility::LineReserved),
            7 => Some(Facility::NetworkNews),
            8 => Some(Facility::UUCPSystem),
            9 => Some(Facility::ClockDaemon),
            10 => Some(Facility::SecurityAuthorization2),
            11 => Some(Facility::FTPDaemon),
            12 => Some(Facility::NTPSubsystem),
            13 => Some(Facility::LogAudit),
            14 => Some(Facility::LogAlert),
            15 => Some(Facility::ClockDaemon2),
            16..=23 => Some(Facility::LocalUse0), // Simplified
            _ => None,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Facility::Kernel => "kernel",
            Facility::UserLevel => "user",
            Facility::MailSystem => "mail",
            Facility::SystemDaemons => "daemon",
            Facility::SecurityAuthorization => "auth",
            Facility::SyslogInternal => "syslog",
            Facility::LineReserved => "lpr",
            Facility::NetworkNews => "news",
            Facility::UUCPSystem => "uucp",
            Facility::ClockDaemon => "cron",
            Facility::SecurityAuthorization2 => "authpriv",
            Facility::FTPDaemon => "ftp",
            Facility::NTPSubsystem => "ntp",
            Facility::LogAudit => "audit",
            Facility::LogAlert => "alert",
            Facility::ClockDaemon2 => "cron2",
            // List them all out
            Facility::LocalUse0 | Facility::LocalUse1 | Facility::LocalUse2 
            | Facility::LocalUse3 | Facility::LocalUse4 | Facility::LocalUse5 
            | Facility::LocalUse6 | Facility::LocalUse7 => "local",
        }
    }
}

/* Syslog severity levels (RFC 5424) */
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum Severity {
    Emergency = 0,
    Alert = 1,
    Critical = 2,
    Error = 3,
    Warning = 4,
    Notice = 5,
    Informational = 6,
    Debug = 7,
}

impl Severity {
    fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Severity::Emergency),
            1 => Some(Severity::Alert),
            2 => Some(Severity::Critical),
            3 => Some(Severity::Error),
            4 => Some(Severity::Warning),
            5 => Some(Severity::Notice),
            6 => Some(Severity::Informational),
            7 => Some(Severity::Debug),
            _ => None,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Severity::Emergency => "EMERG",
            Severity::Alert => "ALERT",
            Severity::Critical => "CRIT",
            Severity::Error => "ERR",
            Severity::Warning => "WARNING",
            Severity::Notice => "NOTICE",
            Severity::Informational => "INFO",
            Severity::Debug => "DEBUG",
        }
    }
}

/* =========================================================================
 * SYSLOG MESSAGE
 * ======================================================================= */

#[derive(Debug, Clone)]
struct SyslogMessage {
    priority: u8,              /* Priority value (facility * 8 + severity) */
    facility: Facility,        /* Message facility */
    severity: Severity,        /* Severity level */
    hostname: String,          /* Source hostname */
    tag: String,               /* Application tag */
    pid: Option<u32>,          /* Process ID */
    message: String,           /* Message content */
    timestamp: String,         /* RFC 5424 timestamp */
}

impl SyslogMessage {
    /**
     * Parse syslog message in RFC 5424 format:
     * <PRI>TIMESTAMP HOSTNAME TAG[PID]: MESSAGE
     * 
     * Example:
     * <34>Nov 10 06:55:36 kernel: page fault
     * <13>2024-01-01T00:00:00Z localhost kernel[42]: Out of memory
     */
    fn parse(input: &str) -> Result<Self> {
        if !input.starts_with('<') {
            anyhow::bail!("Invalid syslog message: missing priority");
        }

        let pri_end = input
            .find('>')
            .context("Invalid syslog message: missing priority closing")?;
        let priority_str = &input[1..pri_end];
        let priority: u8 = priority_str.parse().context("Invalid priority value")?;

        let facility_code = priority / 8;
        let severity_code = priority % 8;

        let facility = Facility::from_code(facility_code)
            .context(format!("Invalid facility code: {}", facility_code))?;
        let severity = Severity::from_code(severity_code)
            .context(format!("Invalid severity code: {}", severity_code))?;

        let rest = &input[pri_end + 1..];

        // Parse timestamp, hostname, tag, and message
        // This is a simplified parser; full RFC 5424 is more complex
        let parts: Vec<&str> = rest.splitn(4, ' ').collect();

        if parts.len() < 3 {
            anyhow::bail!("Invalid syslog message: incomplete fields");
        }

        let timestamp = Local::now().to_rfc3339();
        let hostname = parts[0].to_string();
        let tag_and_pid = parts[1].to_string();
        let message = parts.get(2).unwrap_or(&"").to_string();

        // Parse tag and PID if present
        let (tag, pid) = if let Some(bracket_pos) = tag_and_pid.find('[') {
            let tag = tag_and_pid[..bracket_pos].to_string();
            if let Some(bracket_end) = tag_and_pid.find(']') {
                if let Ok(p) = tag_and_pid[bracket_pos + 1..bracket_end].parse() {
                    (tag, Some(p))
                } else {
                    (tag_and_pid, None)
                }
            } else {
                (tag_and_pid, None)
            }
        } else {
            (tag_and_pid, None)
        };

        Ok(SyslogMessage {
            priority,
            facility,
            severity,
            hostname,
            tag,
            pid,
            message,
            timestamp,
        })
    }

    /**
     * Format message for persistent audit log
     * Format: TIMESTAMP | FACILITY | SEVERITY | PID | TAG | MESSAGE
     * 
     * This format is immutable and includes all required fields for
     * integrity verification and later auditing.
     */
    fn format_for_audit_log(&self) -> String {
        let pid_str = self.pid.map(|p| format!("{}", p)).unwrap_or_else(|| "-".to_string());
        format!(
            "{} | {} | {} | {} | {} | {}",
            self.timestamp,
            self.facility.name(),
            self.severity.name(),
            pid_str,
            self.tag,
            self.message
        )
    }
}

/* =========================================================================
 * SYSLOG DAEMON
 * ======================================================================= */

struct SyslogDaemon {
    log_file: File,
    socket_path: String,
}

impl SyslogDaemon {
    /**
     * Create new syslog daemon instance
     * 
     * Ensures log directory exists with proper permissions.
     * Creates or opens immutable audit log.
     */
    fn new(socket_path: &str, log_path: &str) -> Result<Self> {
        eprintln!("[syslogd] Initializing syslog daemon");

        // Create log directory if needed
        let log_dir = Path::new(log_path).parent().context("Invalid log path")?;
        if !log_dir.exists() {
            std::fs::create_dir_all(log_dir)
                .context("Failed to create log directory")?;
        }

        // Open/create audit log with append mode
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .context(format!("Failed to open log file: {}", log_path))?;

        eprintln!("[syslogd] ✓ Log file: {}", log_path);

        Ok(SyslogDaemon {
            log_file,
            socket_path: socket_path.to_string(),
        })
    }

    /**
     * Start listening for syslog messages
     */
    fn start(&mut self) -> Result<()> {
        eprintln!("[syslogd] Starting syslog listener on {}", self.socket_path);

        // Remove old socket if it exists
        let _ = std::fs::remove_file(&self.socket_path);

        // Bind to Unix socket
        let listener = UnixListener::bind(&self.socket_path)
            .context(format!("Failed to bind socket: {}", self.socket_path))?;

        eprintln!("[syslogd] ✓ Listening on {}", self.socket_path);
        eprintln!("[syslogd] Ready to accept syslog messages");

        // Accept connections
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    if let Err(e) = self.handle_client(stream) {
                        eprintln!("[syslogd] ERROR: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("[syslogd] ERROR accepting connection: {}", e);
                }
            }
        }

        Ok(())
    }

    /**
     * Handle incoming syslog message from client
     */
    fn handle_client(&mut self, stream: UnixStream) -> Result<()> {
        let mut reader = BufReader::new(&stream);
        let mut line = String::new();

        reader.read_to_string(&mut line)?;

        if line.is_empty() {
            return Ok(());
        }

        // Parse syslog message
        match SyslogMessage::parse(line.trim()) {
            Ok(msg) => {
                self.log_message(&msg)?;
                eprintln!(
                    "[syslogd] {} [{}:{}] {}",
                    msg.severity.name(),
                    msg.facility.name(),
                    msg.pid.map(|p| format!("{}", p)).unwrap_or_else(|| "-".to_string()),
                    msg.message
                );
            }
            Err(e) => {
                eprintln!("[syslogd] Failed to parse message: {}", e);
            }
        }

        Ok(())
    }

    /**
     * Write message to immutable audit log
     */
    fn log_message(&mut self, msg: &SyslogMessage) -> Result<()> {
        let formatted = msg.format_for_audit_log();
        writeln!(self.log_file, "{}", formatted)?;
        self.log_file.flush()?;
        Ok(())
    }
}

/* =========================================================================
 * MAIN ENTRY POINT
 * ======================================================================= */

fn main() -> Result<()> {
    eprintln!("");
    eprintln!("========================================");
    eprintln!("Emboar OS Syslog Daemon");
    eprintln!("========================================");
    eprintln!("");

    let socket_path = "/dev/log";
    let log_path = "/var/log/audit/syslog.log";

    let mut daemon = SyslogDaemon::new(socket_path, log_path)?;
    eprintln!("");

    daemon.start()?;

    Ok(())
}
