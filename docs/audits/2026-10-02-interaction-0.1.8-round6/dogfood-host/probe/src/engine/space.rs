//! Free space and volume rules — ported from tobi/disktree space.rs
//! (the Linux paths: `/proc/self/mounts`, `/proc/self/mountinfo`, statvfs).

use std::io;
use std::path::{Path, PathBuf};

/// A volume's capacity in bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpaceInfo {
    /// Total size of the volume.
    pub total: u64,
    /// Free blocks, including the reserve only root may write to.
    pub free: u64,
    /// Free blocks this user may actually write; what `df -h` reports.
    pub available: u64,
}

impl SpaceInfo {
    /// Space in use, computed from `free` rather than `available` so the
    /// figure does not jump when a reserve is opened to root.
    pub const fn used(&self) -> u64 {
        self.total.saturating_sub(self.free)
    }

    pub fn used_fraction(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.used() as f64 / self.total as f64) as f32
        }
    }
}

/// Read the space on the volume containing `path`.
pub fn space_info(path: &Path) -> io::Result<SpaceInfo> {
    let stat = rustix::fs::statvfs(path)?;
    // `f_frsize` is the fragment size the block counts are expressed in;
    // `f_bsize` is only a hint for I/O. Some filesystems report zero for
    // `f_frsize`, so fall back rather than claiming a zero-sized volume.
    let block = if stat.f_frsize == 0 {
        stat.f_bsize
    } else {
        stat.f_frsize
    };
    Ok(SpaceInfo {
        total: stat.f_blocks.saturating_mul(block),
        free: stat.f_bfree.saturating_mul(block),
        available: stat.f_bavail.saturating_mul(block),
    })
}

/// One line of the mount table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mount {
    pub source: String,
    pub point: PathBuf,
    pub fstype: String,
    pub options: String,
}

/// Parse `/proc/self/mounts`.
pub fn parse_mounts(table: &str) -> Vec<Mount> {
    table
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let source = fields.next()?.to_string();
            // Spaces in mount points are escaped as \040.
            let point = PathBuf::from(fields.next()?.replace("\\040", " "));
            let fstype = fields.next()?.to_string();
            let options = fields.next().unwrap_or_default().to_string();
            Some(Mount {
                source,
                point,
                fstype,
                options,
            })
        })
        .collect()
}

/// Mount points below `root` that are not part of `root`'s volume, which a
/// scan of that volume must not enter.
///
/// "The volume" is the mount *source*, not the device number: btrfs gives
/// every subvolume its own `st_dev`, so `/home` on Omarchy looks like a
/// different filesystem from `/` though it is the same disk. Everything
/// else is left out: pseudo filesystems (`/proc`, `/sys`), tmpfs, other
/// disks, network shares, and automount points — entering one of those
/// would mount a NAS just to measure it. Snapshot subvolumes are left out
/// too, because every file in them shares its blocks with the live one.
pub fn foreign_mounts(mounts: &[Mount], root: &Path) -> Vec<PathBuf> {
    let Some(own) = mounts
        .iter()
        .filter(|mount| root.starts_with(&mount.point))
        .max_by_key(|mount| mount.point.as_os_str().len())
    else {
        return Vec::new();
    };
    mounts
        .iter()
        .filter(|mount| mount.point != root && mount.point.starts_with(root))
        .filter(|mount| {
            mount.source != own.source
                || mount.fstype != own.fstype
                || is_snapshot(mount)
        })
        .map(|mount| mount.point.clone())
        .collect()
}

fn is_snapshot(mount: &Mount) -> bool {
    let named = mount
        .point
        .file_name()
        .is_some_and(|name| name == ".snapshots");
    named
        || mount.options.split(',').any(|option| {
            option.starts_with("subvol=") && option.contains("snapshots")
        })
}

/// The top of the disk `path` lives on: the shortest mount point above it
/// with the same source.
pub fn volume_root(mounts: &[Mount], path: &Path) -> Option<PathBuf> {
    let own = mounts
        .iter()
        .filter(|mount| path.starts_with(&mount.point))
        .max_by_key(|mount| mount.point.as_os_str().len())?;
    mounts
        .iter()
        .filter(|mount| {
            mount.source == own.source
                && mount.fstype == own.fstype
                && path.starts_with(&mount.point)
        })
        .min_by_key(|mount| mount.point.as_os_str().len())
        .map(|mount| mount.point.clone())
}

/// [`volume_root`] for this machine.
pub fn volume_root_for(path: &Path) -> Option<PathBuf> {
    let table = std::fs::read_to_string("/proc/self/mounts").ok()?;
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    volume_root(&parse_mounts(&table), &path)
}

