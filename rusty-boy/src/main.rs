mod file_rom;
mod memory;
mod rusty_boy;

use std::io::BufWriter;
use std::path::PathBuf;
use std::str::FromStr;

use ppu::{Color, DISPLAY_HEIGHT, DISPLAY_WIDTH};

use rusty_boy::RustyBoy;

fn _save_png(idx: usize, frame: &[[Color; DISPLAY_WIDTH]; DISPLAY_HEIGHT]) -> anyhow::Result<()> {
    let path = PathBuf::from_str(&format!("frame_{idx}.png"))?;

    const MAX: u8 = 255;
    let frame: Vec<u8> = frame
        .into_iter()
        .map(|l| {
            l.into_iter().map(|c| match c {
                Color::White => MAX,
                Color::LightGrey => MAX / 3 * 2,
                Color::DarkGrey => MAX / 3,
                Color::Black => 0,
            })
        })
        .flatten()
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

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from_str("main.gb")?;
    let mut rusty_boy = RustyBoy::new_with_rom(&path)?;

    let sdl_context = sdl2::init().unwrap();
    let video_subsys = sdl_context.video().unwrap();

    let window = video_subsys
        .window("rusty-boy", DISPLAY_WIDTH as u32, DISPLAY_HEIGHT as u32)
        .position_centered()
        .build()?;

    let mut canvas = window.into_canvas().build()?;
    canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0xff, 0));
    canvas.clear();
    canvas.present();

    let mut event_pump = sdl_context.event_pump().unwrap();
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                sdl2::event::Event::Quit { .. }
                | sdl2::event::Event::KeyDown {
                    keycode: Some(sdl2::keyboard::Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        let frame = rusty_boy.run_until_next_frame().unwrap();
        for (y, line) in frame.iter().enumerate() {
            for (x, p) in line.iter().enumerate() {
                const MAX: u8 = 255;
                let color = match p {
                    Color::White => MAX,
                    Color::LightGrey => MAX / 3 * 2,
                    Color::DarkGrey => MAX / 3,
                    Color::Black => 0,
                };
                canvas.set_draw_color(sdl2::pixels::Color::RGB(color, color, color));
                canvas
                    .draw_point(sdl2::rect::Point::new(x as i32, y as i32))
                    .unwrap();
            }
        }

        canvas.present();
        std::thread::sleep(std::time::Duration::new(0, 1_000_000_000u32 / 60));
    }

    Ok(())
}
