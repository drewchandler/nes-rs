use super::*;

const OFFICIAL_OPCODES: [u8; 151] = [
        0x00,0x01,0x05,0x06,0x08,0x09,0x0a,0x0d,0x0e,
        0x10,0x11,0x15,0x16,0x18,0x19,0x1d,0x1e,
        0x20,0x21,0x24,0x25,0x26,0x28,0x29,0x2a,0x2c,0x2d,0x2e,
        0x30,0x31,0x35,0x36,0x38,0x39,0x3d,0x3e,
        0x40,0x41,0x45,0x46,0x48,0x49,0x4a,0x4c,0x4d,0x4e,
        0x50,0x51,0x55,0x56,0x58,0x59,0x5d,0x5e,
        0x60,0x61,0x65,0x66,0x68,0x69,0x6a,0x6c,0x6d,0x6e,
        0x70,0x71,0x75,0x76,0x78,0x79,0x7d,0x7e,
        0x81,0x84,0x85,0x86,0x88,0x8a,0x8c,0x8d,0x8e,
        0x90,0x91,0x94,0x95,0x96,0x98,0x99,0x9a,0x9d,
        0xa0,0xa1,0xa2,0xa4,0xa5,0xa6,0xa8,0xa9,0xaa,0xac,0xad,0xae,
        0xb0,0xb1,0xb4,0xb5,0xb6,0xb8,0xb9,0xba,0xbc,0xbd,0xbe,
        0xc0,0xc1,0xc4,0xc5,0xc6,0xc8,0xc9,0xca,0xcc,0xcd,0xce,
        0xd0,0xd1,0xd5,0xd6,0xd8,0xd9,0xdd,0xde,
        0xe0,0xe1,0xe4,0xe5,0xe6,0xe8,0xe9,0xea,0xec,0xed,0xee,
        0xf0,0xf1,0xf5,0xf6,0xf8,0xf9,0xfd,0xfe,
    ];


#[derive(Debug, PartialEq)]
enum Access {
    Read(u16),
    Write(u16, u8),
}

struct Bus {
    mem: [u8; 65536],
    accesses: Vec<Access>,
}

impl Bus {
    fn new() -> Bus {
        Bus { mem: [0; 65536], accesses: Vec::new() }
    }
}

impl Interconnect for Bus {
    fn read_word(&mut self, addr: u16) -> u8 {
        self.accesses.push(Access::Read(addr));
        self.mem[addr as usize]
    }

    fn write_word(&mut self, addr: u16, value: u8) {
        self.accesses.push(Access::Write(addr, value));
        if addr != 0x2004 && addr != 0x4014 { self.mem[addr as usize] = value; }
    }

    fn read_double(&mut self, addr: u16) -> u16 {
        let low = self.read_word(addr) as u16;
        low | ((self.read_word(addr.wrapping_add(1)) as u16) << 8)
    }

    fn write_double(&mut self, addr: u16, value: u16) {
        self.write_word(addr, value as u8);
        self.write_word(addr.wrapping_add(1), (value >> 8) as u8);
    }
}

fn setup(program: &[u8]) -> (Cpu, Bus) {
    let mut cpu = Cpu::new();
    cpu.pc = 0x8000;
    cpu.sp = 0xfd;
    cpu.p = 0;
    let mut bus = Bus::new();
    bus.mem[0x8000..0x8000 + program.len()].copy_from_slice(program);
    (cpu, bus)
}

#[test]
fn binary_adc_and_sbc_exhaustive_including_decimal_flag() {
    // The 2A03 retains D but always computes binary arithmetic.
    for decimal in &[0, DECIMAL_MODE] {
        for a in 0..256i16 {
            for operand in 0..256i16 {
                for carry in 0..2i16 {
                    for subtract in &[false, true] {
                        let mut cpu = Cpu::new();
                        cpu.a = a as u8;
                        cpu.p = *decimal | carry as u8;
                        let result = if *subtract {
                            cpu.sbc(operand as u8);
                            a - operand - (1 - carry)
                        } else {
                            cpu.adc(operand as u8);
                            a + operand + carry
                        };
                        let byte = result as u8;
                        let signed = if *subtract {
                            (a as u8 as i8 as i16) - (operand as u8 as i8 as i16) - (1 - carry)
                        } else {
                            (a as u8 as i8 as i16) + (operand as u8 as i8 as i16) + carry
                        };
                        let expected_carry = if *subtract { result >= 0 } else { result > 255 };
                        let flags = *decimal
                            | if expected_carry { CARRY_FLAG } else { 0 }
                            | if byte == 0 { ZERO_FLAG } else { 0 }
                            | if signed < -128 || signed > 127 { OVERFLOW_FLAG } else { 0 }
                            | (byte & NEGATIVE_FLAG);
                        assert_eq!((cpu.a, cpu.p), (byte, flags),
                                   "a={} operand={} carry={} subtract={}", a, operand, carry, subtract);
                    }
                }
            }
        }
    }
}

