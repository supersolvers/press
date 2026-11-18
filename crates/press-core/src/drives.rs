// SPDX-License-Identifier: GPL-3.0-or-later
//! The drives plugged in or connected to, beyond the disk the system is
//! on: USB disks and the like, which the system mounts under /media or
//! /run/media, and drives on the network, mounted by the system (NFS, SMB,
//! sshfs) or by GIO for the user (under $XDG_RUNTIME_DIR/gvfs).
//!
//! The desktop shows each as an icon and the file manager lists them; both
//! read them from here, so that they agree.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drive {
    /// What to call it: the name it was mounted under, or for a share on
    /// the network the share and the machine it is on.
    pub name: String,
    /// Where its files are.
    pub path: PathBuf,
    /// Across the network, rather than plugged in.
    pub network: bool,
}

/// Kinds of file system that live on another machine.
const NETWORK: [&str; 12] = ["nfs", "nfs4", "cifs", "smb3", "smbfs", "fuse.sshfs", "sshfs", "davfs", "fuse.davfs", "fuse.rclone", "afpfs", "9p"];

/// The drives mounted now, in order of name.
pub fn mounted() -> Vec<Drive> {
    let mut found = from_mountinfo(&std::fs::read_to_string("/proc/self/mountinfo").unwrap_or_default());
    if let Ok(list) = std::fs::read_dir(gvfs()) {
        for entry in list.flatten() {
            let id = entry.file_name().to_string_lossy().into_owned();
            if let Some((name, network)) = gvfs_name(&id) {
                found.push(Drive { name, path: entry.path(), network });
            }
        }
    }
    found.sort_by_key(|d| d.name.to_lowercase());
    found.dedup_by(|a, b| a.path == b.path);
    found
}

/// Where GIO puts the drives it mounts for the user, so that every program
/// can reach them as ordinary folders.
pub fn gvfs() -> PathBuf {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).filter(|p| p.is_absolute());
    runtime
        .unwrap_or_else(|| {
            use std::os::unix::fs::MetadataExt;
            // whose this process is, without asking the C library
            let uid = std::fs::metadata("/proc/self").map(|m| m.uid()).unwrap_or(0);
            PathBuf::from(format!("/run/user/{uid}"))
        })
        .join("gvfs")
}

/// The drives among the system's mounts, from /proc/self/mountinfo: what
/// is mounted under /media, /run/media or /mnt, and anything from across
/// the network wherever it is.
pub fn from_mountinfo(text: &str) -> Vec<Drive> {
    text.lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split(' ').collect();
            let point = unescape(fields.get(4)?);
            let kind = fields.iter().position(|f| *f == "-").and_then(|i| fields.get(i + 1)).copied().unwrap_or_default();
            let network = NETWORK.contains(&kind);
            let plugged = ["/media/", "/run/media/", "/mnt/"].iter().any(|p| point.starts_with(p));
            // GIO's own folder, and the portal's, are not drives
            let plumbing = kind.starts_with("fuse.gvfs") || kind == "fuse.portal" || kind == "autofs";
            ((network || plugged) && !plumbing).then(|| Drive { name: Path::new(&point).file_name().map_or_else(|| point.clone(), |n| n.to_string_lossy().into_owned()), path: PathBuf::from(point), network })
        })
        .collect()
}

/// Mount points are written with a space, a tab, a line break and a
/// backslash as a backslash and three octal digits.
fn unescape(field: &str) -> String {
    let mut out = Vec::new();
    let bytes = field.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 3 < bytes.len() && bytes[i + 1..i + 4].iter().all(|b| (b'0'..=b'7').contains(b)) {
            out.push(u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 4]).unwrap_or("0"), 8).unwrap_or(b'?'));
            i += 4;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// What to call a drive GIO has mounted, from the name of its folder