/// [`foreign_mounts`] for this machine; `None` when the mount table cannot
/// be read, so the caller can fall back to comparing devices.
pub fn foreign_mounts_for(root: &Path) -> Option<Vec<PathBuf>> {
    let table = std::fs::read_to_string("/proc/self/mounts").ok()?;
    Some(foreign_mounts(&parse_mounts(&table), root))
}

/// One line of `/proc/self/mountinfo`.
///
/// What [`Mount`] says, plus which directory of the filesystem the mount
/// shows — what tells a bind mount from the filesystem itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MountView {
    pub device: String,
    pub source: String,
    pub fstype: String,
    pub point: PathBuf,
    pub fs_root: PathBuf,
}

/// Parse `/proc/self/mountinfo`. Lines it cannot read are skipped.
pub fn parse_mountinfo(table: &str) -> Vec<MountView> {
    table
        .lines()
        .filter_map(|line| {
            // Optional fields run up to a lone `-`; after it come the type
            // and the source.
            let (left, right) = line.split_once(" - ")?;
            let mut left = left.split_whitespace().skip(2);
            let device = left.next()?.to_string();
            let fs_root = PathBuf::from(unescape_octal(left.next()?));
            let point = PathBuf::from(unescape_octal(left.next()?));
            let mut right = right.split_whitespace();
            let fstype = right.next()?.to_string();
            let source = unescape_octal(right.next()?);
            Some(MountView {
                device,
                source,
                fstype,
                point,
                fs_root,
            })
        })
        .collect()
}

/// The kernel writes space, tab, newline and backslash in mount fields as
/// three octal digits after a backslash. Decoded in one pass, so a name
/// that really contains `\040` is not decoded twice.
fn unescape_octal(field: &str) -> String {
    let bytes = field.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let digits = bytes.get(index + 1..index + 4);
        let value = digits
            .filter(|_| bytes[index] == b'\\')
            .and_then(|digits| std::str::from_utf8(digits).ok())
            .and_then(|digits| u8::from_str_radix(digits, 8).ok());
        if let Some(value) = value {
            out.push(value);
            index += 4;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Paths under `root` that show a directory the scan already reaches
/// another way, so walking them would count the same files twice.
///
/// A bind mount, a second mount of the same disk, or the top of a Btrfs
/// filesystem mounted beside its subvolumes all show one directory at two
/// paths. Device numbers cannot tell: a bind mount has its original's. The
/// mount table can, since each line names the filesystem and the directory
/// of it that is shown. When one directory is visible twice, the view
/// showing the widest part of the filesystem is kept, and the narrower one
/// is left out. The scanned root itself is never left out; if it is the
/// narrower view, its copy inside the wider one is.
pub fn repeated_mounts(mounts: &[MountView], root: &Path) -> Vec<PathBuf> {
    repeated_mounts_excluding(mounts, root, &[])
}

fn same_filesystem(left: &MountView, right: &MountView) -> bool {
    left.fstype == right.fstype
        && (left.device == right.device
            // Btrfs assigns device numbers per subvolume. Its block device
            // identifies the filesystem shared by those different roots.
            || (left.fstype == "btrfs"
                && left.source.starts_with("/dev/")
                && left.source == right.source))
}

fn covering_mount<'a>(mounts: &'a [MountView], path: &Path) -> Option<&'a MountView> {
    mounts
        .iter()
        .filter(|mount| path.starts_with(&mount.point))
        .max_by_key(|mount| mount.point.components().count())
}

fn repeated_mounts_excluding(
    mounts: &[MountView],
    root: &Path,
    excluded: &[PathBuf],
) -> Vec<PathBuf> {
    let own = covering_mount(mounts, root);
    let mut visible: Vec<&MountView> = mounts
        .iter()
        .filter(|mount| {
            (mount.point.starts_with(root) || Some(*mount) == own)
                && !excluded.iter().any(|skip| mount.point.starts_with(skip))
        })
        .collect();
    visible.sort_by_key(|mount| {
        (
            mount.fs_root.components().count(),
            mount.point.components().count(),
            &mount.point,
        )
    });
    let mut kept: Vec<&MountView> = Vec::new();
    let mut repeated: Vec<PathBuf> = Vec::new();
    for mount in visible {
        let elsewhere = kept.iter().find_map(|wider| {
            if !same_filesystem(wider, mount) {
                return None;
            }
            let inside = mount.fs_root.strip_prefix(&wider.fs_root).ok()?;
            let path = wider.point.join(inside);
            if !path.starts_with(root)
                || path == mount.point
                || excluded
                    .iter()
                    .chain(&repeated)
                    .any(|skip| path.starts_with(skip))
            {
                return None;
            }
            // Another mount can hide the supposed original directory.
            let covering = covering_mount(mounts, &path)?;
            let relative = path.strip_prefix(&covering.point).ok()?;
            (same_filesystem(covering, mount)
                && covering.fs_root.join(relative) == mount.fs_root)
                .then_some(path)
        });
        let skip = elsewhere.map(|copy| {
            if mount.point != root && mount.point.starts_with(root) {
                mount.point.clone()
            } else {
                copy
            }
        });
        if let Some(skip) = skip
            && !root.starts_with(&skip)
            // A duplicate view can contain independently mounted data.
            // Keep it rather than hiding that data with its parent.
            && !mounts
                .iter()
                .any(|child| child.point != skip && child.point.starts_with(&skip))
        {
            if skip != mount.point {
                kept.push(mount);
            }
            repeated.push(skip);
        } else {
            kept.push(mount);
        }
    }
    repeated
}

