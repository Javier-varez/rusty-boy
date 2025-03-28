use serde::Deserialize;
use sm83::{
    core::{Cpu, Cycles, ExitReason, Flag, Flags, Registers},
    interrupts::{Interrupt, Interrupts},
    memory::{Address, Memory},
};

include!(concat!(env!("OUT_DIR"), "/generated_tests.rs"));

struct MemoryImpl {
    memory: Vec<u8>,
}

impl Memory for MemoryImpl {
    fn read(&self, address: Address) -> u8 {
        self.memory[address as usize]
    }
    fn write(&mut self, address: Address, value: u8) {
        self.memory[address as usize] = value;
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    #[serde(default)]
    a: u8,
    #[serde(default)]
    b: u8,
    #[serde(default)]
    c: u8,
    #[serde(default)]
    d: u8,
    #[serde(default)]
    e: u8,
    #[serde(default)]
    h: u8,
    #[serde(default)]
    l: u8,
    #[serde(default)]
    sp: u16,
    #[serde(default)]
    pc: u16,
    #[serde(default)]
    irq_en: bool,
    #[serde(default)]
    flags: Vec<char>,
    // Map of base addr + data at addr
    #[serde(default)]
    memory: Option<std::collections::HashMap<String, Vec<u8>>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Program {
    #[serde(default)]
    base: Address,
    instructions: Vec<u8>,
}

/// The exit reason of the CPU after stepping an instruction.
#[derive(Debug, Deserialize, Eq, PartialEq, Default)]
pub enum StepExitReason {
    /// The step of the instruction concluded successfully, and took the given number of clock cycles.
    #[default]
    Step,
    /// An interrupt was taken, and and took the given number of clock cycles.
    InterruptTaken,
    /// The CPU is stopped, and executed the given number of cycles
    Stop,
    /// The CPU is halted, and executed the given number of cycles
    Halt,
    /// The CPU attempted to execute an illegal OpCode.
    IllegalOpcode,
}

#[derive(Debug, Deserialize)]
enum TestInterrupt {
    Vblank,
    Lcd,
    Timer,
    Serial,
    Joypad,
}

impl<T: AsRef<Interrupt>> From<T> for TestInterrupt {
    fn from(value: T) -> Self {
        match value.as_ref() {
            Interrupt::Vblank => Self::Vblank,
            Interrupt::Lcd => Self::Lcd,
            Interrupt::Timer => Self::Timer,
            Interrupt::Serial => Self::Serial,
            Interrupt::Joypad => Self::Joypad,
        }
    }
}

impl From<&TestInterrupt> for Interrupt {
    fn from(value: &TestInterrupt) -> Self {
        match value {
            TestInterrupt::Vblank => Self::Vblank,
            TestInterrupt::Lcd => Self::Lcd,
            TestInterrupt::Timer => Self::Timer,
            TestInterrupt::Serial => Self::Serial,
            TestInterrupt::Joypad => Self::Joypad,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TestInterrupts {
    cycle: usize,
    triggers: Vec<TestInterrupt>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TestInterruptAck {
    cycle: usize,
    ack: TestInterrupt,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Test {
    entry_state: State,
    exit_state: State,
    program: Program,
    cycles: usize,
    #[serde(default)]
    exit_reason: StepExitReason,
    #[serde(default)]
    interrupt_triggers: Vec<TestInterrupts>,
    #[serde(default)]
    interrupt_acknowledges: Vec<TestInterruptAck>,
}

fn parse_u16(string: &str) -> u16 {
    if string.starts_with("0x") {
        u16::from_str_radix(string.trim_start_matches("0x"), 16).unwrap()
    } else {
        string.parse::<u16>().unwrap()
    }
}

fn build_memory(program: &Program, state: &State) -> Vec<u8> {
    let mut data = vec![0; 0x1_0000];

    if let Some(memory) = &state.memory {
        for (base, slice) in memory {
            let base = parse_u16(base);
            let dest = data.iter_mut().skip(base as usize);
            for (dest, src) in dest.zip(slice.iter()) {
                *dest = *src;
            }
        }
    }

    data.iter_mut()
        .skip(program.base as usize)
        .zip(program.instructions.iter())
        .for_each(|(d, s)| *d = *s);

    data
}

fn translate_flags(flags: &[char]) -> Flags {
    flags.iter().fold(Flags::new(), |flags, flag| {
        flags.with(
            match flag {
                'N' => Flag::N,
                'Z' => Flag::Z,
                'C' => Flag::C,
                'H' => Flag::H,
                _ => panic!("Invalid flag {}", flag),
            },
            true,
        )
    })
}

fn translate_exit_reason(exit_reason: ExitReason) -> StepExitReason {
    match exit_reason {
        ExitReason::Halt(_) => StepExitReason::Halt,
        ExitReason::Step(_) => StepExitReason::Step,
        ExitReason::Stop(_) => StepExitReason::Stop,
        ExitReason::InterruptTaken(_, _) => StepExitReason::InterruptTaken,
        ExitReason::IllegalOpcode => StepExitReason::IllegalOpcode,
    }
}

fn translate_interrupts(interrupts: &[TestInterrupt]) -> Interrupts {
    interrupts
        .iter()
        .map(|i| {
            let i: Interrupt = i.into();
            i
        })
        .fold(Interrupts::new(), |r, i| r | i)
}

fn run_test(test_suite_name: &str, tests_str: &str) {
    let tests: std::collections::HashMap<String, Test> =
        toml::from_str(tests_str).expect("Invalid tests");

    for (test_case, test) in tests {
        let memory = build_memory(&test.program, &test.entry_state);
        let mut memory_interface = MemoryImpl { memory };

        let mut cpu = Cpu::new();

        // Set initial state
        *cpu.get_mut_regs() = Registers {
            a_reg: test.entry_state.a,
            b_reg: test.entry_state.b,
            c_reg: test.entry_state.c,
            d_reg: test.entry_state.d,
            e_reg: test.entry_state.e,
            h_reg: test.entry_state.h,
            l_reg: test.entry_state.l,
            sp_reg: test.entry_state.sp,
            pc_reg: test.entry_state.pc,
            irq_en: test.entry_state.irq_en,
            flags: translate_flags(&test.entry_state.flags),
        };

        let mut active_interrupts = Interrupts::new();
        let mut executed_cycles = Cycles::new(0);
        let exit_reason = loop {
            let interrupts = if let Some(interrupts) = test
                .interrupt_triggers
                .iter()
                .find(|test_interrupts| Cycles::new(test_interrupts.cycle) == executed_cycles)
            {
                translate_interrupts(&interrupts.triggers)
            } else {
                Interrupts::new()
            };
            active_interrupts = active_interrupts | interrupts;
            println!("Active interrupts at {executed_cycles:?} = {active_interrupts:?}");
            let reason = cpu.step(&mut memory_interface, active_interrupts);

            let (step_cycles, ack) = match reason {
                ExitReason::Halt(cycles) | ExitReason::Step(cycles) | ExitReason::Stop(cycles) => {
                    (cycles, None)
                }
                ExitReason::InterruptTaken(cycles, ack) => {
                    println!("Active irqs {active_interrupts:?}, ack {ack:?}");
                    active_interrupts = active_interrupts.acknowledge(ack);
                    (cycles, Some(ack))
                }
                ExitReason::IllegalOpcode => {
                    break reason;
                }
            };
            executed_cycles = executed_cycles + step_cycles;
            println!("Executed {step_cycles:?}");

            let expected_ack = test
                .interrupt_acknowledges
                .iter()
                .find(|i| Cycles::new(i.cycle) == executed_cycles)
                .map(|i| {
                    let i: Interrupt = (&i.ack).into();
                    i
                });

            assert_eq!(
                expected_ack, ack,
                "interrupt acknowledge mismatch found (expected != ack) at {:?} in test `{}::{}`",
                executed_cycles, test_suite_name, test_case
            );

            if executed_cycles >= Cycles::new(test.cycles) {
                break reason;
            }
        };

        let expected_cpu_state = Registers {
            a_reg: test.exit_state.a,
            b_reg: test.exit_state.b,
            c_reg: test.exit_state.c,
            d_reg: test.exit_state.d,
            e_reg: test.exit_state.e,
            h_reg: test.exit_state.h,
            l_reg: test.exit_state.l,
            sp_reg: test.exit_state.sp,
            pc_reg: test.exit_state.pc,
            irq_en: test.exit_state.irq_en,
            flags: translate_flags(&test.exit_state.flags),
        };

        assert_eq!(
            expected_cpu_state,
            *cpu.get_regs(),
            "Unexpected CPU state in test `{}::{}` (expected != actual)",
            test_suite_name,
            test_case
        );

        let expected_memory = build_memory(&test.program, &test.exit_state);
        assert_eq!(
            expected_memory, memory_interface.memory,
            "Unexpected memory in test `{}::{}` (expected != actual)",
            test_suite_name, test_case
        );

        assert_eq!(
            Cycles::new(test.cycles),
            executed_cycles,
            "Did not run for the expected number of cycles in test `{}::{}` (expected != actual)",
            test_suite_name,
            test_case
        );

        assert_eq!(
            test.exit_reason,
            translate_exit_reason(exit_reason),
            "Did not run finish with the expected exit reason in test `{}::{}` (expected != actual)",
            test_suite_name,
            test_case
        );
    }
}
