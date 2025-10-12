use anyhow::Result;

use std::path::PathBuf;
use std::process::Command;

#[derive(serde::Deserialize)]
struct LsBlkOutput {
    blockdevices: Vec<BlockDevice>,
}

#[derive(serde::Deserialize)]
struct BlockDevice {
    name: String,
    mountpoints: Vec<PathBuf>,
    #[serde(default)]
    children: Vec<BlockDevice>,
}

fn run_lsblk(args: &[&str]) -> Result<Vec<u8>> {
    let mut cmd = Command::new("lsblk");
    cmd.args(args);
    Ok(cmd.output()?.stdout)
}

fn find_disk(devs: &[BlockDevice], req_mountpoint: &str) -> Result<Option<(String, PathBuf)>> {
    for dev in devs {
        if let Some(mountpoint) = dev
            .mountpoints
            .iter()
            .find(|mountpoint| mountpoint.ends_with(req_mountpoint))
        {
            // Found
            return Ok(Some((dev.name.clone(), mountpoint.clone())));
        }

        if let Some(res) = find_disk(&dev.children, req_mountpoint)? {
            return Ok(Some(res));
        }
    }
    Ok(None)
}

pub fn lookup_disk_by_mountpoint(mountpoint: &str) -> Result<Option<(String, PathBuf)>> {
    let unparsed = run_lsblk(&["-J"])?;
    let parsed: LsBlkOutput = serde_json::from_slice(&unparsed)?;

    find_disk(&parsed.blockdevices, mountpoint)
}

pub fn eject_disk(name: &str) -> Result<()> {
    let path = format!("/dev/{name}");
    let mut cmd = Command::new("sudo");
    cmd.args(["eject", &path]);
    let exit_status = cmd.status()?;
    if !exit_status.success() {
        anyhow::bail!("Ejecting disk returned with code {exit_status}");
    }
    Ok(())
}
