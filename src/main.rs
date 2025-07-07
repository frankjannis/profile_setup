use std::{io::Write, path::Path};

fn main() {
    println!("Starting Profile Setup");

    // let home = std::env::var("HOME").unwrap();
    // let home = Path::new(&home);
    let home = Path::new("/home/jannis");
    println!("Home at: {home:?}");
    ask_confirmation(|| {});

    let output = std::process::Command::new("pacman")
        .arg("-Qi")
        .arg("yay")
        .output().unwrap();
    if !output.status.success() {
        println!("Installing yay");
        let output = std::process::Command::new("pacman")
            .arg("-S")
            .arg("yay")
            .status().unwrap();
        handle_exit_status(output);
    };


    println!("Updating...");
    let output = std::process::Command::new("yay")
        .status().unwrap();
    handle_exit_status(output);


    let install_file = std::fs::read_to_string("install.txt").unwrap();
    let install = install_file.lines()
        .map(|l| remove_comment(l))
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>();
    println!("Installing: {install:#?}");
    ask_confirmation(|| {
        let output = std::process::Command::new("yay")
            .arg("-S")
            .arg("--needed") //do not reinstall up to date packages
            .args(install)
            .status().unwrap();
        handle_exit_status(output);
    });

    let cloning_file = std::fs::read_to_string("clone.txt").unwrap();
    let cloning = cloning_file.lines()
        .map(|l| remove_comment(l))
        .filter(|l| !l.is_empty())
        .map(|l| l.split_once(' ').unwrap())
        .map(|(u, t)| (u.trim(), t.trim()))
        .map(|(u, t)| (u, if t.starts_with("~/") {home.join(&t[2..])} else {t.into()}))
        .collect::<Vec<_>>();
    println!("Cloning:\n{cloning:#?}");
    let tmp_path = Path::new("/tmp/profile_setup/");
    println!("Backups at: {tmp_path:?}");
    ask_confirmation(|| {
        std::fs::create_dir(tmp_path).unwrap();
        for (url, target) in &cloning {
            if target.is_dir() {
                let dir_name = target.file_name().unwrap();
                let output = std::process::Command::new("mv")
                    .arg(target)
                    .arg(tmp_path.join(dir_name))
                    .status().unwrap();
                assert!(output.success());
            };
            let output = std::process::Command::new("git")
                .arg("clone")
                .arg(url)
                .arg(target)
                .status().unwrap();
            handle_exit_status(output);
        }
    });


    println!("Setting up samba shares");
    ask_confirmation(|| {
        let samba_credentials_path = std::path::Path::new("/etc/samba-credentials");
        let mut samba_credentials = std::fs::File::create(samba_credentials_path).unwrap();
        writeln!(samba_credentials, "username=jannis\npassword=").unwrap();
        let output = std::process::Command::new("helix")
            .arg(samba_credentials_path)
            .status().unwrap();
        handle_exit_status(output);
        let output = std::process::Command::new("chmod")
            .arg("600")
            .arg(samba_credentials_path)
            .status().unwrap();
        handle_exit_status(output);
        let fstab_path = std::path::Path::new("/etc/fstab");
        let mut fstab = std::fs::OpenOptions::new()
            .append(true).open(fstab_path).unwrap();
        for share in [
            "video",
            "dropboxReadOnly",
        ] {
            writeln!(fstab, "//172.16.97.200/{share} /mnt/truenas/{share} cifs credentials={},iocharset=utf8,uid=1000,gid=1000,file_mode=0777,dir_mode=0777,nofail 0 0", samba_credentials_path.display()).unwrap();
        };
        let output = std::process::Command::new("helix")
            .arg(fstab_path)
            .status().unwrap();
        handle_exit_status(output);
        let output = std::process::Command::new("systemctl")
            .arg("daemon-reload")
            .status().unwrap();
        handle_exit_status(output);
        let output = std::process::Command::new("mount")
            .arg("-a")
            .status().unwrap();
        handle_exit_status(output);
    });


    println!("Setting up autologin");
    ask_confirmation(|| {
        let output = std::process::Command::new("mkdir")
            .arg("-p")
            .arg("/etc/sddm.conf.d")
            .status().unwrap();
        handle_exit_status(output);
        let mut autologin_conf = std::fs::File::create("/etc/sddm.conf.d/autologin.conf").unwrap();
        writeln!(autologin_conf, "[Autologin]\nUser=jannis\nSession=sway\n").unwrap();
        let output = std::process::Command::new("systemctl")
            .arg("--force")
            .arg("enable")
            .arg("sddm.service")
            .status().unwrap();
        handle_exit_status(output);
    });
}

fn ask_confirmation(f: impl FnOnce() -> ()) {
    print!("Continue? [Y/n/s]: ");
    std::io::stdout().flush().unwrap();

    let mut input = String::new();
    if let Ok(_) = std::io::stdin().read_line(&mut input) {
        let input = input.trim().to_lowercase();
        if input == "y" || input == "yes" || input == "" {
            f();
            return;
        } else if input == "s" || input == "skip" {
            return;
        }
    }

    std::process::exit(0);
}

fn remove_comment(s: &str) -> &str {
    match s.find('#') {
        Some(i) => &s[..i],
        None => s,
    }.trim()
}

const FAILURE: &str = r"
-------------------------------------------------
-------------------------------------------------
-------------------------------------------------
--------------PREVIOUS CMD FAILED----------------
-------------------------------------------------
-------------------------------------------------
-------------------------------------------------
";

fn handle_exit_status(s: std::process::ExitStatus) {
    if !s.success() {
        println!("{FAILURE}");
        ask_confirmation(|| {});
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_comment_test() {
        let line = r"   #   # asd ## s";
        assert!(remove_comment(line).is_empty());
        let line = r"#   # asd ## s";
        assert!(remove_comment(line).is_empty());
        let line = r" abc #   # asd ## s";
        assert_eq!(remove_comment(line), "abc");
    }
}
