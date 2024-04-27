use clap::Parser;
use std::path::PathBuf;

use rusty_boy::disassembler::Disassembler;

/// Disassembles the given ROM, producing a stream of sm83 instructions
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// The ROM to disassemble
    rom_path: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let data = std::fs::read(args.rom_path)?;
    let disassembler = Disassembler::new(&data);

    let header = disassembler.header()?;
    println!("Rom Header:");
    println!("\tTitle: \"{}\"", header.title);
    println!("\tManufacturer code: {:?}", header.manufacturer_code);
    println!("\tCGB: {:?}", header.cgb_flag);
    assert_eq!(header.rom_size % (32 * 1024), 0);
    println!("\tROM size: {} KiB", header.rom_size / 1024);
    println!("\tRAM size: {}", header.ram_size);
    println!("\tType: {}", header.cartridge_type);
    println!("\tEntrypoint:");
    for (addr, insn) in disassembler.entrypoint()? {
        println!("\t\t{:#x}\t{}", addr, insn);
    }

    for (addr, insn) in disassembler.disassemble()? {
        println!("{:#x}\t{}", addr, insn);
    }

    Ok(())
}
