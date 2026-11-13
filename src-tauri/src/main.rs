// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;
use is_root::is_root;
use std::fs;
 use rsbash::rash;

fn print_help() {
    println!("slideshow-thing - Mr. Wilson slideshow doohickey");
    println!("");
    println!("Available commands:");
    println!("  ./kioskapp setup - Runs the setup script to set up USB detection");
    println!("  ./kioskapp debug - Runs the app with a debug overlay on");
    println!("  ./kioskapp uninstall - Removes anything the setup script did");
}

fn write_files() {
    let shell_script = r#"#!/bin/bash
MOUNTPOINT="/mnt/slideshow-images"
FLAG="$MOUNTPOINT/.nodevice"
LOCKFILE="/run/mount-slideshow.lock"
 
exec 9>"$LOCKFILE"
flock -n 9 || exit 0
 
mkdir -p "$MOUNTPOINT"
 
is_removable_usb_disk() {
    local disk="$1" syspath
    [[ "$(cat "/sys/block/$disk/removable" 2>/dev/null)" == "1" ]] || return 1
    syspath="$(readlink -f "/sys/block/$disk" 2>/dev/null)"
    [[ "$syspath" == */usb[0-9]*/* ]]
}
 
find_device() {
    local name type fstype dev disk
    while read -r name type fstype; do
        [[ "$type" == "part" || "$type" == "disk" ]] || continue
        [[ "$fstype" == "vfat" ]] || continue
 
        dev="/dev/$name"
        disk="$(lsblk -no PKNAME "$dev" 2>/dev/null | head -n1)"
        [[ -z "$disk" ]] && disk="$name"   # filesystem directly on the disk
 
        is_removable_usb_disk "$disk" || continue
 
        # vfat covers FAT12/16/32, so verify it's actually FAT32
        [[ "$(blkid -p -s VERSION -o value "$dev" 2>/dev/null)" == "FAT32" ]] || continue
 
        echo "$dev"
        return 0
    done < <(lsblk -rno NAME,TYPE,FSTYPE)
    return 1
}
 
if mountpoint -q "$MOUNTPOINT"; then
    src="$(findmnt -rn -M "$MOUNTPOINT" -o SOURCE)"
    [[ -b "$src" ]] && exit 0
    umount -l "$MOUNTPOINT"
fi
 
dev="$(find_device)"
 
if [[ -z "$dev" ]]; then
    touch "$FLAG"
    exit 0
fi
 
rm -f "$FLAG"
 
if ! mount -t vfat -o ro,nosuid,nodev,noexec "$dev" "$MOUNTPOINT"; then
    echo "mount-slideshow: failed to mount $dev" >&2
    touch "$FLAG"
    exit 1
fi"#;

    let systemd_service = r#"[Unit]
Description=Mount USB stick for slideshow
After=network.target

[Service]
Type=oneshot
ExecStart=/opt/scripts/usb-mount.sh
User=root
    "#;

    let systemd_timer = r#"[Unit]
Description=Run USB Mount Script every 5 seconds

[Timer]
OnBootSec=5s
OnUnitActiveSec=5s
AccuracySec=1s
Unit=usb-mount.service

[Install]
WantedBy=timers.target
    "#;

    let file_creation = fs::create_dir_all("/opt/scripts");
    match file_creation {
        Ok(_result) => {println!("Created /opt/scripts");},
        Err(err) => panic!("Failed to create /opt/scripts, are you root? :: {err:?}"),
    }

    let _ = fs::write("/opt/scripts/usb-mount.sh", shell_script);
    println!("Wrote mounting script to /opt/scripts/usb-mount.sh");

    // make it executable
    use std::os::unix::fs::PermissionsExt;
    let perms = fs::Permissions::from_mode(0o755);
    let _ = fs::set_permissions("/opt/scripts/usb-mount.sh", perms);
    println!("Made /opt/scripts/usb-mount.sh executable/");

    // write service files
    let _ = fs::write("/etc/systemd/system/usb-mount.service", systemd_service);
    println!("Wrote /etc/systemd/system/usb-mount.service");

    let _ = fs::write("/etc/systemd/system/usb-mount.timer", systemd_timer);
    println!("Wrote /etc/systemd/system/usb-mount.timer");

    // reload and enable services
    println!("Reloading systemd daemon...");
    let _ = rash!("systemctl daemon-reload").unwrap();
    
    println!("Enabling timer service...");
    let _ = rash!("sudo systemctl enable --now usb-mount.timer").unwrap();

    println!("All done! Hopefully should work now.");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        let command = &args[1];
        
        if (command != "setup" && command != "debug" && command != "uninstall") || command.eq("help") {
            print_help();
            return;
        }

        if command.eq("uninstall") {
            if !is_root() {
                println!("This command has to be run as root as it involves modifying system files.");
                return;
            }

            println!("Removing mounting script and systemd service...");
            let _ = fs::remove_file("/opt/scripts/usb-mount.sh");
            let _ = fs::remove_file("/etc/systemd/system/usb-mount.timer");
            let _ = fs::remove_file("/etc/systemd/system/usb-mount.service");

            println!("Reloading systemd daemon...");
            let _ = rash!("systemctl daemon-reload").unwrap();

            println!("All done!");
            return;
        }

        if command.eq("setup") {
            if !is_root() {
                println!("This command has to be run as root as it involves modifying system files.");
                return;
            }
            write_files();
            return;
        }

        return;
    }

    slideshow_thing_lib::run()
}
