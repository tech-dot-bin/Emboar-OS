# Emboar OS - Command Reference (80+ Commands)

## Overview

This document provides 80+ essential commands for Emboar OS, organized by category. Each command includes:
- **Usage**: Command syntax
- **ELI5**: Simple 2-3 sentence explanation
- **Logic**: Implementation details
- **Example**: Practical usage

---

## File Operations (15 commands)

### 1. **ls** - List Files
**Usage**: `ls [OPTIONS] [PATH]`  
**ELI5**: Shows you all the files and folders in a directory, letting you see their names, sizes, and permissions.  
**Logic**: Reads directory inode, iterates entries, displays with ANSI colors (directories=blue, executables=green).  
**Example**: `ls -la /home` (detailed listing with hidden files)

### 2. **cp** - Copy Files
**Usage**: `cp [OPTIONS] <source> <destination>`  
**ELI5**: Makes a copy of a file or folder, duplicating its contents to a new location with the same data.  
**Logic**: Open source, allocate destination, read/write in 64KB chunks, preserve permissions/timestamps.  
**Example**: `cp -r /home/alice/project /backup/project_copy`

### 3. **mv** - Move/Rename Files
**Usage**: `mv <source> <destination>`  
**ELI5**: Moves a file to a new location or renames it; on the same filesystem, it's fast because it just updates the reference.  
**Logic**: Try `rename()` syscall first (O(1)); fail over to copy+delete if cross-filesystem.  
**Example**: `mv old_filename.txt new_filename.txt`

### 4. **rm** - Delete Files
**Usage**: `rm [OPTIONS] <file>`  
**ELI5**: Permanently deletes a file by removing its directory entry; the space is marked as reusable.  
**Logic**: Unlink inode from directory, decrement link count, free blocks if unreferenced.  
**Example**: `rm -i confidential.txt` (prompt before deletion)

### 5. **mkdir** - Create Directory
**Usage**: `mkdir [OPTIONS] <directory>`  
**ELI5**: Creates a new folder (directory) on disk where you can store files and organize your data.  
**Logic**: Create new inode, initialize directory table, set permissions from umask.  
**Example**: `mkdir -p /home/alice/projects/newproject` (create parent dirs too)

### 6. **touch** - Create/Update File
**Usage**: `touch <file>`  
**ELI5**: Creates an empty file or updates the timestamp of an existing file to the current time.  
**Logic**: If exists, call `utimes()` to update mtime/atime; else create empty file.  
**Example**: `touch completed_task.txt`

### 7. **find** - Search Files
**Usage**: `find <path> [OPTIONS] -name <pattern>`  
**ELI5**: Searches recursively for files matching a pattern, useful for finding lost files deep in folder structure.  
**Logic**: Recursive directory traversal, fnmatch pattern matching, filter by inode properties.  
**Example**: `find /home -name "*.log" -type f`

### 8. **shred** - Secure Delete
**Usage**: `shred [OPTIONS] <file>`  
**ELI5**: Overwrites a file multiple times with random data before deleting it, making recovery nearly impossible.  
**Logic**: Write random data 3 passes (default), then unlink.  
**Example**: `shred -vfz -n 7 crypto_key.txt` (7 passes, verbose, zero final pass)

### 9. **cat** - Display File Contents
**Usage**: `cat [OPTIONS] <file>`  
**ELI5**: Prints the entire contents of a text file to the screen, showing everything at once.  
**Logic**: Open file, read in 64KB chunks, write to stdout without buffering.  
**Example**: `cat /etc/emboar/config`

### 10. **head** - Show First Lines
**Usage**: `head [OPTIONS] <file>`  
**ELI5**: Shows the first 10 lines of a file (or a different number you specify) without loading the whole file.  
**Logic**: Open file, read until reaching line count or EOF, output and close.  
**Example**: `head -20 /var/log/system.log`

### 11. **tail** - Show Last Lines
**Usage**: `tail [OPTIONS] <file>`  
**ELI5**: Shows the last 10 lines of a file, useful for checking recent log entries or end of large files.  
**Logic**: Seek to EOF, read backwards in 4KB chunks until collecting N lines.  
**Example**: `tail -f /var/log/app.log` (follow mode, updates as file grows)

### 12. **diff** - Compare Files
**Usage**: `diff [OPTIONS] <file1> <file2>`  
**ELI5**: Shows the differences between two files, highlighting what lines changed, were added, or removed.  
**Logic**: Read both files, compute LCS (Longest Common Subsequence), output unified or context diff format.  
**Example**: `diff -u original.txt modified.txt`

### 13. **patch** - Apply Changes
**Usage**: `patch [OPTIONS] <file> < <patch_file>`  
**ELI5**: Takes a list of changes (a "patch" file) and applies them to a file automatically.  
**Logic**: Parse unified diff format, validate context lines, apply hunks sequentially.  
**Example**: `patch /etc/config.conf < security_update.patch`