#[test]
fn missing_official_opcodes_execute_with_indexing_and_timing() {
    let (mut cpu, mut bus) = setup(&[0x1d, 0xff, 0x20, 0x1e, 0xff, 0x20]);
    cpu.x = 1;
    cpu.a = 1;
    bus.mem[0x2100] = 0x80;
    assert_eq!(cpu.step(&mut bus), 5);
    assert_eq!(cpu.a, 0x81);
    assert_eq!(cpu.step(&mut bus), 7);
    assert_eq!(bus.mem[0x2100], 0);
    assert_eq!(cpu.p, ZERO_FLAG | CARRY_FLAG);
}

#[test]
fn all_official_opcodes_decode() {
    assert_eq!(OFFICIAL_OPCODES.len(), 151);
    for opcode in &OFFICIAL_OPCODES { Instruction::from_opcode(*opcode); }
}

#[test]
fn indirect_pointers_wrap_within_their_pages() {
    for opcode in &[0xa1, 0xb1] {
        let (mut cpu, mut bus) = setup(&[*opcode, 0xfe]);
        cpu.x = 1;
        cpu.y = 1;
        if *opcode == 0xb1 { bus.mem[0x8001] = 0xff; }
        bus.mem[0xff] = 0xff;
        bus.mem[0] = 0x12;
        bus.mem[0x100] = 0x56;
        bus.mem[0x12ff] = 0xab;
        bus.mem[0x1300] = 0xcd;
        assert_eq!(cpu.step(&mut bus), 6);
        assert_eq!(cpu.a, if *opcode == 0xa1 { 0xab } else { 0xcd });
    }
    let (mut cpu, mut bus) = setup(&[0x6c, 0xff, 0x12]);
    bus.mem[0x12ff] = 0x34;
    bus.mem[0x1200] = 0x56;
    bus.mem[0x1300] = 0x78;
    assert_eq!(cpu.step(&mut bus), 5);
    assert_eq!(cpu.pc, 0x5634);
}

#[test]
fn every_branch_accounts_for_taken_and_page_cross_cycles() {
    for &(opcode, flag, when_set) in &[
        (0x90, CARRY_FLAG, false), (0xb0, CARRY_FLAG, true),
        (0xf0, ZERO_FLAG, true), (0xd0, ZERO_FLAG, false),
        (0x30, NEGATIVE_FLAG, true), (0x10, NEGATIVE_FLAG, false),
        (0x70, OVERFLOW_FLAG, true), (0x50, OVERFLOW_FLAG, false),
    ] {
        for taken in &[false, true] {
            for &(pc, offset, target, extra) in &[
                (0x8000, 2, 0x8004, 0), (0x80fd, 1, 0x8100, 1),
                (0x80fe, 0xfd, 0x80fd, 1), (0xfffe, 0xff, 0xffff, 1),
            ] {
                let (mut cpu, mut bus) = setup(&[]);
                cpu.pc = pc;
                cpu.p = if *taken == when_set { flag } else { 0 };
                bus.mem[pc as usize] = opcode;
                bus.mem[pc.wrapping_add(1) as usize] = offset;
                assert_eq!(cpu.step(&mut bus), if *taken { 3 + extra } else { 2 });
                assert_eq!(cpu.pc, if *taken { target } else { pc.wrapping_add(2) });
            }
        }
    }
}

#[test]
fn indexed_reads_charge_only_page_crosses_and_stores_are_fixed() {
    for &(opcode, base_cycles) in &[(0xbd, 4), (0xb9, 4), (0xbc, 4), (0xbe, 4), (0x9d, 5), (0x99, 5)] {
        for low in &[0xfe, 0xff] {
            let (mut cpu, mut bus) = setup(&[opcode, *low, 0x20]);
            cpu.x = 1;
            cpu.y = 1;
            let store = opcode == 0x9d || opcode == 0x99;
            assert_eq!(cpu.step(&mut bus), base_cycles + if !store && *low == 0xff { 1 } else { 0 });
            let dummy = bus.accesses.iter().any(|a| *a == Access::Read(0x2000));
            assert_eq!(dummy, *low == 0xff);
        }
    }
    for &(opcode, expected) in &[(0xb1, 5), (0x91, 6)] {
        let (mut cpu, mut bus) = setup(&[opcode, 0x20]);
        bus.mem[0x20] = 0x10;
        cpu.y = 1;
        assert_eq!(cpu.step(&mut bus), expected);
    }
}

