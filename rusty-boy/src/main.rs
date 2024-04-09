mod file_rom;

use std::io::BufWriter;
use std::path::PathBuf;
use std::str::FromStr;

use ppu::{Color, Ppu, PpuResult, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use sm83::core::{Cpu, Interrupts};

use file_rom::FileRom;

struct GbAddressSpace<'a> {
    rom: &'a mut FileRom,
    ppu: &'a mut Ppu,
}

impl<'a> sm83::memory::Memory for GbAddressSpace<'a> {
    fn read(&mut self, address: sm83::memory::Address) -> u8 {
        match address {
            0x0000..=0x7FFF => self.rom.read(address),
            0x8000..=0x9FFF | 0xFE00..=0xFE9F | 0xFF40..=0xFF4B => self.ppu.read(address),
            _ => panic!("Invalid read address: {}", address),
        }
    }

    fn write(&mut self, address: sm83::memory::Address, value: u8) {
        match address {
            0x0000..=0x7fff => self.rom.write(address, value),
            0x8000..=0x9FFF | 0xFE00..=0xFE9F | 0xFF40..=0xFF4B => self.ppu.write(address, value),
            _ => panic!("Invalid write address: {}, value {}", address, value),
        }
    }
}

fn handle_frame(
    idx: usize,
    frame: &[[Color; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
) -> anyhow::Result<()> {
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
    let mut rom = FileRom::from_file(&path)?;
    let rom = &mut rom;

    let mut cpu = Cpu::new();
    let mut ppu = Ppu::new();

    cpu.get_mut_regs().pc_reg = 0x100;
    dbg!(cpu.get_regs());

    const NUM_FRAMES: usize = 10;
    let mut frame_idx = 0;

    loop {
        let ppu = &mut ppu;
        let mut memory = GbAddressSpace { rom, ppu };

        let result = cpu.step(&mut memory, Interrupts::new());
        match result {
            sm83::core::ExitReason::Step(cycles) => {
                if let PpuResult::FrameComplete(frame) = ppu.run(cycles) {
                    handle_frame(frame_idx, frame)?;
                    frame_idx += 1;
                }
            }
            _ => {
                panic!("Unexpected PPU exit reason")
            }
        }

        if frame_idx > NUM_FRAMES {
            break;
        }
    }

    dbg!(cpu.get_regs());

    Ok(())
}
