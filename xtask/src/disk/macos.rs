use anyhow::Result;
use std::{path::PathBuf, process::Command};

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct DiskutilListOutput {
    // Only lists entire disks, not partitions
    whole_disks: Vec<String>,
    // Includes disk partitions
    all_disks: Vec<String>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct DiskutilInfoOutput {
    media_name: String,
    mount_point: Option<String>,
}

fn run_diskutil(args: &[&str]) -> Result<Vec<u8>> {
    let mut cmd = Command::new("diskutil");
    cmd.args(args);
    Ok(cmd.output()?.stdout)
}

pub fn lookup_disk_by_name(name: &str) -> Result<Option<(String, PathBuf)>> {
    let diskutil_list: DiskutilListOutput = plist::from_bytes(&run_diskutil(&["list", "-plist"])?)?;

    for disk in &diskutil_list.whole_disks {
        let info: DiskutilInfoOutput =
            plist::from_bytes(&run_diskutil(&["info", "-plist", disk])?)?;
        if info.media_name == name {
            // Found the target disk, now get the first partition
            for part in &diskutil_list.all_disks {
                if part.len() > disk.len() && part.starts_with(disk) {
                    let part_info: DiskutilInfoOutput =
                        plist::from_bytes(&run_diskutil(&["info", "-plist", part])?)?;

                    let mount_point = part_info.mount_point.expect("Partition is not mounted!");
                    return Ok(Some((part.to_string(), mount_point.into())));
                }
            }
        }
    }

    Ok(None)
}

pub fn eject_disk(name: &str) -> Result<()> {
    run_diskutil(&["eject", name])?;
    Ok(())
}
