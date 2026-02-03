use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

pub fn reveal_in_file_manager<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("-R")
            .arg(path)
            .status()
            .context("Failed to invoke Finder")?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        let arg = format!("/select,{}", path.display());
        Command::new("explorer")
            .arg(arg)
            .status()
            .context("Failed to invoke Explorer")?;
        return Ok(());
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err(anyhow::anyhow!("Unsupported platform"))
    }
}

pub fn open_file<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path)
            .status()
            .context("Failed to open file")?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(path)
            .status()
            .context("Failed to open file")?;
        return Ok(());
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err(anyhow::anyhow!("Unsupported platform"))
    }
}
