//! Pure parsing shared by the hardware collectors.
//!
//! These run on every platform and are tested with fixed inputs, which is the
//! only way the Linux and Windows parsing paths can be exercised here.

/// A PCI vendor name from any identifier that contains the vendor id: the
/// kernel's `1002:73ff`, or Windows' `PCI\VEN_10DE&DEV_2504&…`.
pub fn pci_vendor(id: &str) -> Option<&'static str> {
    let tail = id.find("VEN_").map_or(id, |at| &id[at + 4..]);
    let vendor = tail.get(..4)?.to_ascii_uppercase();
    match vendor.as_str() {
        "10DE" => Some("NVIDIA"),
        "1002" => Some("AMD"),
        "8086" => Some("Intel"),
        "1AF4" | "1B36" => Some("Red Hat"),
        "15AD" => Some("VMware"),
        _ => None,
    }
}

/// Installed packages in a `dpkg` status file. A removed-but-unpurged package
/// stays in the file with a different status, and a package installed for two
/// architectures appears twice, which is two installed packages.
pub fn dpkg_installed(status: &str) -> u64 {
    status
        .lines()
        .filter(|line| line.trim() == "Status: install ok installed")
        .count() as u64
}

/// The model line of an NVIDIA procfs `information` file.
pub fn nvidia_model(information: &str) -> Option<String> {
    information
        .lines()
        .find_map(|line| line.strip_prefix("Model:"))
        .map(|model| model.trim().to_owned())
        .filter(|model| !model.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_ids_are_read_from_both_identifier_shapes() {
        assert_eq!(pci_vendor("1002:73ff"), Some("AMD"));
        assert_eq!(pci_vendor("10de:2504"), Some("NVIDIA"));
        assert_eq!(
            pci_vendor(r"PCI\VEN_8086&DEV_46A6&SUBSYS_00000000"),
            Some("Intel")
        );
        assert_eq!(pci_vendor("80ee:beef"), None);
        assert_eq!(pci_vendor(""), None);
        assert_eq!(pci_vendor("VEN_"), None);
    }

    #[test]
    fn only_really_installed_packages_are_counted() {
        let status = "\
Package: a
Status: install ok installed

Package: b
Status: deinstall ok config-files

Package: c
Status: install ok installed
";
        assert_eq!(dpkg_installed(status), 2);
        assert_eq!(dpkg_installed(""), 0);
        // Leading whitespace is tolerated; a near miss is not a match.
        assert_eq!(
            dpkg_installed("  Status: install ok installed\nStatus: install ok installed-extras"),
            1
        );
    }

    #[test]
    fn the_nvidia_model_line_is_extracted_or_absent() {
        let information = "Model:           NVIDIA GeForce RTX 3060\nIRQ:   100\n";
        assert_eq!(
            nvidia_model(information).as_deref(),
            Some("NVIDIA GeForce RTX 3060")
        );
        assert_eq!(nvidia_model("Model:\n"), None);
        assert_eq!(nvidia_model("IRQ: 1\n"), None);
    }
}