### 14. **tar** - Archive Files
**Usage**: `tar [OPTIONS] <archive_file> [files...]`  
**ELI5**: Bundles multiple files and folders into a single archive file, optionally compressing it for smaller storage.  
**Logic**: Serialize file headers + contents into single stream, optionally pipe through gzip/bzip2.  
**Example**: `tar -czf backup.tar.gz /home/alice/documents`

### 15. **zip** - Compress Archive
**Usage**: `zip [OPTIONS] <archive.zip> <files...>`  
**ELI5**: Creates a compressed ZIP file containing multiple files; useful for sharing or reducing storage space.  
**Logic**: For each file, compute DEFLATE compression, store with central directory index.  
**Example**: `zip -r encrypted_backup.zip /home`

---

## System Information (12 commands)

### 16. **specs** - Hardware Specs
**Usage**: `specs [OPTIONS]`  
**ELI5**: Displays detailed information about your computer's hardware: CPU type, memory size, disk capacity.  
**Logic**: Parse `/proc/cpuinfo`, `/proc/meminfo`, read UEFI/SMBIOS tables, query device drivers.  
**Example**: `specs --verbose` (includes cache info, thermal data)

### 17. **top** - Process Monitor
**Usage**: `top [OPTIONS]`  
**ELI5**: Real-time display of running processes showing CPU and memory usage, updated every second.  
**Logic**: Parse `/proc/<pid>/stat` repeatedly, sort by CPU/memory, render with color highlight.  
**Example**: `top -u alice` (show processes for user alice)

### 18. **prio** - Adjust Process Priority
**Usage**: `prio [OPTIONS] <pid> <priority>`  
**ELI5**: Changes how much CPU time a process gets; negative numbers = higher priority, positive = lower priority.  
**Logic**: Call `setpriority()` syscall; validate range [-20, 19]; update scheduler.  
**Example**: `prio 1234 -5` (boost PID 1234 to priority -5)

### 19. **uptime** - System Runtime
**Usage**: `uptime`  
**ELI5**: Shows how long your computer has been running since the last reboot and current system load average.  
**Logic**: Read `/proc/uptime` for seconds since boot, parse load from `/proc/loadavg`.  
**Example**: Output: `14:32:15 up 45 days, 12:34, 2 users, load average: 0.45, 0.32, 0.28`

### 20. **whoami** - Current User
**Usage**: `whoami`  
**ELI5**: Displays the username of the currently logged-in user, useful for confirming identity in scripts.  
**Logic**: Call `getuid()` syscall, lookup in `/etc/passwd`, return username.  
**Example**: Output: `alice`

### 21. **df** - Disk Space
**Usage**: `df [OPTIONS] [filesystem]`  
**ELI5**: Shows how much disk space is used and available on each mounted filesystem.  
**Logic**: Call `statfs()` syscall for each mount, compute usage percentage, format with units (GB, MB).  
**Example**: `df -h` (human-readable, shows /dev/mapper/emboar_vg-root 85% full)

### 22. **free** - Memory Usage
**Usage**: `free [OPTIONS]`  
**ELI5**: Displays total system memory, how much is used, cached, and available in a table format.  
**Logic**: Parse `/proc/meminfo`, calculate used/buffered/cached, format into columns.  
**Example**: `free -h` (shows: 16GB total, 12GB used, 4GB free)

### 23. **dmesg** - Kernel Messages
**Usage**: `dmesg [OPTIONS]`  
**ELI5**: Shows system messages from the kernel, useful for diagnosing hardware problems or driver errors.  
**Logic**: Read from kernel ring buffer (usually `/dev/kmsg` or via syslog interface).  
**Example**: `dmesg | grep -i error` (find all error messages)

### 24. **lsusb** - List USB Devices
**Usage**: `lsusb [OPTIONS]`  
**ELI5**: Lists all USB devices connected to your system (flash drives, mice, keyboards, etc.).  
**Logic**: Enumerate `/sys/bus/usb/devices/`, parse vendor/product IDs from descriptors.  
**Example**: `lsusb -v` (verbose, shows detailed USB device info)

### 25. **lspci** - List PCI Devices
**Usage**: `lspci [OPTIONS]`  
**ELI5**: Lists all hardware components connected via PCI bus (graphics cards, network cards, etc.).  
**Logic**: Read PCI configuration space at 0xcf8/0xcfc, enumerate all slots/functions.  
**Example**: `lspci -k` (show kernel driver for each device)

### 26. **hwinfo** - Hardware Info
**Usage**: `hwinfo [OPTIONS]`  
**ELI5**: Comprehensive hardware information including BIOS version, CPU temperature, power consumption.  
**Logic**: Query DMI tables, parse ACPI, read thermal zones from `/sys/class/thermal/`.  
**Example**: `hwinfo --temp` (show current CPU/disk temperatures)

