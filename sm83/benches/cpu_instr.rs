use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

use sm83::{core::Cpu, interrupts::Interrupts, memory::Memory};

struct FakeMemoryMap([u8; 65536]);

impl FakeMemoryMap {
    fn new() -> Self {
        Self([0; 65536])
    }
}

impl Memory for FakeMemoryMap {
    fn read(&self, address: sm83::memory::Address) -> u8 {
        self.0[address as usize]
    }

    fn write(&mut self, address: sm83::memory::Address, value: u8) {
        self.0[address as usize] = value;
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut mem = FakeMemoryMap::new();
    mem.0[0] = 0x77; // ld [hl], a
    mem.0[0x1234] = 0x0;

    c.bench_function("write_mem", |b| {
        b.iter_batched_ref(
            || {
                let mut cpu = Cpu::new();
                cpu.get_mut_regs().h_reg = 0x12;
                cpu.get_mut_regs().l_reg = 0x34;
                cpu.get_mut_regs().pc_reg = 0x0;
                cpu.get_mut_regs().a_reg = 0xC5;
                cpu
            },
            |cpu| {
                cpu.step(&mut mem, Interrupts::new());
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
