use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{Duration, Instant};

use anyhow::bail;
use cartridge::Cartridge;
use clap::Parser;
use ppu::{Color, Frame, DISPLAY_HEIGHT, DISPLAY_WIDTH};

use rusty_boy::RustyBoy;

/// Runs the given Game Boy emulator ROM
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// The ROM to run
    rom_path: PathBuf,

    /// Saves PNG files with each frame to the current directory
    #[arg(short)]
    save_pngs: bool,

    /// Enable debugging
    #[arg(short)]
    debug: bool,
}

fn save_png(idx: usize, frame: &[[Color; DISPLAY_WIDTH]; DISPLAY_HEIGHT]) -> anyhow::Result<()> {
    let path = PathBuf::from_str(&format!("frame_{idx}.png"))?;

    const MAX: u8 = 255;
    let frame: Vec<u8> = frame
        .iter()
        .flat_map(|l| {
            l.iter().map(|c| match c {
                Color::White => MAX,
                Color::LightGrey => MAX / 3 * 2,
                Color::DarkGrey => MAX / 3,
                Color::Black => 0,
            })
        })
        .collect();

    let file = std::fs::File::create(&path)?;
    let w = BufWriter::new(file);
    let mut png_encoder = png::Encoder::new(w, DISPLAY_WIDTH as u32, DISPLAY_HEIGHT as u32);

    png_encoder.set_color(png::ColorType::Grayscale);
    png_encoder.set_depth(png::BitDepth::Eight);
    let mut writer = png_encoder.write_header()?;

    writer.write_image_data(&frame)?;
    Ok(())
}

pub fn sleep_until(deadline: Instant) {
    let now = Instant::now();
    if let Some(delay) = deadline.checked_duration_since(now) {
        std::thread::sleep(delay);
    }
}

fn draw_surface_argb8888(surface: &mut [u8], frame: &Frame) -> anyhow::Result<()> {
    let pixel_iter = frame.iter().flat_map(|l| l.iter());

    // The size of each ARGB8888 pixel is 4 bytes
    const PIXEL_SIZE: usize = 4;
    for (dest, src) in surface.chunks_mut(PIXEL_SIZE).zip(pixel_iter) {
        const MAX: u8 = 255;
        let color = match src {
            Color::White => MAX,
            Color::LightGrey => MAX / 3 * 2,
            Color::DarkGrey => MAX / 3,
            Color::Black => 0,
        };
        dest[0] = color; // B
        dest[1] = color; // G
        dest[2] = color; // R
        dest[3] = 0xFF; // A
    }

    Ok(())
}

fn draw_surface_rgb888(surface: &mut [u8], frame: &Frame) -> anyhow::Result<()> {
    let pixel_iter = frame.iter().flat_map(|l| l.iter());

    // The size of each RGB888 pixel is 4 bytes, last one is unused...
    const PIXEL_SIZE: usize = 4;
    for (dest, src) in surface.chunks_mut(PIXEL_SIZE).zip(pixel_iter) {
        const MAX: u8 = 255;
        let color = match src {
            Color::White => MAX,
            Color::LightGrey => MAX / 3 * 2,
            Color::DarkGrey => MAX / 3,
            Color::Black => 0,
        };
        dest[0] = color; // B
        dest[1] = color; // G
        dest[2] = color; // R
    }

    Ok(())
}

fn attempt_restore_save_file(rusty_boy: &mut RustyBoy, rom_path: &Path) -> anyhow::Result<()> {
    let save_file_path = rom_path.with_extension("save");

    let data = match std::fs::read(&save_file_path) {
        Ok(data) => data,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(v) => {
            anyhow::bail!("Unable to read save file: {v}")
        }
    };

    rusty_boy
        .restore_cartridge_ram(&data)
        .map_err(|e| anyhow::format_err!("Unable to load cartridge ram: {}", e))?;

    Ok(())
}

fn save_file(rom_path: &Path, data: &[u8]) -> anyhow::Result<()> {
    let save_file_path = rom_path.with_extension("save");
    std::fs::write(&save_file_path, data)?;
    Ok(())
}