### 27. **sysctl** - System Parameters
**Usage**: `sysctl [OPTIONS] <param> [value]`  
**ELI5**: View or change kernel parameters that control system behavior (like network settings or security settings).  
**Logic**: Read/write from `/proc/sys/` or `sysctl_set()` syscall.  
**Example**: `sysctl net.ipv4.ip_forward` (check if IP forwarding enabled)

---

## Security & Encryption (16 commands)

### 28. **encrypt** - Encrypt File
**Usage**: `encrypt [OPTIONS] <file> [--key-vault | --password]`  
**ELI5**: Scrambles a file using a secret key or password so only authorized people can read it.  
**Logic**: Generate random IV, derive key via Argon2id, AES-256-GCM encryption, append IV+auth tag.  
**Example**: `encrypt sensitive.txt --key-vault` (encrypt, store key in secure vault)

### 29. **decrypt** - Decrypt File
**Usage**: `decrypt [OPTIONS] <file.encrypted> [--password]`  
**ELI5**: Unscrambles an encrypted file by verifying its authenticity and reversing the encryption using the correct key.  
**Logic**: Extract IV, derive key from password, verify GCM tag, AES-256-GCM decryption.  
**Example**: `decrypt data.encrypted --password` (prompt for password)

### 30. **hashcheck** - Verify File Integrity
**Usage**: `hashcheck [OPTIONS] <file> [--algorithm=sha512|sha256]`  
**ELI5**: Computes a digital fingerprint of a file; anyone can verify the file hasn't been modified by comparing fingerprints.  
**Logic**: Compute SHA-512/SHA-256, format as hex, optionally write to `.hash` file.  
**Example**: `hashcheck backup.tar.gz > backup.tar.gz.sha512`

