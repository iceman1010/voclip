use self_update::cargo_crate_version;
use std::env;

pub fn update() -> Result<(), Box<dyn std::error::Error>> {
    let target = match env::consts::OS {
        "linux" => "linux-x86_64",
        "macos" => {
            if env::consts::ARCH == "aarch64" {
                "macos-aarch64"
            } else {
                "macos-x86_64"
            }
        }
        "windows" => "windows-x86_64",
        _ => return Err("Unsupported platform".into()),
    };

    let bin_name = format!("voclip{}", env::consts::EXE_SUFFIX);

    println!("Checking for updates...");

    let status = self_update::backends::github::Update::configure()
        .repo_owner("iceman1010")
        .repo_name("voclip")
        .bin_name(&bin_name)
        .target(target)
        .current_version(cargo_crate_version!())
        .build()?
        .update()?;

    println!("Update status: `{}`!", status.version());
    Ok(())
}