#[cfg(feature = "profile")]
fn configure_sched_affinity() -> anyhow::Result<()> {
    log::info!("Setting CPU affinity");
    let mut cpuset = nix::sched::CpuSet::new();
    cpuset.set(0)?; // Just one cpu
    nix::sched::sched_setaffinity(nix::unistd::Pid::this(), &cpuset)?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    #[cfg(feature = "profile")]
    configure_sched_affinity()?;

    env_logger::init();

    let args = Args::parse();

    let rom_data = std::fs::read(&args.rom_path)?;
    let cartridge = Cartridge::try_new(rom_data)
        .map_err(|e| anyhow::format_err!("Invalid cartridge: {}", e))?;
    let mut rusty_boy = RustyBoy::new_with_cartridge(cartridge);

    #[cfg(feature = "approximate")]
    rusty_boy.configure_cpu_step(sm83::core::Cycles::new(60));

    if rusty_boy.supports_battery_backed_ram() {
        attempt_restore_save_file(&mut rusty_boy, &args.rom_path)?;
    }

    if args.debug {
        rusty_boy.enable_debug()
    }

    let sdl_context = sdl2::init().unwrap();
    let video_subsys = sdl_context.video().unwrap();

    let window = video_subsys
        .window("rusty-boy", DISPLAY_WIDTH as u32, DISPLAY_HEIGHT as u32)
        .position_centered()
        .build()?;

    let mut event_pump = sdl_context.event_pump().unwrap();
    let surface = window.surface(&event_pump).unwrap();
    let update_surface_func = match surface.pixel_format_enum() {
        sdl2::pixels::PixelFormatEnum::ARGB8888 => draw_surface_argb8888,
        sdl2::pixels::PixelFormatEnum::RGB888 => draw_surface_rgb888,
        _ => bail!(
            "Unsupported pixel format: {:?}",
            surface.pixel_format_enum()
        ),
    };

    let mut frame_id = 0;

    let mut joypad = rusty_boy::joypad::State::new();

    let mut next_deadline = Instant::now();
    let mut start = Instant::now();
    let mut load = Duration::from_millis(0);
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                sdl2::event::Event::Quit { .. }
                | sdl2::event::Event::KeyDown {
                    keycode: Some(sdl2::keyboard::Keycode::Escape),
                    ..
                } => break 'running,

                sdl2::event::Event::KeyDown {
                    keycode: Some(key), ..
                } => match key {
                    sdl2::keyboard::Keycode::W => joypad.up = true,
                    sdl2::keyboard::Keycode::S => joypad.down = true,
                    sdl2::keyboard::Keycode::D => joypad.right = true,
                    sdl2::keyboard::Keycode::A => joypad.left = true,
                    sdl2::keyboard::Keycode::J => joypad.a = true,
                    sdl2::keyboard::Keycode::K => joypad.b = true,
                    sdl2::keyboard::Keycode::Semicolon => joypad.start = true,
                    sdl2::keyboard::Keycode::Space => joypad.select = true,
                    _ => {}
                },

                sdl2::event::Event::KeyUp {
                    keycode: Some(key), ..
                } => match key {
                    sdl2::keyboard::Keycode::W => joypad.up = false,
                    sdl2::keyboard::Keycode::S => joypad.down = false,
                    sdl2::keyboard::Keycode::D => joypad.right = false,
                    sdl2::keyboard::Keycode::A => joypad.left = false,
                    sdl2::keyboard::Keycode::J => joypad.a = false,
                    sdl2::keyboard::Keycode::K => joypad.b = false,
                    sdl2::keyboard::Keycode::Semicolon => joypad.start = false,
                    sdl2::keyboard::Keycode::Space => joypad.select = false,
                    _ => {}
                },

                _ => {}
            }
        }

        rusty_boy.update_keys(&joypad);

        let frame = {
            let frame_start = Instant::now();

            #[cfg(feature = "approximate")]
            rusty_boy.run_until_next_frame(false);

            let frame = rusty_boy.run_until_next_frame(true);
            let frame_end = Instant::now();
            load += frame_end - frame_start;
            frame
        };

        if args.save_pngs {
            save_png(frame_id, frame)?;
        }

        let mut surface = window.surface(&event_pump).unwrap();
        surface.with_lock_mut(|p| {
            update_surface_func(p, frame).unwrap();
        });
        surface.finish().unwrap();

        {
            let now = Instant::now();
            let duration = now - start;
            if duration > Duration::from_secs(1) {
                let load_pct = load.as_nanos() as f64 / duration.as_nanos() as f64 * 100.0;
                log::info!("CPU usage is {} %", load_pct);
                start = now;
                load = Duration::from_secs(0);
            }
        }

        #[cfg(feature = "approximate")]
        const FRAME_TIME: Duration = Duration::from_nanos(33_333_333); // 30 fps
        #[cfg(not(feature = "approximate"))]
        const FRAME_TIME: Duration = Duration::from_nanos(16_666_667); // 60 fps

        next_deadline += FRAME_TIME;

        sleep_until(next_deadline);

        frame_id += 1;
    }

    if rusty_boy.supports_battery_backed_ram() {
        if let Some(ram) = rusty_boy.get_cartridge_ram() {
            save_file(&args.rom_path, ram)?;
        }
    }

    Ok(())
}
