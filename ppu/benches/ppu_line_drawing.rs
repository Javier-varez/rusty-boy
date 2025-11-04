use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

use ppu::Ppu;
use rand::Rng;
use tock_registers::interfaces::ReadWriteable;

pub fn construct_ppu() -> Ppu {
    let mut ppu = Ppu::new();

    const VRAM_BEGIN: u16 = 0x8000;
    const VRAM_END: u16 = 0xA000;

    let mut rng = rand::rng();
    // Fill the VRAM with random data
    for i in VRAM_BEGIN..VRAM_END {
        ppu.write(i, rng.random());
    }
    ppu.regs.lcdc.modify(ppu::regs::LCDC::ENABLE::On);
    ppu.regs
        .lcdc
        .modify(ppu::regs::LCDC::BG_AND_WINDOW_ENABLE::Enabled);
    ppu
}

pub fn construct_ppu_with_oam() -> Ppu {
    let mut ppu = construct_ppu();
    ppu.regs
        .lcdc
        .modify(ppu::regs::LCDC::WINDOW_ENABLE::Enabled);
    ppu.regs.lcdc.modify(ppu::regs::LCDC::OBJ_ENABLE::Enabled);

    const OAM_BEGIN: u16 = 0xFE00;
    const OAM_END: u16 = 0xFEA0;

    let mut rng = rand::rng();
    // Fill the OAM with random data
    for i in OAM_BEGIN..OAM_END {
        ppu.write(i, rng.random());
    }
    ppu.oam_scan();

    ppu
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("draw_line", |b| {
        b.iter_batched_ref(
            construct_ppu,
            |ppu| ppu.draw_line(),
            BatchSize::PerIteration,
        );
    });

    c.bench_function("draw_line_with_window", |b| {
        b.iter_batched_ref(
            || {
                let ppu = construct_ppu();
                ppu.regs
                    .lcdc
                    .modify(ppu::regs::LCDC::WINDOW_ENABLE::Enabled);
                ppu
            },
            |ppu| ppu.draw_line(),
            BatchSize::PerIteration,
        );
    });

    c.bench_function("draw_line_with_window_and_objects", |b| {
        b.iter_batched_ref(
            construct_ppu_with_oam,
            |ppu| ppu.draw_line(),
            BatchSize::PerIteration,
        );
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
