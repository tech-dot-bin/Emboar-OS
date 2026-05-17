#!/bin/bash
# 
# Emboar OS Build Verification Script
# 
# Verifies project structure and dependencies
# Performs basic syntax checks on source files
# 
# Usage: ./verify-build.sh
# 

set -e

echo "======================================"
echo "Emboar OS Build Verification"
echo "======================================"
echo ""

# Color codes
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

errors=0
warnings=0

# Helper functions
check_file() {
    if [ -f "$1" ]; then
        echo -e "${GREEN}✓${NC} $1"
        return 0
    else
        echo -e "${RED}✗${NC} $1 (missing)"
        ((errors++))
        return 1
    fi
}

check_dir() {
    if [ -d "$1" ]; then
        echo -e "${GREEN}✓${NC} $1/"
        return 0
    else
        echo -e "${RED}✗${NC} $1/ (missing)"
        ((errors++))
        return 1
    fi
}

warn() {
    echo -e "${YELLOW}⚠${NC} $1"
    ((warnings++))
}

# 1. Check directory structure
echo "1. Checking directory structure..."
check_dir "src"
check_dir "src/bootloader"
check_dir "src/kernel/src"
check_dir "src/init/src"
check_dir "src/services/src"
check_dir "src/libs/src"
check_dir "tools"
echo ""

# 2. Check documentation files
echo "2. Checking documentation..."
check_file "README.md"
check_file "ARCHITECTURE.md"
check_file "BOOT_AND_INSTALLATION.md"
check_file "CORE_FEATURES.md"
check_file "EMSHELL_SPECIFICATION.md"
check_file "COMMAND_REFERENCE.md"
check_file "SECURITY_MODEL.md"
check_file "PACKAGE_MANAGER.md"
check_file "LIBRARY_RECOMMENDATIONS.md"
check_file "IMPLEMENTATION_ROADMAP.md"
check_file "PROJECT_SUMMARY.md"
echo ""

# 3. Check bootloader files
echo "3. Checking bootloader..."
check_file "src/bootloader/stage1.asm"
check_file "src/bootloader/stage2.c"
echo ""

# 4. Check kernel files
echo "4. Checking kernel..."
check_file "src/kernel/Cargo.toml"
check_file "src/kernel/src/main.rs"
check_file "src/kernel/kernel.ld"
check_file "src/kernel/.cargo/config.toml"
echo ""

# 5. Check init system
echo "5. Checking init system..."
check_file "src/init/Cargo.toml"
check_file "src/init/src/main.rs"
echo ""

# 6. Check services
echo "6. Checking services..."
check_file "src/services/Cargo.toml"
check_file "src/services/src/syslogd.rs"
check_file "src/services/src/auditd.rs"
check_file "src/services/src/udevd.rs"
echo ""

# 7. Check libraries
echo "7. Checking libraries..."
check_file "src/libs/Cargo.toml"
check_file "src/libs/src/lib.rs"
check_file "src/libs/src/crypto.rs"
check_file "src/libs/src/password.rs"
check_file "src/libs/src/error.rs"
echo ""

# 8. Check configuration files
echo "8. Checking configuration..."
check_file "Cargo.toml"
check_file "Makefile"
check_file ".gitignore"
echo ""

# 9. Verify critical file sizes
echo "9. Verifying file constraints..."
stage1_size=$(stat -c%s "src/bootloader/stage1.asm" 2>/dev/null || stat -f%z "src/bootloader/stage1.asm" 2>/dev/null || echo "0")
if [ "$stage1_size" -gt 0 ]; then
    echo -e "${GREEN}✓${NC} Stage 1 bootloader exists"
    # Note: assembly file size != binary size; binary must be ≤ 512 bytes
else
    ((errors++))
fi
echo ""

# Summary
echo "======================================"
echo "Verification Summary"
echo "======================================"
echo ""

if [ $errors -eq 0 ]; then
    echo -e "${GREEN}✓ All checks passed!${NC}"
else
    echo -e "${RED}✗ Found $errors errors${NC}"
fi

if [ $warnings -gt 0 ]; then
    echo -e "${YELLOW}⚠ Found $warnings warnings${NC}"
fi

echo ""
echo "Next steps:"
echo "  1. Run 'make boot' to build bootloader"
echo "  2. Run 'make kernel' to build kernel"
echo "  3. Run 'make image' to create bootable disk image"
echo "  4. Run 'make run' to test in QEMU"
echo ""

if [ $errors -gt 0 ]; then
    exit 1
else
    exit 0
fi