### 31. **audit** - Security Audit Log
**Usage**: `audit [OPTIONS] [--follow]`  
**ELI5**: Shows a detailed log of all sensitive system actions (who ran sudo, who accessed encrypted files).  
**Logic**: Tail `/var/log/audit/audit.log`, parse JSON entries, filter by user/command/time.  
**Example**: `audit --follow --filter-user=alice` (show all alice's privileged actions in real-time)

### 32. **firewall** - Network Firewall
**Usage**: `firewall [OPTIONS] [enable|disable|status|add-rule|remove-rule]`  
**ELI5**: Controls which network connections are allowed; you can block or allow specific ports and IP addresses.  
**Logic**: Interface with nftables/iptables kernel module, manage rules in `/etc/firewall/rules.conf`.  
**Example**: `firewall add-rule --incoming --protocol=tcp --port=22 --action=reject`

### 33. **vault** - Key Management
**Usage**: `vault [OPTIONS] [list|create|delete|rotate]`  
**ELI5**: Secure storage for cryptographic keys and passwords; keeps them encrypted and protected from unauthorized access.  
**Logic**: Store encrypted keys in `/var/lib/vault/` (LUKS-protected), access via authenticated socket.  
**Example**: `vault create --key-name=db_password --value=secret123` (store password)

### 34. **wipe** - Secure Memory Wipe
**Usage**: `wipe [OPTIONS] [--memory|--disk|--ram-swap]`  
**ELI5**: Overwrites sensitive data in RAM or disk with random garbage so nobody can recover deleted information.  
**Logic**: Allocate large buffer, fill with `/dev/urandom`, mlock to prevent paging, sleep then free.  
**Example**: `wipe --memory` (clear RAM of secrets before shutdown)

### 35. **secure-delete** - Overwrite Before Delete
**Usage**: `secure-delete [OPTIONS] <file> [--passes=3|7|35]`  
**ELI5**: Safely deletes a file by overwriting it multiple times before removing it from the filesystem.  
**Logic**: Perform N passes of PRNG data, then unlink inode.  
**Example**: `secure-delete config.txt --passes=7` (DoD 5220.22-M standard 7 passes)

### 36. **keygen** - Generate Cryptographic Keys
**Usage**: `keygen [OPTIONS] --type=[rsa|ecdh|aes] --bits=<bits>`  
**ELI5**: Creates new cryptographic keys for encryption, signing, or authentication; stores them securely.  
**Logic**: Generate random seed, use openssl/ring to create key pairs, store private key in vault.  
**Example**: `keygen --type=rsa --bits=4096 --output=/etc/emboar/signing_key`

### 37. **sign** - Create Digital Signature
**Usage**: `sign [OPTIONS] <file> [--key=path] [--output=signature.sig]`  
**ELI5**: Creates a digital signature proving you created or approved a file; anyone can verify it's authentic.  
**Logic**: SHA-512 hash file, RSA-4096 sign hash, DER encode signature.  
**Example**: `sign package.emb --key=/etc/emboar/private_key.pem`

### 38. **verify** - Verify Digital Signature
**Usage**: `verify [OPTIONS] <file> --signature=<sig.file> [--key=public.pem]`  
**ELI5**: Confirms that a file's digital signature is authentic and hasn't been tampered with since signing.  
**Logic**: Extract signature, recompute file hash, RSA verify against public key.  
**Example**: `verify package.emb --signature=package.emb.sig`

### 39. **passwd** - Change Password
**Usage**: `passwd [OPTIONS] [username]`  
**ELI5**: Changes the login password for a user account; requires the old password to confirm your identity.  
**Logic**: Prompt for old password, verify against Argon2id hash, hash new password, update `/etc/emboar/shadow`.  
**Example**: `passwd` (change own password)

### 40. **sudo-list** / **emsudo-list** - List Sudo Permissions
**Usage**: `sudo-list [OPTIONS] [username]`  
**ELI5**: Shows what commands a user is allowed to run with elevated privileges.  
**Logic**: Parse `/etc/emsudo/conf`, filter for requesting user or specified user.  
**Example**: `sudo-list` (show current user's sudoers entry)

### 41. **mfa-setup** - Multi-Factor Authentication
**Usage**: `mfa-setup [OPTIONS] [--totp|--yubikey]`  
**ELI5**: Adds a second authentication factor (like a time-based code or physical key) for extra security.  
**Logic**: Generate TOTP secret, display QR code, validate first codes, store in vault.  
**Example**: `mfa-setup --totp | qr-display` (setup TOTP, display QR)

### 42. **cert-gen** - Generate SSL Certificates
**Usage**: `cert-gen [OPTIONS] --domain=example.com [--self-signed]`  
**ELI5**: Creates a digital certificate for SSH, TLS, or other secure connections to verify identity.  
**Logic**: Generate RSA-2048 keypair, create CSR, self-sign or submit to CA.  
**Example**: `cert-gen --domain=emboar.local --self-signed --days=365`

### 43. **ldap** - LDAP Directory Client
**Usage**: `ldap [OPTIONS] search|modify|add [--filter=...] [--base=...]`  
**ELI5**: Connects to a directory server to look up user information and manage accounts across multiple systems.  
**Logic**: LDAP protocol client, support StartTLS, SASL authentication, async operations.  
**Example**: `ldap search --filter="(uid=alice)" --base="dc=emboar,dc=local"`

---

## Network Commands (13 commands)

### 44. **netstat** - Network Statistics
**Usage**: `netstat [OPTIONS] [-t|-u|-l|-p]`  
**ELI5**: Shows all active network connections and listening ports, useful for detecting suspicious connections.  
**Logic**: Parse `/proc/net/tcp` and `/proc/net/udp`, resolve IP addresses and port names.  
**Example**: `netstat -lnp` (show listening ports with process IDs)

### 45. **ip** - Network Interface Configuration
**Usage**: `ip [OPTIONS] addr|route|link [add|del|show]`  
**ELI5**: Manages network interfaces, IP addresses, and routing tables; similar to legacy `ifconfig` but more powerful.  
**Logic**: Use netlink API to configure network interfaces, parse `rtnetlink` messages.  
**Example**: `ip addr add 192.168.1.100/24 dev eth0`

### 46. **ping** - Test Connectivity
**Usage**: `ping [OPTIONS] <host> [--count=4]`  
**ELI5**: Sends test packets to a remote computer to check if it's online and measure response time.  
**Logic**: Send ICMP echo requests, measure RTT, display statistics.  
**Example**: `ping -c 5 8.8.8.8` (send 5 pings)

### 47. **ssh-emb** - Secure Shell (EmBoar Edition)
**Usage**: `ssh-emb [OPTIONS] [user@]host [command]`  
**ELI5**: Securely connects to a remote computer and lets you run commands as if you were sitting there.  
**Logic**: RSA/ECDH key exchange, AES-256-CTR encryption, OpenSSH-compatible protocol.  
**Example**: `ssh-emb -i ~/.ssh/id_rsa alice@remote.host`

### 48. **curl-p** - Privacy-Aware HTTP Client
**Usage**: `curl-p [OPTIONS] <URL>`  
**ELI5**: Downloads files from the internet like `curl`, but removes identifying headers for privacy.  
**Logic**: HTTP(S) client, strip User-Agent, Referer, randomize order of headers, use Tor if configured.  
**Example**: `curl-p https://example.com/file.zip --output file.zip`

### 49. **trace** - Network Packet Trace
**Usage**: `trace [OPTIONS] <host> [--hops=30]`  
**ELI5**: Shows the path packets take to reach a remote computer, displaying each relay point and response time.  
**Logic**: Send ICMP/UDP probe packets with incrementing TTL, display ICMP responses.  
**Example**: `trace 8.8.8.8` (show route to Google DNS)

### 50. **dns-sec** - DNSSEC Lookup
**Usage**: `dns-sec [OPTIONS] <domain> [--record-type=A|MX|NS]`  
**ELI5**: Looks up a domain name with cryptographic verification to ensure the answer is authentic.  
**Logic**: DNSSEC-enabled resolver, validate RRSIG signatures, check trust chain.  
**Example**: `dns-sec example.com --record-type=A` (verify A record is authentic)

### 51. **nslookup** - DNS Lookup
**Usage**: `nslookup [OPTIONS] <domain> [server]`  
**ELI5**: Translates a domain name like example.com into its IP address.  
**Logic**: DNS client, query specified nameserver, parse DNS response packets.  
**Example**: `nslookup example.com 8.8.8.8`

### 52. **scp-emb** - Secure Copy
**Usage**: `scp-emb [OPTIONS] <source> <destination>`  
**ELI5**: Securely copies files between computers over encrypted SSH connection.  
**Logic**: SSH-based file transfer protocol, supports recursive directory copy.  
**Example**: `scp-emb alice@remote:/data/file.txt ./` (copy from remote to local)

### 53. **sftp-emb** - Secure FTP
**Usage**: `sftp-emb [OPTIONS] [user@]host`  
**ELI5**: Interactive secure file transfer with full directory browsing, like FTP but encrypted.  
**Logic**: SSH subsystem for file transfer, OpenSSH SFTP protocol compatible.  
**Example**: `sftp-emb alice@remote.host` (enter interactive mode)

### 54. **nftables** - Network Filtering
**Usage**: `nftables [OPTIONS] [add|delete|list] [rules]`  
**ELI5**: Advanced network packet filtering; can accept/deny traffic based on complex conditions.  
**Logic**: Interface with Linux nftables kernel subsystem, manage rule chains and sets.  
**Example**: `nftables add rule inet filter input tcp dport 22 accept`

### 55. **whois** - Domain Information
**Usage**: `whois [OPTIONS] <domain|IP>`  
**ELI5**: Looks up public registration information about a domain or IP address.  
**Logic**: Query whois servers, parse responses, display registrant/technical contact info.  
**Example**: `whois example.com`

### 56. **sniff** - Packet Sniffer
**Usage**: `sniff [OPTIONS] [--interface=eth0] [--filter="tcp port 443"]`  
**ELI5**: Captures and displays all network traffic on an interface; useful for network diagnostics.  
**Logic**: Raw socket capture, libpcap-compatible filtering, display with protocol parsing.  
**Example**: `sniff --interface=eth0 --filter="src 192.168.1.1"` (monitor one source)

---

## Process Management (12 commands)

### 57. **ps** - Process List
**Usage**: `ps [OPTIONS] [-e|-u <user>]`  
**ELI5**: Shows all running processes on your system with details like their ID, resource usage, and start time.  
**Logic**: Enumerate `/proc/<pid>/` directories, parse `stat` and `status` files, format output.  
**Example**: `ps aux` (all processes with full details)

### 58. **kill** - Terminate Process
**Usage**: `kill [OPTIONS] <pid> [--signal=TERM|KILL|HUP]`  
**ELI5**: Sends a termination signal to a process; default is graceful (TERM), or force (KILL) if it doesn't respond.  
**Logic**: Call `kill()` syscall with signal number, verify process exists.  
**Example**: `kill -KILL 1234` (force-kill PID 1234)

### 59. **nice** - Run at Lower Priority
**Usage**: `nice [OPTIONS] [--adjustment=10] <command>`  
**ELI5**: Runs a command with reduced priority so it uses less CPU time and doesn't slow down your system.  
**Logic**: Call `setpriority()` to increase nice value, then `exec()` command.  
**Example**: `nice --adjustment=10 tar -czf backup.tar.gz /data`

### 60. **renice** - Change Process Priority
**Usage**: `renice [OPTIONS] <priority> [-p pid|-u user]`  
**ELI5**: Changes the priority of a running process, making it run faster or slower without stopping it.  
**Logic**: Call `setpriority()` syscall with specified nice value and process/user target.  
**Example**: `renice -5 -p 1234` (boost PID 1234 to nice -5)

### 61. **cron-emb** - Task Scheduler
**Usage**: `cron-emb [OPTIONS] add|remove|list|edit`  
**ELI5**: Schedules commands to run automatically at specific times (daily, hourly, weekly, etc.).  
**Logic**: Parse crontab-like syntax, daemon monitors time, executes matching jobs in isolation.  
**Example**: `cron-emb add "0 2 * * * /usr/bin/backup.sh"` (run daily at 2 AM)

### 62. **bg** - Background Process
**Usage**: `bg [OPTIONS] [%job_id]`  
**ELI5**: Resumes a suspended process in the background so it runs without occupying your terminal.  
**Logic**: Send SIGCONT to process group, mark as background in shell job table.  
**Example**: `bg %1` (resume job #1 in background)

### 63. **fg** - Foreground Process
**Usage**: `fg [OPTIONS] [%job_id]`  
**ELI5**: Brings a background process to the foreground; you'll see its output and control it again.  
**Logic**: Send SIGCONT, set terminal I/O to process group, wait for completion.  
**Example**: `fg %1` (bring job #1 to foreground)

### 64. **jobs** - List Jobs
**Usage**: `jobs [OPTIONS] [-l|-s|-r]`  
**ELI5**: Lists all jobs running in the current shell session, showing their status and process ID.  
**Logic**: Display shell job table, check process status via `/proc/<pid>/stat`.  
**Example**: `jobs -l` (show job details with PIDs)

### 65. **nohup** - Immune to Hangup
**Usage**: `nohup [OPTIONS] <command> [args...]`  
**ELI5**: Runs a command that won't stop even if you close your terminal; output goes to a file.  
**Logic**: Ignore SIGHUP, redirect stdout to `nohup.out`, continue in subshell.  
**Example**: `nohup long_running_task.sh &` (runs in background, immune to disconnection)

### 66. **disown** - Orphan Process
**Usage**: `disown [OPTIONS] [%job_id]`  
**ELI5**: Removes a job from the shell's control so closing the shell won't affect it.  
**Logic**: Remove from shell job table, let process become orphan (adopted by init).  
**Example**: `disown %1` (stop tracking job #1)

### 67. **strace** - System Call Trace
**Usage**: `strace [OPTIONS] <command> [args...]`  
**ELI5**: Shows all system calls a program makes, useful for debugging why it's failing or misbehaving.  
**Logic**: Use ptrace() to attach to process, intercept syscalls, display with timestamps.  
**Example**: `strace ls /tmp 2>&1 | grep "open"`

### 68. **systemctl-emb** - Service Manager
**Usage**: `systemctl-emb [OPTIONS] [start|stop|restart|enable|status] <service>`  
**ELI5**: Controls background services; you can start them, stop them, or configure them to auto-start.  
**Logic**: Communicate with init daemon via socket, manage service state files.  
**Example**: `systemctl-emb restart sshd` (restart SSH service)

---

## Development & Scripting (13 commands)

### 69. **echo** - Print Output
**Usage**: `echo [OPTIONS] [string...]`  
**ELI5**: Prints text to the screen; can format with colors, variables, or special characters.  
**Logic**: Write arguments separated by spaces, append newline, support escape sequences.  
**Example**: `echo "Hello world" --color=green`

### 70. **grep** - Text Search
**Usage**: `grep [OPTIONS] <pattern> [file...]`  
**ELI5**: Searches for lines containing a pattern and displays only those matching lines.  
**Logic**: Compile regex, iterate lines, display matches with optional line numbers/colors.  
**Example**: `grep -n "error" /var/log/app.log` (show lines with "error" and line numbers)

### 71. **sed** - Stream Editor
**Usage**: `sed [OPTIONS] '<command>' [file...]`  
**ELI5**: Transforms text by substituting, deleting, or rearranging lines according to patterns.  
**Logic**: Parse sed script, compile regex, apply transformations line-by-line.  
**Example**: `sed 's/oldtext/newtext/g' file.txt` (replace all occurrences)

### 72. **awk** - Text Processing
**Usage**: `awk [OPTIONS] '<program>' [file...]`  
**ELI5**: Processes text line-by-line, splitting fields and allowing calculations or filtering.  
**Logic**: Parse AWK script, tokenize input, execute pattern-action on matching records.  
**Example**: `ps aux | awk '{print $1, $3}' | grep alice` (show user and CPU for alice)

### 73. **nano-emb** - Text Editor
**Usage**: `nano-emb [OPTIONS] <file>`  
**ELI5**: Simple text editor with menus at the bottom; easier than vi/vim for beginners.  
**Logic**: Terminal-based editor using ncurses, support syntax highlighting for code.  
**Example**: `nano-emb /etc/emboar/config`

### 74. **vim-emb** - Advanced Text Editor
**Usage**: `vim-emb [OPTIONS] <file>`  
**ELI5**: Powerful but complex text editor with many keyboard shortcuts; used by advanced developers.  
**Logic**: Modal editor (insert/command modes), full regex support, plugin system.  
**Example**: `vim-emb -c "set number" file.txt` (open with line numbers)

### 75. **gcc-emb** - C Compiler
**Usage**: `gcc-emb [OPTIONS] <file.c> [-o output]`  
**ELI5**: Compiles C code into executable programs; part of the toolchain for building system software.  
**Logic**: Preprocess, compile to assembly, assemble, link with libc and system libraries.  
**Example**: `gcc-emb -O2 -Wall program.c -o program`

### 76. **make** - Build Automation
**Usage**: `make [OPTIONS] [target]`  
**ELI5**: Automatically builds projects by running commands specified in a Makefile; rebuilds only what changed.  
**Logic**: Parse Makefile, track dependencies, execute targets with correct order.  
**Example**: `make clean && make all`

### 77. **python-emb** - Python Interpreter
**Usage**: `python-emb [OPTIONS] <script.py> [args...]`  
**ELI5**: Runs Python scripts; Python is a simple, readable programming language great for scripting.  
**Logic**: Parse Python bytecode, interpret AST with runtime, manage memory/objects.  
**Example**: `python-emb analyze_logs.py /var/log/app.log`

### 78. **cargo-emb** - Rust Build System
**Usage**: `cargo-emb [OPTIONS] build|run|test [--release]`  
**ELI5**: Builds and runs Rust programs; Rust is secure and fast like C but memory-safe.  
**Logic**: Parse Cargo.toml, invoke rustc compiler, manage crate dependencies.  
**Example**: `cargo-emb build --release` (build optimized Rust binary)

### 79. **gdb-emb** - Debugger
**Usage**: `gdb-emb [OPTIONS] <binary> [args...]`  
**ELI5**: Debugger for finding bugs in programs; lets you step through code and examine variables.  
**Logic**: Use ptrace() to control program execution, set breakpoints, display memory/registers.  
**Example**: `gdb-emb ./my_program --args arg1 arg2`

### 80. **git-emb** - Version Control
**Usage**: `git-emb [OPTIONS] [add|commit|push|pull]`  
**ELI5**: Tracks changes to files over time; lets teams collaborate and roll back bad changes.  
**Logic**: Create object database, track commits, manage branches, compute diffs.  
**Example**: `git-emb commit -m "Fix security bug in auth"`

### 81. **valgrind-emb** - Memory Analyzer
**Usage**: `valgrind-emb [OPTIONS] <program> [args...]`  
**ELI5**: Checks programs for memory leaks and invalid memory access; helps catch subtle bugs.  
**Logic**: Instrumentation-based memory tracking, instrument every memory operation.  
**Example**: `valgrind-emb --leak-check=full ./myapp`

---

## Miscellaneous & Shell Features (11+ commands)

### 82. **clear** - Clear Screen
**Usage**: `clear [OPTIONS]`  
**ELI5**: Clears the terminal screen of all previous output and moves cursor to the top.  
**Logic**: Write ANSI escape sequences to clear display.  
**Example**: `clear`

### 83. **exit** - Exit Shell
**Usage**: `exit [OPTIONS] [exit_code]`  
**ELI5**: Closes the current shell session or terminal; you can optionally specify a status code.  
**Logic**: Call `_exit()` syscall with status code.  
**Example**: `exit 0` (exit successfully)

### 84. **history** - Command History
**Usage**: `history [OPTIONS] [--count=20]`  
**ELI5**: Shows previously executed commands so you can see what you (or someone else) ran.  
**Logic**: Read encrypted history file (~/.emshell_history), display with timestamps.  
**Example**: `history | grep ssh` (show all SSH commands run)

### 85. **alias** - Command Shortcut
**Usage**: `alias <name>='<command>'`  
**ELI5**: Creates a nickname for a command so you can type less; e.g., alias `ll='ls -la'`.  
**Logic**: Store in shell environment, expand on subsequent input.  
**Example**: `alias backup='tar -czf backup.tar.gz'`

### 86. **unalias** - Remove Shortcut
**Usage**: `unalias <alias_name>`  
**ELI5**: Removes a previously defined alias so the nickname no longer works.  
**Logic**: Delete from shell alias table.  
**Example**: `unalias backup`

### 87. **type** - Command Type
**Usage**: `type [OPTIONS] <command>`  
**ELI5**: Tells you what a command is: a built-in shell function, an executable file, or an alias.  
**Logic**: Search shell built-ins, alias table, then $PATH; report type and location.  
**Example**: `type ls` (shows: "ls is /bin/ls")

### 88. **which** - Command Path
**Usage**: `which [OPTIONS] <command>`  
**ELI5**: Shows the full path to an executable command so you know exactly which version runs.  
**Logic**: Search directories in $PATH, return first match.  
**Example**: `which python-emb` (shows: "/usr/bin/python-emb")

### 89. **man** - Manual Pages
**Usage**: `man [OPTIONS] <command> [section]`  
**ELI5**: Displays technical documentation for a command with full usage, options, and examples.  
**Logic**: Format manual pages with groff, display with pager (less/more).  
**Example**: `man ls` (show ls manual)

### 90. **doesthisdo** - ELI5 Explanation
**Usage**: `doesthisdo [OPTIONS] <command>`  
**ELI5**: Shows a simple 2-3 sentence explanation of what a command does (no jargon).  
**Logic**: Fetch from `/etc/emboar/docs/doesthisdo/<cmd>.txt`.  
**Example**: `doesthisdo tar` (shows: "Bundles multiple files...")

### 91. **alias --list** - List All Aliases
**Usage**: `alias --list [pattern]`  
**ELI5**: Shows all aliases you've created with their expansions.  
**Logic**: Display shell alias table.  
**Example**: `alias --list | grep backup`

### 92. **source** / **(.)** - Execute Script
**Usage**: `source <file>` or `. <file>`  
**ELI5**: Runs a shell script in the current shell session (not in a subshell), so changes persist.  
**Logic**: Parse file as shell commands, execute in current context, inherit variables.  
**Example**: `source ~/.emshellrc` (load shell configuration)

---

## Package Management (6+ commands)

### 93. **ebm install** - Install Package
**Usage**: `ebm install <package_name> [--from=repository]`  
**ELI5**: Downloads and installs a software package; adds all files and dependencies.  
**Logic**: Query repository metadata, verify signature, download, extract, execute pre-install hooks.  
**Example**: `ebm install apache-emb --from=official-repo`

### 94. **ebm remove** - Uninstall Package
**Usage**: `ebm remove <package_name>`  
**ELI5**: Uninstalls a package and cleans up files it created.  
**Logic**: Execute pre-remove hooks, unlink files, execute post-remove hooks, update package database.  
**Example**: `ebm remove apache-emb`

### 95. **ebm update** - Update Packages
**Usage**: `ebm update [package_name]`  
**ELI5**: Downloads and installs newer versions of installed packages.  
**Logic**: Check repository for updates, verify signatures, install with dependency resolution.  
**Example**: `ebm update` (update all packages)

### 96. **ebm list** - List Installed
**Usage**: `ebm list [OPTIONS] [--installed|--available]`  
**ELI5**: Shows installed packages and their versions; optionally shows available packages in repository.  
**Logic**: Read package database, query repository metadata, format into table.  
**Example**: `ebm list --installed` (show what's installed)

### 97. **ebm search** - Search Packages
**Usage**: `ebm search [OPTIONS] <query>`  
**ELI5**: Searches repository for packages matching a keyword; useful for finding new software.  
**Logic**: Query repository API, match by name/description, return results with versions.  
**Example**: `ebm search "web server"`

### 98. **ebm info** - Package Details
**Usage**: `ebm info <package_name>`  
**ELI5**: Shows detailed information about a package: version, size, dependencies, author.  
**Logic**: Display metadata.json from .emb file or repository.  
**Example**: `ebm info apache-emb`

---

## Additional Advanced Commands (Optional Expansion)

### 99. **debug** - Kernel Debugger
**Usage**: `debug [OPTIONS] [kdb|kdump]`  
**ELI5**: Connects to kernel debugger for deep system inspection when developing kernel code.  
**Logic**: Enable KDB over serial port or network, interact with kernel symbol table.  
**Example**: `debug kdb` (enter kernel debugger)

### 100. **reboot** - Restart System
**Usage**: `reboot [OPTIONS]`  
**ELI5**: Restarts the computer immediately (with optional delay).  
**Logic**: Call `reboot()` syscall if privileged, or schedule with init daemon.  
**Example**: `reboot --delay=60` (reboot after 60 seconds)

### 101. **halt** - Shutdown System
**Usage**: `halt [OPTIONS]`  
**ELI5**: Powers down the computer safely; triggers shutdown sequence and memory wipe.  
**Logic**: Call `shutdown()` syscall, trigger RAM wipe process, cut power (ACPI).  
**Example**: `halt --force` (immediate shutdown)

### 102. **date** - Current DateTime
**Usage**: `date [OPTIONS] [+format]`  
**ELI5**: Shows the current date and time; supports custom formatting.  
**Logic**: Get current time via `clock_gettime()`, format with strftime.  
**Example**: `date +"%Y-%m-%d %H:%M:%S"`

### 103. **time** - Measure Execution Time
**Usage**: `time [OPTIONS] <command>`  
**ELI5**: Runs a command and measures how long it took in real time, CPU time, and memory.  
**Logic**: Fork process, call getrusage() after completion, calculate deltas.  
**Example**: `time tar -czf backup.tar.gz /home` (show duration)

---

## Summary

**Total Commands**: 100+  
**Categories**: 9 major (Files, System, Security, Network, Process, Development, Misc, Package, Advanced)  
**Common Features**:
- Every command supports `--help`
- Every command supports `man <cmd>`
- Every command supports `doesthisdo <cmd>` for ELI5 explanation
- Error exit codes documented (0=success, >0=failure with specific codes)
- Logging integration for audit trail

---

*Emboar OS Command Reference v1.0*