/// (such as `smb-share:server=nas,share=music`), and whether it is on the
/// network. Phones and cameras plugged in are mounted there too.
pub fn gvfs_name(id: &str) -> Option<(String, bool)> {
    let (scheme, rest) = id.split_once(':')?;
    let value = |key: &str| rest.split(',').find_map(|kv| kv.strip_prefix(key)?.strip_prefix('=')).map(unpercent);
    let host = value("host").or_else(|| value("server"));
    let network = !matches!(scheme, "mtp" | "gphoto2" | "afc");
    let name = match (value("share").or_else(|| value("volume")), &host) {
        (Some(share), Some(host)) => format!("{share} on {host}"),
        (_, Some(host)) if network => host.clone(),
        // a phone or a camera, named with underscores for spaces
        (_, Some(device)) => device.replace('_', " "),
        _ => scheme.into(),
    };
    Some((name, network))
}

fn unpercent(s: &str) -> String {
    let (mut out, mut it) = (Vec::new(), s.bytes());
    while let Some(b) = it.next() {
        let hex = |c: Option<u8>| c.and_then(|c| (c as char).to_digit(16));
        match b {
            b'%' => match (hex(it.next()), hex(it.next())) {
                (Some(h), Some(l)) => out.push((h * 16 + l) as u8),
                _ => out.push(b'?'),
            },
            _ => out.push(b),
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The drive a path is on, if it is on one of these.
pub fn holding<'a>(drives: &'a [Drive], path: &Path) -> Option<&'a Drive> {
    drives.iter().filter(|d| path.starts_with(&d.path)).max_by_key(|d| d.path.as_os_str().len())
}

/// Let a drive go: unmount it, by way of GIO, which asks the system to
/// when the user may not do it themselves. Says why not if it could not.
pub fn eject(path: &Path) -> Result<(), String> {
    let out = std::process::Command::new("gio").arg("mount").arg("-u").arg(path).stdin(std::process::Stdio::null()).output().map_err(|e| format!("gio could not be run: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    let said = String::from_utf8_lossy(&out.stderr);
    let said = said.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("it is still in use");
    // gio starts its lines with what it was doing
    Err(said.trim_start_matches("gio: ").rsplit(": ").next().unwrap_or(said).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drives_are_found_among_the_mounts() {
        let info = "\
22 1 259:2 / / rw,relatime shared:1 - ext4 /dev/nvme0n1p2 rw
40 22 0:35 / /run/user/1000 rw shared:9 - tmpfs tmpfs rw
41 40 0:50 / /run/user/1000/gvfs rw shared:10 - fuse.gvfsd-fuse gvfsd-fuse rw
42 40 0:51 / /run/user/1000/doc rw shared:11 - fuse.portal portal rw
50 22 8:17 / /media/k/MY\\040STICK rw shared:20 - vfat /dev/sdb1 rw
51 22 0:60 / /srv/music rw shared:21 - nfs4 nas:/music rw
52 22 0:61 / /home/k/box rw shared:22 - fuse.sshfs k@box: rw
53 22 259:3 / /home/k/Games rw shared:23 - btrfs /dev/nvme1n1 rw
";
        let found = from_mountinfo(info);
        assert_eq!(
            found,
            vec![
                Drive { name: "MY STICK".into(), path: "/media/k/MY STICK".into(), network: false },
                Drive { name: "music".into(), path: "/srv/music".into(), network: true },
                Drive { name: "box".into(), path: "/home/k/box".into(), network: true },
            ]
        );
        assert_eq!(holding(&found, Path::new("/media/k/MY STICK/photos")).map(|d| d.name.as_str()), Some("MY STICK"));
        assert_eq!(holding(&found, Path::new("/home/k")), None);
    }

    #[test]
    fn what_gio_mounts_is_named_for_people() {
        assert_eq!(gvfs_name("smb-share:server=nas,share=music"), Some(("music on nas".into(), true)));
        assert_eq!(gvfs_name("sftp:host=192.168.122.124,user=ci"), Some(("192.168.122.124".into(), true)));
        assert_eq!(gvfs_name("dav:host=files.example.org,ssl=true,prefix=%2Fremote.php"), Some(("files.example.org".into(), true)));
        assert_eq!(gvfs_name("afp-volume:host=old-mac.local,volume=Macintosh%20HD"), Some(("Macintosh HD on old-mac.local".into(), true)));
        assert_eq!(gvfs_name("mtp:host=Google_Pixel_8_1A2B3C"), Some(("Google Pixel 8 1A2B3C".into(), false)));
        assert_eq!(gvfs_name("nonsense"), None);
    }
}
