// Integration tests for Emboar OS

#[cfg(test)]
mod tests {
    use std::process::Command;

    #[test]
    fn test_kernel_builds() {
        let output = Command::new("cargo")
            .args(["build", "--release", "--package", "emboar-kernel"])
            .output()
            .expect("Failed to build kernel");

        assert!(output.status.success(), 
                "Kernel build failed: {}", 
                String::from_utf8_lossy(&output.stderr));
    }

    #[test]
    fn test_all_services_build() {
        let services = vec!["auditd", "syslogd", "udevd"];
        
        for service in services {
            let output = Command::new("cargo")
                .args(["build", "--release", "--package", "emboar-services", "--bin", service])
                .output()
                .unwrap_or_else(|_| panic!("Failed to build {}", service));

            assert!(output.status.success(), 
                    "{} build failed: {}", 
                    service, 
                    String::from_utf8_lossy(&output.stderr));
        }
    }

    #[test]
    fn test_crypto_library() {
        let output = Command::new("cargo")
            .args(["test", "--package", "emboar-libs", "--", "--nocapture"])
            .output()
            .expect("Failed to run crypto tests");

        assert!(output.status.success(), 
                "Crypto tests failed: {}", 
                String::from_utf8_lossy(&output.stderr));
    }

}