/// [`repeated_mounts`] for this machine, in `root`'s own spelling. Empty
/// where there is no `/proc/self/mountinfo`.
pub fn repeated_mounts_for(
    root: &Path,
    canonical: &Path,
    one_filesystem: bool,
) -> Vec<PathBuf> {
    let Ok(table) = std::fs::read_to_string("/proc/self/mountinfo") else {
        return Vec::new();
    };
    let excluded = if one_filesystem {
        foreign_mounts_for(canonical).unwrap_or_default()
    } else {
        Vec::new()
    };
    repeated_mounts_excluding(&parse_mountinfo(&table), canonical, &excluded)
        .iter()
        .filter_map(|path| path.strip_prefix(canonical).ok())
        .map(|below| root.join(below))
        .collect()
}

/// Directories under `root` a scan must never enter. Empty on Linux; kept
/// for parity with the original's shape (macOS fills it).
pub fn never_scanned(_root: &Path, _canonical: &Path) -> Vec<PathBuf> {
    Vec::new()
}

/// A mounted volume worth offering as a scan root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Volume {
    pub point: PathBuf,
    pub device: Option<String>,
    pub space: Option<SpaceInfo>,
}

/// Every volume worth offering as a scan root, fullest first.
pub fn volumes() -> Vec<Volume> {
    let Ok(table) = std::fs::read_to_string("/proc/self/mounts") else {
        return Vec::new();
    };
    volumes_in(&parse_mounts(&table))
}

pub fn volumes_in(mounts: &[Mount]) -> Vec<Volume> {
    let mut seen: Vec<&Mount> = Vec::new();
    for mount in mounts {
        if !is_volume_candidate(mount) {
            continue;
        }
        // One device mounted twice (a btrfs disk at `/`, `/home`,
        // `/var/log`) is one volume: keep the shortest mount point, which
        // is the top of that disk.
        if let Some(known) = seen.iter_mut().find(|known| {
            known.source == mount.source && known.fstype == mount.fstype
        }) {
            if mount.point.as_os_str().len() < known.point.as_os_str().len() {
                *known = mount;
            }
            continue;
        }
        seen.push(mount);
    }
    let mut volumes: Vec<Volume> = seen
        .iter()
        .map(|mount| Volume {
            point: mount.point.clone(),
            device: Some(mount.source.clone()),
            space: space_info(&mount.point).ok(),
        })
        .collect();
    // The scarcest room is the most interesting to a cleanup tool.
    sort_by_free_space(&mut volumes);
    volumes
}

fn sort_by_free_space(volumes: &mut [Volume]) {
    volumes.sort_by_key(|volume| {
        (
            volume.space.is_none(),
            volume.space.map_or(0, |space| space.available),
            volume.point.clone(),
        )
    });
}

