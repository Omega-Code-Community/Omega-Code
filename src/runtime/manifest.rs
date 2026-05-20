use crate::platform::detect::{Arch, Os, Platform};

pub struct RuntimePackage {
    pub url: &'static str,
    pub checksum: &'static str,
    pub is_sha256: bool,
    pub archive_name: &'static str,
}

pub fn jre_package(platform: Platform) -> RuntimePackage {
    match (platform.os, platform.arch) {
        (Os::Windows, Arch::X64) => RuntimePackage {
            url: "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.9%2B10/OpenJDK21U-jre_x64_windows_hotspot_21.0.9_10.zip",
            checksum: "39c5e23f3ce4d420663afba8ffde28034b72e2b3e240943dc2321bc1f912eef9",
            is_sha256: true,
            archive_name: "jre.zip",
        },

        (Os::Windows, Arch::Arm64) => RuntimePackage {
            url: "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.9%2B10/OpenJDK21U-jre_aarch64_windows_hotspot_21.0.9_10.zip",
            checksum: "418099dc1e6dfe9b374fedbb33a10786964a1c465e5b09c385ecb1a4e2fe883c",
            is_sha256: true,
            archive_name: "jre.zip",
        },

        (Os::Linux, Arch::X64) => RuntimePackage {
            url: "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.9%2B10/OpenJDK21U-jre_x64_linux_hotspot_21.0.9_10.tar.gz",
            checksum: "aeab55d064a1a27a3744b0880b9b414077b4ed2b1790817eea3df60aec946431",
            is_sha256: true,
            archive_name: "jre.tar.gz",
        },

        (Os::Linux, Arch::Arm64) => RuntimePackage {
            url: "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.9%2B10/OpenJDK21U-jre_aarch64_linux_hotspot_21.0.9_10.tar.gz",
            checksum: "7f8c230ba505b418e4288e2b34758a6e4da32470944740e5ba0cfaae02271c22",
            is_sha256: true,
            archive_name: "jre.tar.gz",
        },

        (Os::MacOS, Arch::X64) => RuntimePackage {
            url: "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.9%2B10/OpenJDK21U-jre_x64_mac_hotspot_21.0.9_10.tar.gz",
            checksum: "945abc49249f1e89a2a6a008d70d63dc42b25bbe5e1711ff97aeec70063008d2",
            is_sha256: true,
            archive_name: "jre.tar.gz",
        },

        (Os::MacOS, Arch::Arm64) => RuntimePackage {
            url: "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.9%2B10/OpenJDK21U-jre_aarch64_mac_hotspot_21.0.9_10.tar.gz",
            checksum: "1f7f6506b598e85d7d8ff8b36563d98657d2d81b16bfca3cd242d7906cfbd11b",
            is_sha256: true,
            archive_name: "jre.tar.gz",
        },
    }
}

pub fn python_package(platform: Platform) -> RuntimePackage {
    match (platform.os, platform.arch) {
        (Os::Windows, Arch::X64) => RuntimePackage {
            url: "https://www.python.org/ftp/python/3.11.9/python-3.11.9-embed-amd64.zip",
            checksum: "6d9aa08531d48fcc261ba667e2df17c4",
            is_sha256: false,
            archive_name: "python.zip",
        },

        (Os::Windows, Arch::Arm64) => RuntimePackage {
            url: "https://www.python.org/ftp/python/3.11.9/python-3.11.9-embed-arm64.zip",
            checksum: "8611b6aa35483ab1c61d45e0d9f2de0d",
            is_sha256: false,
            archive_name: "python.zip",
        },

        (Os::Linux, Arch::X64) => RuntimePackage {
            url: "https://github.com/astral-sh/python-build-standalone/releases/download/20250626/cpython-3.11.13+20250626-x86_64-unknown-linux-gnu-install_only.tar.gz",
            checksum: "3bf2066dd96c86aacfd2b016699667e2a0bcb97ec63fb7791e230c7deda0a90f",
            is_sha256: true,
            archive_name: "python.tar.gz",
        },

        (Os::Linux, Arch::Arm64) => RuntimePackage {
            url: "https://github.com/astral-sh/python-build-standalone/releases/download/20250626/cpython-3.11.13+20250626-aarch64-unknown-linux-gnu-install_only.tar.gz",
            checksum: "7b65d27357f3101576e6a30ebfd1462d794a02be2c2068abfb147480a8a494df",
            is_sha256: true,
            archive_name: "python.tar.gz",
        },

        (Os::MacOS, Arch::X64) => RuntimePackage {
            url: "https://github.com/astral-sh/python-build-standalone/releases/download/20250626/cpython-3.11.13+20250626-x86_64-apple-darwin-install_only.tar.gz",
            checksum: "82a709c1d8220f26a0d3b35a566047e1fed0a41d39013953720ce92dac527a3d",
            is_sha256: true,
            archive_name: "python.tar.gz",
        },

        (Os::MacOS, Arch::Arm64) => RuntimePackage {
            url: "https://github.com/astral-sh/python-build-standalone/releases/download/20250626/cpython-3.11.13+20250626-aarch64-apple-darwin-install_only.tar.gz",
            checksum: "fd3bc3b011b49fc66ccc85099c7ec646242f5802f7759e5c26f06a9d41476ae2",
            is_sha256: true,
            archive_name: "python.tar.gz",
        },
    }
}