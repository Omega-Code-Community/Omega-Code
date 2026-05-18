#[derive(Debug, Clone, Copy)]
pub enum Os {
    Windows,
    Linux,
    MacOS,
}

#[derive(Debug, Clone, Copy)]
pub enum Arch {
    X64,
    Arm64,
}

#[derive(Debug, Clone, Copy)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
}

pub fn current_platform() -> Platform {
    let os = match std::env::consts::OS {
        "windows" => Os::Windows,
        "linux" => Os::Linux,
        "macos" => Os::MacOS,
        _ => panic!("unsupported os"),
    };

    let arch = match std::env::consts::ARCH {
        "x86_64" => Arch::X64,
        "aarch64" => Arch::Arm64,
        _ => panic!("unsupported arch"),
    };

    Platform { os, arch }
}