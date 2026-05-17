# Emboar OS - Package Manager (EBM)

## 1. EBM Architecture

### 1.1 Overview

EBM (Emboar Package Manager) is a secure, cryptographically-verified package manager supporting `.emb` packages with RSA-4096 signature verification.

### 1.2 Package Format

```
myapp.emb (ZIP container with specific structure):
├── metadata.json
├── manifest.sig
├── payload/
│   ├── bin/myapp
│   ├── lib/libmyapp.so
│   ├── etc/myapp.conf
│   └── man/myapp.1
└── scripts/
    ├── pre-install.sh
    ├── post-install.sh
    ├── pre-remove.sh
    └── post-remove.sh
```

### 1.3 Metadata Format

**File**: `metadata.json`

```json
{
    "name": "apache-emb",
    "version": "2.4.52",
    "architecture": "x86_64",
    "os": "emboar",
    "author": "Apache Foundation",
    "license": "Apache 2.0",
    "dependencies": [
        {
            "name": "openssl",
            "version": ">=1.1.1",
            "required": true
        },
        {
            "name": "pcre2",
            "version": ">=10.30",
            "required": false
        }
    ],
    "description": "World's most popular web server",
    "homepage": "https://httpd.apache.org",
    "size_bytes": 1048576,
    "checksum_sha512": "abc123...",
    "signing_key_id": "0x1234567890ABCDEF",
    "signature_algorithm": "RSA-4096",
    "installation_path": "/opt/apache-emb",
    "scripts": {
        "pre_install": true,
        "post_install": true,
        "pre_remove": true,
        "post_remove": false
    }
}
```

---

## 2. Installation Process

### 2.1 Workflow

```
User Input: ebm install apache-emb
    ↓
Query Repository → Fetch metadata
    ↓
Dependency Resolution → Build installation tree
    ↓
Download Package → Verify checksum (SHA-512)
    ↓
Extract package.emb → Verify integrity
    ↓
RSA-4096 Signature Verification
    ↓
Run pre-install hook
    ↓
Copy files to destination
    ↓
Update package database
    ↓
Run post-install hook
    ↓
Success or Rollback on error
```

### 2.2 Implementation (Rust)