fn is_volume_candidate(mount: &Mount) -> bool {
    if is_snapshot(mount) {
        return false;
    }
    if mount
        .options
        .split(',')
        .any(|option| option == "automounted" || option.starts_with("autofs"))
    {
        return false;
    }
    !matches!(
        mount.fstype.as_str(),
        "autofs"
            | "cgroup"
            | "cgroup2"
            | "configfs"
            | "debugfs"
            | "devpts"
            | "devtmpfs"
            | "fuse.portal"
            | "fusectl"
            | "hugetlbfs"
            | "mqueue"
            | "nsfs"
            | "overlay"
            | "proc"
            | "pstore"
            | "securityfs"
            | "sysfs"
            | "tmpfs"
            | "tracefs"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const OMARCHY: &str = "\
sys /sys sysfs rw 0 0
run /run tmpfs rw 0 0
/dev/mapper/root / btrfs rw,subvolid=256,subvol=/@ 0 0
/dev/mapper/root /home btrfs rw,subvolid=257,subvol=/@home 0 0
/dev/mapper/root /var/log btrfs rw,subvolid=258,subvol=/@log 0 0
/dev/mapper/root /.snapshots btrfs rw,subvolid=260,subvol=/@snapshots 0 0
/dev/nvme0n1p1 /boot vfat rw 0 0
systemd-1 /mnt/nas-home autofs rw,direct 0 0
tmpfs /tmp tmpfs rw 0 0
portal /run/user/1000/doc fuse.portal rw 0 0
";

    const MOUNTINFO: &str = "\
22 1 0:21 /@ / rw,relatime - btrfs /dev/mapper/root rw,subvol=/@
23 22 0:21 /@home /home rw,relatime - btrfs /dev/mapper/root rw,subvol=/@home
24 22 0:21 /@log /var/log rw,relatime - btrfs /dev/mapper/root rw,subvol=/@log
25 22 0:21 /@snapshots /.snapshots rw - btrfs /dev/mapper/root rw
26 22 0:30 / /data rw,relatime shared:5 - btrfs /dev/mapper/data rw
27 22 0:30 / /mnt/data rw,relatime shared:5 - btrfs /dev/mapper/data rw
28 22 0:31 / /tmp rw - tmpfs tmpfs rw
29 22 259:3 / /boot rw - vfat /dev/nvme0n1p1 rw
";

    fn repeated(table: &str, root: &str) -> Vec<PathBuf> {
        let mut paths =
            repeated_mounts(&parse_mountinfo(table), Path::new(root));
        paths.sort();
        paths
    }

    #[test]
    fn subvolumes_are_not_repeats_but_a_second_mount_of_a_disk_is() {
        assert_eq!(repeated(MOUNTINFO, "/"), [PathBuf::from("/mnt/data")]);
        assert!(repeated(MOUNTINFO, "/home/tobi").is_empty());
        assert!(repeated(MOUNTINFO, "/data").is_empty());
        assert!(
            repeated(MOUNTINFO, "/mnt/data").is_empty(),
            "the view that was asked for is scanned"
        );
    }

    #[test]
    fn a_bind_mount_inside_the_scan_is_left_out() {
        let table = format!(
            "{MOUNTINFO}30 23 0:21 /@home/tobi/src /srv/src rw - btrfs /dev/mapper/root rw\n"
        );
        assert_eq!(
            repeated(&table, "/"),
            [PathBuf::from("/mnt/data"), PathBuf::from("/srv/src")]
        );
        assert!(repeated(&table, "/home/tobi").is_empty());
        assert!(repeated(&table, "/srv").is_empty());
    }

    #[test]
    fn the_top_of_a_btrfs_disk_beside_its_subvolumes_is_counted_once() {
        let table = format!(
            "{MOUNTINFO}31 22 0:21 / /mnt/top rw - btrfs /dev/mapper/root rw\n"
        );
        assert_eq!(
            repeated(&table, "/"),
            [
                PathBuf::from("/.snapshots"),
                PathBuf::from("/home"),
                PathBuf::from("/mnt/data"),
                PathBuf::from("/mnt/top/@"),
                PathBuf::from("/var/log"),
            ]
        );
    }

    #[test]
    fn a_volume_includes_its_subvolumes_and_nothing_else() {
        let mounts = parse_mounts(OMARCHY);
        let mut foreign = foreign_mounts(&mounts, Path::new("/"));
        foreign.sort();
        let expected: Vec<PathBuf> = [
            "/.snapshots",
            "/boot",
            "/mnt/nas-home",
            "/run",
            "/run/user/1000/doc",
            "/sys",
            "/tmp",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();
        assert_eq!(foreign, expected, "/home and /var/log are the same disk");
    }

    #[test]
    fn the_whole_disk_is_the_top_of_the_home_volume() {
        let mounts = parse_mounts(OMARCHY);
        let root = volume_root(&mounts, Path::new("/home/eddix"));
        assert_eq!(root, Some(PathBuf::from("/")), "@home is on the root disk");
    }

    #[test]
    fn a_home_scan_has_no_foreign_mounts_here() {
        let mounts = parse_mounts(OMARCHY);
        assert!(foreign_mounts(&mounts, Path::new("/home/eddix")).is_empty());
    }

    #[test]
    fn volume_candidates_are_real_disks_not_pseudo_filesystems() {
        let mounts = parse_mounts(OMARCHY);
        let points: Vec<PathBuf> = volumes_in(&mounts)
            .iter()
            .map(|volume| volume.point.clone())
            .collect();
        assert!(points.contains(&PathBuf::from("/")), "{points:?}");
        assert!(points.contains(&PathBuf::from("/boot")), "{points:?}");
        assert_eq!(points.len(), 2, "{points:?}");
    }

    #[test]
    fn a_real_volume_reports_plausible_numbers() {
        let temp = std::env::temp_dir();
        let space = space_info(&temp).expect("temp dir has a volume");
        assert!(space.total > 0, "{space:?}");
        assert!(space.free <= space.total, "{space:?}");
        assert!(space.available <= space.free, "{space:?}");
        assert!((0.0..=1.0).contains(&space.used_fraction()), "{space:?}");
    }
}