#[test]
fn stack_byte_and_word_operations_wrap_for_every_sp() {
    for sp in 0..256u16 {
        let (mut cpu, mut bus) = setup(&[]);
        cpu.sp = sp as u8;
        cpu.push_word(&mut bus, 0xab);
        assert_eq!(cpu.sp, (sp as u8).wrapping_sub(1));
        assert_eq!(cpu.pop_word(&mut bus), 0xab);
        assert_eq!(cpu.sp, sp as u8);
        bus.accesses.clear();
        cpu.push_double(&mut bus, 0x1234);
        assert_eq!(bus.accesses, vec![
            Access::Write(0x100 | sp, 0x12),
            Access::Write(0x100 | (sp as u8).wrapping_sub(1) as u16, 0x34),
        ]);
        assert_eq!(cpu.pop_double(&mut bus), 0x1234);
        assert_eq!(cpu.sp, sp as u8);
    }
}

#[test]
fn pc_indexed_addresses_and_returns_wrap() {
    let (mut cpu, mut bus) = setup(&[]);
    cpu.pc = 0xffff;
    bus.mem[0xffff] = 0xea;
    assert_eq!(cpu.step(&mut bus), 2);
    assert_eq!(cpu.pc, 0);
    for opcode in &[0xbd, 0xb9, 0xb1] {
        let (mut cpu, mut bus) = setup(&[*opcode, 0xff, 0xff]);
        bus.mem[0xff] = 0xff;
        bus.mem[0] = 0xff;
        cpu.x = 1;
        cpu.y = 1;
        cpu.step(&mut bus);
        assert_eq!(cpu.a, 0xff);
    }
    let (mut cpu, mut bus) = setup(&[0x60]);
    cpu.push_double(&mut bus, 0xffff);
    assert_eq!(cpu.step(&mut bus), 6);
    assert_eq!(cpu.pc, 0);
    let (mut cpu, mut bus) = setup(&[]);
    cpu.pc = 0xfffe;
    bus.mem[0xfffe] = 0x20;
    bus.mem[0xffff] = 0x34;
    bus.mem[0] = 0x12;
    assert_eq!(cpu.step(&mut bus), 6);
    assert_eq!(cpu.pc, 0x1234);
    assert_eq!(cpu.pop_double(&mut bus), 0);
}

#[test]
fn reset_preserves_registers_sets_i_and_decrements_sp() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    bus.mem[0xfffc] = 0x34;
    bus.mem[0xfffd] = 0x12;
    cpu.reset(&mut bus);
    assert_eq!((cpu.pc, cpu.sp, cpu.p, cpu.cycles), (0x1234, 0xfd, INTERUPT_DISABLE, 7));
    cpu.a = 0x12; cpu.x = 0x34; cpu.y = 0x56;
    cpu.p = DECIMAL_MODE | CARRY_FLAG;
    cpu.reset(&mut bus);
    assert_eq!((cpu.a, cpu.x, cpu.y, cpu.sp, cpu.p, cpu.cycles),
               (0x12, 0x34, 0x56, 0xfa, DECIMAL_MODE | CARRY_FLAG | INTERUPT_DISABLE, 14));
    assert!(!bus.accesses.iter().any(|a| match *a { Access::Write(_, _) => true, _ => false }));
}