```rust
// src/package_manager/installer.rs

use serde_json::Value;
use sha2::{Sha512, Digest};
use std::fs;
use std::path::Path;
use zip::ZipArchive;

pub struct PackageInstaller {
    package_name: String,
    package_path: String,
    metadata: Value,
    public_keys: Vec<Vec<u8>>,
}

impl PackageInstaller {
    pub fn new(name: &str, path: &str) -> Result<Self, String> {
        let metadata = Self::extract_metadata(path)?;
        let public_keys = Self::load_trusted_keys()?;

        Ok(PackageInstaller {
            package_name: name.to_string(),
            package_path: path.to_string(),
            metadata,
            public_keys,
        })
    }

    pub fn install(&self) -> Result<(), String> {
        println!("Installing {}...", self.package_name);

        // Step 1: Verify package integrity
        self.verify_package_integrity()?;

        // Step 2: Verify RSA-4096 signature
        self.verify_rsa_signature()?;

        // Step 3: Resolve dependencies
        self.resolve_dependencies()?;

        // Step 4: Run pre-install hooks
        self.run_pre_install_hooks()?;

        // Step 5: Extract and install files
        self.extract_and_install()?;

        // Step 6: Update package database
        self.update_package_db()?;

        // Step 7: Run post-install hooks
        self.run_post_install_hooks()?;

        println!("Successfully installed {}", self.package_name);
        Ok(())
    }

    fn verify_package_integrity(&self) -> Result<(), String> {
        let metadata_checksum = self.metadata["checksum_sha512"]
            .as_str()
            .ok_or("Missing checksum in metadata")?;

        // Compute SHA-512 of package (excluding signature file)
        let package_data = fs::read(&self.package_path)
            .map_err(|e| e.to_string())?;

        let mut hasher = Sha512::new();
        hasher.update(&package_data);
        let computed_hash = format!("{:x}", hasher.finalize());

        if computed_hash != metadata_checksum {
            return Err("Package checksum verification failed!".to_string());
        }

        Ok(())
    }

    fn verify_rsa_signature(&self) -> Result<(), String> {
        // Extract signature from manifest.sig
        let archive = zip::ZipArchive::new(
            std::fs::File::open(&self.package_path)
                .map_err(|e| e.to_string())?
        ).map_err(|e| e.to_string())?;

        let mut archive = archive;
        let mut sig_file = archive.by_name("manifest.sig")
            .map_err(|e| e.to_string())?;

        let mut signature = Vec::new();
        sig_file.read_to_end(&mut signature)
            .map_err(|e| e.to_string())?;

        // Compute payload hash
        let payload_data = fs::read(&self.package_path)
            .map_err(|e| e.to_string())?;

        let mut hasher = Sha512::new();
        hasher.update(&payload_data);
        let payload_hash = hasher.finalize();

        // Verify against each trusted key
        for public_key in &self.public_keys {
            if self.rsa4096_verify(&payload_hash, &signature, public_key)? {
                return Ok(());
            }
        }

        Err("RSA-4096 signature verification failed!".to_string())
    }

    fn rsa4096_verify(&self, data_hash: &[u8], signature: &[u8], 
                      public_key: &[u8]) -> Result<bool, String> {
        // Use ring crate for RSA verification
        use ring::signature::{self, RSA_PKCS1_2048_8192_SHA512};

        let public_key = signature::UnparsedPublicKey::new(
            &RSA_PKCS1_2048_8192_SHA512,
            public_key
        );

        match public_key.verify(data_hash, signature) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn resolve_dependencies(&self) -> Result<(), String> {
        if let Some(deps) = self.metadata["dependencies"].as_array() {
            for dep in deps {
                let name = dep["name"].as_str().ok_or("Missing dep name")?;
                let version = dep["version"].as_str().ok_or("Missing dep version")?;
                let required = dep["required"].as_bool().unwrap_or(true);

                if is_installed(name, version)? {
                    println!("  ✓ {} {} already installed", name, version);
                } else if required {
                    println!("  Installing dependency: {}", name);
                    // Recursively install dependency
                    self.install_dependency(name, version)?;
                }
            }
        }
        Ok(())
    }

    fn run_pre_install_hooks(&self) -> Result<(), String> {
        if self.metadata["scripts"]["pre_install"].as_bool().unwrap_or(false) {
            println!("Running pre-install hooks...");
            self.run_script("scripts/pre-install.sh")?;
        }
        Ok(())
    }

    fn extract_and_install(&self) -> Result<(), String> {
        let file = fs::File::open(&self.package_path)
            .map_err(|e| e.to_string())?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| e.to_string())?;

        let install_path = self.metadata["installation_path"].as_str()
            .ok_or("Missing installation_path")?;

        println!("Extracting files to {}...", install_path);

        for i in 0..archive.len() {
            let mut file_entry = archive.by_index(i)
                .map_err(|e| e.to_string())?;

            let outpath = Path::new(install_path).join(file_entry.name());

            if file_entry.is_dir() {
                fs::create_dir_all(&outpath)
                    .map_err(|e| e.to_string())?;
            } else {
                if let Some(parent) = outpath.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| e.to_string())?;
                }
                let mut outfile = fs::File::create(&outpath)
                    .map_err(|e| e.to_string())?;
                std::io::copy(&mut file_entry, &mut outfile)
                    .map_err(|e| e.to_string())?;
            }
        }

        Ok(())
    }

    fn update_package_db(&self) -> Result<(), String> {
        let db_path = "/var/lib/ebm/packages.db";
        let mut packages = self.load_package_db(db_path)?;

        packages[&self.package_name] = self.metadata.clone();

        let db_json = serde_json::to_string_pretty(&packages)
            .map_err(|e| e.to_string())?;

        fs::write(db_path, db_json)
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    fn run_post_install_hooks(&self) -> Result<(), String> {
        if self.metadata["scripts"]["post_install"].as_bool().unwrap_or(false) {
            println!("Running post-install hooks...");
            self.run_script("scripts/post-install.sh")?;
        }
        Ok(())
    }

    fn extract_metadata(path: &str) -> Result<Value, String> {
        let file = fs::File::open(path)
            .map_err(|e| e.to_string())?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| e.to_string())?;

        let mut metadata_file = archive.by_name("metadata.json")
            .map_err(|e| e.to_string())?;

        let mut metadata_str = String::new();
        metadata_file.read_to_string(&mut metadata_str)
            .map_err(|e| e.to_string())?;

        serde_json::from_str(&metadata_str)
            .map_err(|e| e.to_string())
    }

    fn load_trusted_keys() -> Result<Vec<Vec<u8>>, String> {
        // Load trusted public keys from /etc/ebm/trusted_keys/
        let keys_dir = "/etc/ebm/trusted_keys";
        let mut keys = Vec::new();

        for entry in fs::read_dir(keys_dir)
            .map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "pub") {
                let key_data = fs::read(&path)
                    .map_err(|e| e.to_string())?;
                keys.push(key_data);
            }
        }

        Ok(keys)
    }
}
```

---

## 3. Package Commands

### 3.1 Installation

```bash
# Install from official repository
ebm install apache-emb

# Install from file
ebm install ./custom-package.emb

# Install with specific version
ebm install apache-emb=2.4.52

# Install multiple packages
ebm install apache-emb openssl-emb pcre2-emb
```

### 3.2 Removal

```bash
# Remove package
ebm remove apache-emb

# Remove with dependencies
ebm remove --with-deps apache-emb

# Force removal (ignore dependencies)
ebm remove --force apache-emb
```

### 3.3 Updates

```bash
# Update single package
ebm update apache-emb

# Update all packages
ebm update

# Show what would be updated
ebm update --dry-run
```

### 3.4 Queries

```bash
# List installed packages
ebm list

# List installed with versions
ebm list -v

# Search for packages
ebm search "web server"

# Show package details
ebm info apache-emb

# Show package dependencies
ebm depends apache-emb

# Check reverse dependencies
ebm rdepends libz
```

### 3.5 Verification

```bash
# Verify package signature
ebm verify-sig ./package.emb

# Verify all installed packages
ebm verify --all

# Fix corrupted installation
ebm repair apache-emb
```

---

## 4. Repository Structure

**Central Repository**: `https://repo.emboar.io/`

```
https://repo.emboar.io/
├── index.json              # All package metadata
├── x86_64/
│   ├── apache-emb/
│   │   ├── 2.4.50/apache-emb-2.4.50.emb
│   │   ├── 2.4.51/apache-emb-2.4.51.emb
│   │   └── 2.4.52/apache-emb-2.4.52.emb
│   ├── openssl-emb/
│   └── ...
└── signatures/
    ├── key-2024.pub        # Public keys for verification
    ├── key-2025.pub
    └── key-2026.pub
```

---

*Emboar OS Package Manager v1.0*
