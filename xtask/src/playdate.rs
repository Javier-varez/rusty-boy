use std::{
    path::Path,
    time::{Duration, Instant},
};

use serialport::{SerialPort, SerialPortInfo};

use anyhow::{Context, Result};

fn find_port() -> Result<Option<SerialPortInfo>> {
    let ports = serialport::available_ports()?;
    Ok(ports.into_iter().find(|port| {
        matches!(port.port_type, serialport::SerialPortType::UsbPort(ref usb) if
            usb.manufacturer.as_ref().is_some_and(|s| s == "Panic Inc") &&
            usb.product.as_ref().is_some_and(|s| s == "Playdate"))
    }))
}

fn open_port() -> Result<Box<dyn SerialPort>> {
    let port = find_port()?.expect("Could not find Playdate serial port!");
    Ok(serialport::new(&port.port_name, 115200).open()?)
}

pub struct Playdate {
    port: Option<Box<dyn SerialPort>>,
}

impl Playdate {
    pub fn open() -> Result<Self> {
        let port = Some(open_port()?);
        Ok(Self { port })
    }

    fn reconnect(&mut self) -> Result<()> {
        self.port.replace(open_port()?);
        Ok(())
    }

    fn send_command(&mut self, command: &[u8]) -> Result<()> {
        if self.port.is_none() {
            self.reconnect()?;
        }

        let pd = self
            .port
            .as_mut()
            .ok_or(anyhow::anyhow!("Playdate serial is not available"))?;

        pd.write_all(command)?;
        pd.write_all(b"\r\n")?;
        pd.flush()?;

        Ok(())
    }

    fn drop_connection(&mut self) {
        self.port.take();
    }

    pub fn wait_for_connection(&mut self, duration: Duration) -> Result<()> {
        if self.port.is_some() {
            return Ok(());
        };

        let start = Instant::now();
        loop {
            let now = Instant::now();
            if now - start > duration {
                anyhow::bail!("Timeout waiting for port reconnection");
            }
            if find_port()?.is_some() {
                break;
            }
        }
        self.reconnect()
    }

    pub fn load_game(&mut self, src_path: &Path) -> Result<()> {
        self.send_command(b"datadisk")?;
        self.drop_connection();

        let (disk_name, mount_point) = {
            let start = std::time::Instant::now();
            loop {
                let end = std::time::Instant::now();
                if end - start > Duration::from_secs(10) {
                    anyhow::bail!(
                        "Time out while waiting for the playdate to be mounted by the OS"
                    );
                }
                if let Some((disk_name, mount_point)) = crate::disk::find_playdate_data_disk()
                    .context("Error while finding playdate data disk")?
                {
                    println!("Disk {disk_name}, mount_point {mount_point:?}");
                    break (disk_name, mount_point);
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        };

        let mut target_dir = mount_point;
        target_dir.push("Games");

        let sh = xshell::Shell::new()?;
        xshell::cmd!(sh, "cp -r {src_path} {target_dir}/").run()?;

        crate::disk::eject_disk(&disk_name).context("Error ejecting playdate data disk")?;

        Ok(())
    }

    pub fn run_game(&mut self, game: &str) -> Result<()> {
        let mut cmd: Vec<_> = b"run /Games/".into();
        cmd.extend(game.as_bytes());
        self.send_command(&cmd)
    }
}