#[test]
fn brk_and_nmi_stack_distinct_status_and_rti_restores_pc() {
    for is_nmi in &[false, true] {
        let (mut cpu, mut bus) = setup(&[0x00, 0xea]);
        cpu.p = CARRY_FLAG | DECIMAL_MODE;
        let vector = if *is_nmi { 0xfffa } else { 0xfffe };
        bus.mem[vector] = 0;
        bus.mem[vector + 1] = 0x90;
        bus.mem[0x9000] = 0x40;
        if *is_nmi {
            cpu.nmi();
            assert_eq!(cpu.pc, 0x8000); // Entry waits until step.
            assert_eq!(cpu.sp, 0xfd);
        }
        assert_eq!(cpu.step(&mut bus), 7);
        assert_eq!(bus.accesses.len(), 7);
        assert_eq!(cpu.pc, 0x9000);
        assert_eq!(cpu.p, CARRY_FLAG | DECIMAL_MODE | INTERUPT_DISABLE);
        assert_eq!(bus.mem[0x1fb], CARRY_FLAG | DECIMAL_MODE | UNUSED_FLAG
                   | if *is_nmi { 0 } else { BREAK_COMMAND });
        assert_eq!(bus.mem[0x1fc], if *is_nmi { 0 } else { 2 });
        assert_eq!(bus.mem[0x1fd], 0x80);
        assert_eq!(cpu.step(&mut bus), 6);
        assert_eq!(cpu.pc, if *is_nmi { 0x8000 } else { 0x8002 });
        assert_eq!(cpu.p, CARRY_FLAG | DECIMAL_MODE);
        assert_eq!(cpu.cycles, 13);
        assert_eq!(bus.accesses.len(), 13);
        assert_eq!(cpu.sp, 0xfd);
    }
}

#[test]
fn php_and_plp_do_not_latch_break_or_unused_bits() {
    let (mut cpu, mut bus) = setup(&[0x08, 0x28, 0x08]);
    cpu.p = OVERFLOW_FLAG;
    cpu.step(&mut bus);
    assert_eq!(bus.mem[0x1fd], OVERFLOW_FLAG | BREAK_COMMAND | UNUSED_FLAG);
    cpu.step(&mut bus);
    assert_eq!(cpu.p, OVERFLOW_FLAG);
    cpu.step(&mut bus);
    assert_eq!(bus.mem[0x1fd], OVERFLOW_FLAG | BREAK_COMMAND | UNUSED_FLAG);
}

#[test]
fn rmw_instructions_write_old_value_before_new_value() {
    for &(opcode, input, output) in &[
        (0x0e, 0x81, 2), (0x4e, 3, 1), (0x2e, 0x80, 0),
        (0x6e, 3, 1), (0xce, 0, 0xff), (0xee, 0xff, 0),
    ] {
        let (mut cpu, mut bus) = setup(&[opcode, 0, 0x20]);
        bus.mem[0x2000] = input;
        assert_eq!(cpu.step(&mut bus), 6);
        assert_eq!(&bus.accesses[3..], &[
            Access::Read(0x2000), Access::Write(0x2000, input), Access::Write(0x2000, output),
        ]);
    }
}

#[test]
fn dma_copies_every_page_and_accounts_for_both_parities() {
    for page in 0..256u16 {
        for parity in 0..2u64 {
            let (mut cpu, mut bus) = setup(&[0x8d, 0x14, 0x40]);
            // Keep DMA data separate from the instruction, even for page $80.
            cpu.pc = if page == 0x60 { 0x6100 } else { 0x6000 };
            let pc = cpu.pc as usize;
            bus.mem[pc..pc + 3].copy_from_slice(&[0x8d, 0x14, 0x40]);
            for offset in 0..256usize { bus.mem[(page as usize) * 256 + offset] = offset as u8; }
            cpu.a = page as u8;
            cpu.cycles = parity;
            assert_eq!(cpu.step(&mut bus), 517 + parity as u16);
            let writes: Vec<_> = bus.accesses.iter().filter_map(|a| match *a {
                Access::Write(0x2004, value) => Some(value), _ => None,
            }).collect();
            assert_eq!(writes, (0..256u16).map(|v| v as u8).collect::<Vec<_>>());
            assert_eq!(cpu.cycles, parity + 517 + parity);
            assert_eq!(bus.accesses.len(), (517 + parity) as usize);
        }
    }
}

#[test]
fn dma_parity_includes_instruction_and_interrupt_cycles() {
    let (mut cpu, mut bus) = setup(&[0x85, 0x20, 0x8d, 0x14, 0x40]);
    assert_eq!(cpu.step(&mut bus), 3);
    assert_eq!(cpu.step(&mut bus), 518);
    let (mut cpu, mut bus) = setup(&[0x8d, 0x14, 0x40]);
    bus.mem[0xfffa] = 0;
    bus.mem[0xfffb] = 0x80;
    cpu.nmi();
    assert_eq!(cpu.step(&mut bus), 7);
    assert_eq!(cpu.step(&mut bus), 518);
}

