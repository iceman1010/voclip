use self_update::cargo_crate_version;
use std::env;
use std::fs;
use std::process::Command;

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

    let current_exe = env::current_exe()?;
    let backup_path = current_exe.with_extension("bak");
    fs::copy(&current_exe, &backup_path)?;

    let status = self_update::backends::github::Update::configure()
        .repo_owner("iceman1010")
        .repo_name("voclip")
        .bin_name(&bin_name)
        .target(target)
        .current_version(cargo_crate_version!())
        .build()?
        .update()?;

    let test = Command::new(&current_exe).arg("--version").output();

    match test {
        Ok(output) if output.status.success() => {
            let _ = fs::remove_file(&backup_path);
            println!("Update status: `{}`!", status.version());
        }
        _ => {
            eprintln!("Warning: updated binary is incompatible with this system.");
            eprintln!("Restoring previous version...");
            fs::copy(&backup_path, &current_exe)?;
            let _ = fs::remove_file(&backup_path);
            eprintln!("Previous version restored. Consider building from source instead.");
            std::process::exit(1);
        }
    }

    Ok(())
}
