use anyhow::Result;
use std::path::PathBuf;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub fn find_playdate_data_disk() -> Result<Option<PathBuf>> {
    todo!()
}

#[cfg(target_os = "macos")]
mod macos;

use macos::*;

#[cfg(target_os = "macos")]
pub fn find_playdate_data_disk() -> Result<Option<(String, PathBuf)>> {
    lookup_disk_by_name("Playdate")
}

pub use macos::eject_disk;