#[test]
fn rmw_dma_uses_final_write_and_stalls_once() {
    let (mut cpu, mut bus) = setup(&[0xee, 0x14, 0x40]);
    bus.mem[0x4014] = 2;
    bus.mem[0x200] = 0xaa;
    bus.mem[0x300] = 0xbb;
    assert_eq!(cpu.step(&mut bus), 519);
    let writes: Vec<_> = bus.accesses.iter().filter_map(|a| match *a {
        Access::Write(0x2004, v) => Some(v), _ => None,
    }).collect();
    assert_eq!(writes.len(), 256);
    assert_eq!(writes[0], 0xbb);
}

#[test]
fn dummy_reads_follow_indexed_and_branch_bus_addresses() {
    let (mut cpu, mut bus) = setup(&[0xa1, 0xff]);
    cpu.x = 1;
    bus.mem[0] = 0x34;
    bus.mem[1] = 0x12;
    cpu.step(&mut bus);
    assert_eq!(bus.accesses, vec![Access::Read(0x8000), Access::Read(0x8001),
        Access::Read(0xff), Access::Read(0), Access::Read(1), Access::Read(0x1234)]);
    let (mut cpu, mut bus) = setup(&[0x95, 0xff]);
    cpu.x = 1;
    cpu.a = 0xab;
    cpu.step(&mut bus);
    assert_eq!(bus.accesses, vec![Access::Read(0x8000), Access::Read(0x8001),
        Access::Read(0xff), Access::Write(0, 0xab)]);
    let (mut cpu, mut bus) = setup(&[]);
    cpu.pc = 0x80fd;
    bus.mem[0x80fd] = 0xd0;
    bus.mem[0x80fe] = 1;
    cpu.step(&mut bus);
    assert_eq!(bus.accesses, vec![Access::Read(0x80fd), Access::Read(0x80fe),
        Access::Read(0x80ff), Access::Read(0x8000)]);
}

#[test]
fn official_instruction_bus_access_counts_match_reported_cycles() {
    for opcode in &OFFICIAL_OPCODES {
        for index in &[0, 1, 0xff] {
            let (mut cpu, mut bus) = setup(&[*opcode, 0xff, 0x02]);
            cpu.x = *index;
            cpu.y = *index;
            bus.mem[0xff] = 0xff;
            bus.mem[0] = 2;
            let cycles = cpu.step(&mut bus);
            assert_eq!(bus.accesses.len(), cycles as usize,
                       "opcode={:02x} index={}", opcode, index);
        }
    }
}

fn mapped_bus(program: &[u8]) -> ::interconnect::MemoryMappingInterconnect {
    let mut prg = vec![0xea; 16384];
    prg[..program.len()].copy_from_slice(program);
    prg[0x3ffc] = 0;
    prg[0x3ffd] = 0xc0;
    ::interconnect::MemoryMappingInterconnect::new(::rom::Rom {
        mapper: 2, prg_rom: vec![prg], chr_rom: vec![],
    })
}

#[test]
fn indexed_dummy_reads_work_with_real_write_only_ppu_ports() {
    let mut bus = mapped_bus(&[0xa2, 0, 0xa9, 0x80, 0x9d, 0, 0x20]);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);
    assert_eq!(cpu.step(&mut bus), 2);
    assert_eq!(cpu.step(&mut bus), 2);
    assert_eq!(cpu.step(&mut bus), 5);
    assert_eq!(bus.read_word(0x2000), 0x80);
    // CPU instruction fetches must not replace the independent PPU I/O latch.
    bus.read_word(0xc000);
    assert_eq!(bus.read_word(0x2008), 0x80);
    bus.write_word(0x2000, 0x1b);
    assert_eq!(bus.read_word(0x2002) & 0x1f, 0x1b);
    assert_eq!(bus.read_word(0x2001) & 0x1f, 0x1b);
}

#[test]
fn dma_and_rmw_dma_work_with_real_mapped_bus() {
    let mut bus = mapped_bus(&[0xa9, 0xff, 0x8d, 0x14, 0x40, 0xee, 0x14, 0x40]);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);
    assert_eq!(cpu.step(&mut bus), 2);
    assert_eq!(cpu.step(&mut bus), 518);
    // A full page wraps OAMADDR to zero and retains the first source byte.
    assert_eq!(bus.read_word(0x2004), 0xea);
    assert_eq!(cpu.step(&mut bus), 520);
    // Reads from an unpopulated cartridge region retain CPU open bus.
    bus.write_word(0, 0xab);
    assert_eq!(bus.read_word(0x6000), 0xab);
    assert_eq!(bus.read_word(0x4014), 0xab);
}
