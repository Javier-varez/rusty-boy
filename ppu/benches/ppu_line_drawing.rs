use criterion::{criterion_group, criterion_main, Criterion};

use ppu::Ppu;
use tock_registers::interfaces::ReadWriteable;

fn criterion_benchmark(c: &mut Criterion) {
    let mut ppu = Ppu::new();

    const VRAM_BEGIN: u16 = 0x8000;
    const VRAM_END: u16 = 0xA000;
    // Fill the VRAM with random data
    for i in VRAM_BEGIN..VRAM_END {
        ppu.write(i, 0);
    }
    ppu.regs.lcdc.modify(ppu::regs::LCDC::ENABLE::On);

    c.bench_function("draw_line", |b| b.iter(|| ppu.draw_line()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
