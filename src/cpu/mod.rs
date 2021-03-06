mod instruction;

use interconnect::Interconnect;
use self::instruction::{Op, AddressingMode, Instruction};

pub const CARRY_FLAG: u8 = 0x01;
pub const ZERO_FLAG: u8 = 0x02;
pub const INTERUPT_DISABLE: u8 = 0x04;
pub const DECIMAL_MODE: u8 = 0x08;
pub const BREAK_COMMAND: u8 = 0x10;
pub const OVERFLOW_FLAG: u8 = 0x40;
pub const NEGATIVE_FLAG: u8 = 0x80;

pub const RESET_VECTOR: u16 = 0xfffc;
pub const BREAK_VECTOR: u16 = 0xfffe;

pub const STACK_END: u16 = 0x100;

#[cfg_attr(rustfmt, rustfmt_skip)]
static CYCLES: [u16; 256] = [
    7,6,2,8,3,3,5,5,3,2,2,2,4,4,6,6,
    2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
    6,6,2,8,3,3,5,5,4,2,2,2,4,4,6,6,
    2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
    6,6,2,8,3,3,5,5,3,2,2,2,3,4,6,6,
    2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
    6,6,2,8,3,3,5,5,4,2,2,2,5,4,6,6,
    2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
    2,6,2,6,3,3,3,3,2,2,2,2,4,4,4,4,
    2,6,2,6,4,4,4,4,2,5,2,5,5,5,5,5,
    2,6,2,6,3,3,3,3,2,2,2,2,4,4,4,4,
    2,5,2,5,4,4,4,4,2,4,2,4,4,4,4,4,
    2,6,2,8,3,3,5,5,2,2,2,2,4,4,6,6,
    2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
    2,6,3,8,3,3,5,5,2,2,2,2,4,4,6,6,
    2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
];

pub struct Cpu {
    a: u8,
    p: u8,
    pc: u16,
    sp: u8,
    x: u8,
    y: u8,
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            a: 0,
            p: 0,
            pc: 0,
            sp: 0xfd,
            x: 0,
            y: 0,
        }
    }

    pub fn reset(&mut self, interconnect: &mut Interconnect) {
        self.pc = interconnect.read_double(RESET_VECTOR);
    }

    pub fn nmi(&mut self, interconnect: &mut Interconnect) {
        let pc = self.pc;
        self.push_double(interconnect, pc);
        let p = self.p;
        self.push_word(interconnect, p);
        self.pc = interconnect.read_double(0xfffa);
    }

    pub fn step(&mut self, interconnect: &mut Interconnect) -> u16 {
        let opcode = self.read_pc(interconnect);
        let Instruction(op, am) = Instruction::from_opcode(opcode);

        macro_rules! with_value {
            ($f:expr) => ({
                let value = self.value_for(interconnect, &am);
                $f(value)
            })
        }

        macro_rules! with_addr {
            ($f:expr) => ({
                let addr = self.addr_for(interconnect, &am);
                $f(addr)
            })
        }

        let mut dma_performed = false;

        match op {
            Op::Adc => with_value!(|value| self.adc(value)),
            Op::And => with_value!(|value| self.and(value)),
            Op::Asl => self.asl(interconnect, am),
            Op::Bcc => with_addr!(|addr| self.bcc(addr)),
            Op::Bcs => with_addr!(|addr| self.bcs(addr)),
            Op::Beq => with_addr!(|addr| self.beq(addr)),
            Op::Bit => with_value!(|value| self.bit(value)),
            Op::Bmi => with_addr!(|addr| self.bmi(addr)),
            Op::Bne => with_addr!(|addr| self.bne(addr)),
            Op::Bpl => with_addr!(|addr| self.bpl(addr)),
            Op::Brk => self.brk(interconnect),
            Op::Bvc => with_addr!(|addr| self.bvc(addr)),
            Op::Bvs => with_addr!(|addr| self.bvs(addr)),
            Op::Clc => self.clc(),
            Op::Cld => self.cld(),
            Op::Cli => self.cli(),
            Op::Clv => self.clv(),
            Op::Cmp => with_value!(|value| self.cmp(value)),
            Op::Cpx => with_value!(|value| self.cpx(value)),
            Op::Cpy => with_value!(|value| self.cpy(value)),
            Op::Dec => with_addr!(|addr| self.dec(interconnect, addr)),
            Op::Dex => self.dex(),
            Op::Dey => self.dey(),
            Op::Eor => with_value!(|value| self.eor(value)),
            Op::Inc => with_addr!(|addr| self.inc(interconnect, addr)),
            Op::Inx => self.inx(),
            Op::Iny => self.iny(),
            Op::Jmp => with_addr!(|addr| self.jmp(addr)),
            Op::Jsr => with_addr!(|addr| self.jsr(interconnect, addr)),
            Op::Lda => with_value!(|value| self.lda(value)),
            Op::Ldx => with_value!(|value| self.ldx(value)),
            Op::Ldy => with_value!(|value| self.ldy(value)),
            Op::Lsr => self.lsr(interconnect, am),
            Op::Nop => {}
            Op::Ora => with_value!(|value| self.ora(value)),
            Op::Pha => self.pha(interconnect),
            Op::Php => self.php(interconnect),
            Op::Pla => self.pla(interconnect),
            Op::Plp => self.plp(interconnect),
            Op::Rol => self.rol(interconnect, am),
            Op::Ror => self.ror(interconnect, am),
            Op::Rti => self.rti(interconnect),
            Op::Rts => self.rts(interconnect),
            Op::Sbc => with_value!(|value| self.sbc(value)),
            Op::Sec => self.sec(),
            Op::Sed => self.sed(),
            Op::Sei => self.sei(),
            Op::Sta => {
                dma_performed = with_addr!(|addr| self.sta(interconnect, addr));
            }
            Op::Stx => {
                dma_performed = with_addr!(|addr| self.stx(interconnect, addr));
            }
            Op::Sty => {
                dma_performed = with_addr!(|addr| self.sty(interconnect, addr));
            }
            Op::Tax => self.tax(),
            Op::Tay => self.tay(),
            Op::Tsx => self.tsx(),
            Op::Txa => self.txa(),
            Op::Tya => self.tya(),
            Op::Txs => self.txs(),
        }

        if dma_performed {
            512 + CYCLES[opcode as usize]
        } else {
            CYCLES[opcode as usize]
        }
    }

    fn value_for(&mut self, interconnect: &mut Interconnect, am: &AddressingMode) -> u8 {
        match *am {
            AddressingMode::Immediate => self.read_pc(interconnect),
            AddressingMode::Absolute |
            AddressingMode::AbsoluteX |
            AddressingMode::AbsoluteY |
            AddressingMode::ZeroPage |
            AddressingMode::ZeroPageX |
            AddressingMode::ZeroPageY |
            AddressingMode::Indirect |
            AddressingMode::IndirectX |
            AddressingMode::IndirectY => {
                let addr = self.addr_for(interconnect, am);
                interconnect.read_word(addr)
            }
            _ => panic!("Unimplemented addressing mode: {:?}", am),
        }
    }

    fn addr_for(&mut self, interconnect: &mut Interconnect, am: &AddressingMode) -> u16 {
        match *am {
            AddressingMode::Absolute => {
                let lower = self.read_pc(interconnect);
                let higher = self.read_pc(interconnect);
                ((higher as u16) << 8) + lower as u16
            }
            AddressingMode::Indirect => {
                let addr = self.addr_for(interconnect, &AddressingMode::Absolute);
                interconnect.read_double(addr)
            }
            AddressingMode::IndirectX => {
                let zero_page_addr = self.read_pc(interconnect);
                interconnect.read_double(zero_page_addr.overflowing_add(self.x).0 as u16)
            }
            AddressingMode::IndirectY => {
                let zero_page_addr = self.read_pc(interconnect);
                let addr = interconnect.read_double(zero_page_addr as u16);
                addr + self.y as u16
            }
            AddressingMode::ZeroPage => self.read_pc(interconnect) as u16,
            AddressingMode::ZeroPageX => {
                let zero_page_addr = self.read_pc(interconnect);
                zero_page_addr.overflowing_add(self.x).0 as u16 & 0xff
            }
            AddressingMode::ZeroPageY => {
                let zero_page_addr = self.read_pc(interconnect);
                zero_page_addr.overflowing_add(self.y).0 as u16 & 0xff
            }
            AddressingMode::AbsoluteX => {
                self.addr_for(interconnect, &AddressingMode::Absolute) + self.x as u16
            }
            AddressingMode::AbsoluteY => {
                self.addr_for(interconnect, &AddressingMode::Absolute) + self.y as u16
            }
            AddressingMode::Relative => {
                let offset = self.read_pc(interconnect) as i8;
                self.pc.overflowing_add(offset as u16).0
            }
            _ => panic!("Unimplemented addressing mode: {:?}", am),
        }
    }


    fn read_pc(&mut self, interconnect: &mut Interconnect) -> u8 {
        let value = interconnect.read_word(self.pc);
        self.pc += 1;
        value
    }

    fn write_word(&self, interconnect: &mut Interconnect, addr: u16, value: u8) -> bool {
        if addr == 0x4014 {
            let dma_start = (value as u16) << 8;
            for addr in dma_start..dma_start + 256 {
                let value = interconnect.read_word(addr);
                interconnect.write_word(0x2004, value);
            }
            true
        } else {
            interconnect.write_word(addr, value);
            false
        }
    }

    fn carry_flag(&self) -> bool {
        self.p & CARRY_FLAG != 0
    }

    fn set_carry_flag(&mut self, value: bool) {
        self.p = if value {
            self.p | CARRY_FLAG
        } else {
            self.p & !CARRY_FLAG
        };
    }

    fn zero_flag(&self) -> bool {
        self.p & ZERO_FLAG != 0
    }

    fn set_zero_flag(&mut self, value: bool) {
        self.p = if value {
            self.p | ZERO_FLAG
        } else {
            self.p & !ZERO_FLAG
        };
    }

    fn interrupt_disable(&self) -> bool {
        self.p & INTERUPT_DISABLE != 0
    }

    fn set_interrupt_disable(&mut self, value: bool) {
        self.p = if value {
            self.p | INTERUPT_DISABLE
        } else {
            self.p & !INTERUPT_DISABLE
        };
    }

    fn decimal_mode(&self) -> bool {
        self.p & DECIMAL_MODE != 0
    }

    fn set_decimal_mode(&mut self, value: bool) {
        self.p = if value {
            self.p | DECIMAL_MODE
        } else {
            self.p & !DECIMAL_MODE
        };
    }

    fn break_command(&self) -> bool {
        self.p & BREAK_COMMAND != 0
    }

    fn set_break_command(&mut self, value: bool) {
        self.p = if value {
            self.p | BREAK_COMMAND
        } else {
            self.p & !BREAK_COMMAND
        }
    }

    fn overflow_flag(&self) -> bool {
        self.p & OVERFLOW_FLAG != 0
    }

    fn set_overflow_flag(&mut self, value: bool) {
        self.p = if value {
            self.p | OVERFLOW_FLAG
        } else {
            self.p & !OVERFLOW_FLAG
        };
    }

    fn negative_flag(&self) -> bool {
        self.p & NEGATIVE_FLAG != 0
    }

    fn set_negative_flag(&mut self, value: bool) {
        self.p = if value {
            self.p | NEGATIVE_FLAG
        } else {
            self.p & !NEGATIVE_FLAG
        };
    }

    fn set_zn(&mut self, value: u8) -> u8 {
        self.set_zero_flag(value == 0);
        self.set_negative_flag(value & 0x80 != 0);
        value
    }

    fn compare(&mut self, a: u8, b: u8) {
        self.set_zn(a.overflowing_sub(b).0);
        self.set_carry_flag(a >= b);
    }

    fn adc(&mut self, value: u8) {
        let a = self.a;
        let carry_flag = if self.carry_flag() { 1 } else { 0 };
        let (result, carry) = a.overflowing_add(value);
        let (result, carry2) = result.overflowing_add(carry_flag);

        self.a = self.set_zn(result);
        self.set_overflow_flag(((a ^ value) & 0x80 != 0x80) && ((a ^ result) & 0x80 == 0x80));
        self.set_carry_flag(carry || carry2);
    }

    fn and(&mut self, value: u8) {
        let a = self.a;
        self.a = self.set_zn(a & value);
    }

    fn asl(&mut self, interconnect: &mut Interconnect, am: AddressingMode) {
        if let AddressingMode::Accumulator = am {
            let a = self.a;
            self.a = self.arithmetic_shift_left(a);
        } else {
            let addr = self.addr_for(interconnect, &am);
            let value = interconnect.read_word(addr);
            let result = self.arithmetic_shift_left(value);
            interconnect.write_word(addr, result);
        }
    }

    fn bcc(&mut self, addr: u16) {
        if !self.carry_flag() {
            self.pc = addr;
        }
    }

    fn bcs(&mut self, addr: u16) {
        if self.carry_flag() {
            self.pc = addr;
        }
    }

    fn beq(&mut self, addr: u16) {
        if self.zero_flag() {
            self.pc = addr;
        }
    }

    fn bit(&mut self, value: u8) {
        let a = self.a;
        self.set_zero_flag(a & value == 0);
        self.set_overflow_flag(value & 0x40 != 0);
        self.set_negative_flag(value & 0x80 != 0);
    }

    fn bmi(&mut self, addr: u16) {
        if self.negative_flag() {
            self.pc = addr;
        }
    }

    fn bne(&mut self, addr: u16) {
        if !self.zero_flag() {
            self.pc = addr;
        }
    }

    fn bpl(&mut self, addr: u16) {
        if !self.negative_flag() {
            self.pc = addr;
        }
    }

    fn brk(&mut self, interconnect: &mut Interconnect) {
        let pc = self.pc;
        self.push_double(interconnect, pc);
        let p = self.p;
        self.push_word(interconnect, p);
        self.pc = interconnect.read_double(BREAK_VECTOR);
        self.set_break_command(true);
    }

    fn bvc(&mut self, addr: u16) {
        if !self.overflow_flag() {
            self.pc = addr;
        }
    }

    fn bvs(&mut self, addr: u16) {
        if self.overflow_flag() {
            self.pc = addr;
        }
    }

    fn clc(&mut self) {
        self.set_carry_flag(false);
    }

    fn cld(&mut self) {
        self.set_decimal_mode(false);
    }

    fn cli(&mut self) {
        self.set_interrupt_disable(false);
    }

    fn clv(&mut self) {
        self.set_overflow_flag(false);
    }

    fn cmp(&mut self, value: u8) {
        let a = self.a;
        self.compare(a, value);
    }

    fn cpx(&mut self, value: u8) {
        let x = self.x;
        self.compare(x, value);
    }

    fn cpy(&mut self, value: u8) {
        let y = self.y;
        self.compare(y, value);
    }

    fn dec(&mut self, interconnect: &mut Interconnect, addr: u16) {
        let value = self.set_zn(interconnect.read_word(addr).overflowing_sub(1).0);
        interconnect.write_word(addr, value);
    }

    fn dex(&mut self) {
        let x = self.x;
        self.x = self.set_zn(x.overflowing_sub(1).0);
    }

    fn dey(&mut self) {
        let y = self.y;
        self.y = self.set_zn(y.overflowing_sub(1).0);
    }

    fn eor(&mut self, value: u8) {
        let a = self.a;
        self.a = self.set_zn(a ^ value);
    }

    fn inc(&mut self, interconnect: &mut Interconnect, addr: u16) {
        let value = self.set_zn(interconnect.read_word(addr).overflowing_add(1).0);
        interconnect.write_word(addr, value);
    }

    fn inx(&mut self) {
        let x = self.x;
        self.x = self.set_zn(x.overflowing_add(1).0);
    }

    fn iny(&mut self) {
        let y = self.y;
        self.y = self.set_zn(y.overflowing_add(1).0);
    }

    fn jmp(&mut self, addr: u16) {
        self.pc = addr;
    }

    fn jsr(&mut self, interconnect: &mut Interconnect, addr: u16) {
        let return_addr = self.pc - 1;
        self.push_double(interconnect, return_addr);
        self.pc = addr;
    }

    fn lda(&mut self, value: u8) {
        self.a = self.set_zn(value);
    }

    fn ldx(&mut self, value: u8) {
        self.x = self.set_zn(value);
    }

    fn ldy(&mut self, value: u8) {
        self.y = self.set_zn(value);
    }

    fn lsr(&mut self, interconnect: &mut Interconnect, am: AddressingMode) {
        if let AddressingMode::Accumulator = am {
            let a = self.a;
            self.a = self.logical_shift_right(a);
        } else {
            let addr = self.addr_for(interconnect, &am);
            let value = interconnect.read_word(addr);
            let result = self.logical_shift_right(value);
            interconnect.write_word(addr, result);
        }
    }

    fn ora(&mut self, value: u8) {
        let a = self.a;
        self.a = self.set_zn(a | value);
    }

    fn pha(&mut self, interconnect: &mut Interconnect) {
        let a = self.a;
        self.push_word(interconnect, a);
    }

    fn php(&mut self, interconnect: &mut Interconnect) {
        let p = self.p;
        self.push_word(interconnect, p);
    }

    fn pla(&mut self, interconnect: &mut Interconnect) {
        let a = self.pop_word(interconnect);
        self.a = self.set_zn(a);
    }

    fn plp(&mut self, interconnect: &mut Interconnect) {
        self.p = self.pop_word(interconnect);
    }

    fn rol(&mut self, interconnect: &mut Interconnect, am: AddressingMode) {
        if let AddressingMode::Accumulator = am {
            let a = self.a;
            self.a = self.rotate_left(a);
        } else {
            let addr = self.addr_for(interconnect, &am);
            let value = interconnect.read_word(addr);
            let result = self.rotate_left(value);
            interconnect.write_word(addr, result);
        }
    }

    fn ror(&mut self, interconnect: &mut Interconnect, am: AddressingMode) {
        if let AddressingMode::Accumulator = am {
            let a = self.a;
            self.a = self.rotate_right(a);
        } else {
            let addr = self.addr_for(interconnect, &am);
            let value = interconnect.read_word(addr);
            let result = self.rotate_right(value);
            interconnect.write_word(addr, result);
        }
    }

    fn rti(&mut self, interconnect: &mut Interconnect) {
        self.p = self.pop_word(interconnect);
        self.pc = self.pop_double(interconnect);
    }

    fn rts(&mut self, interconnect: &mut Interconnect) {
        self.pc = self.pop_double(interconnect) + 1;
    }

    fn sbc(&mut self, value: u8) {
        let a = self.a;
        let carry_flag = if self.carry_flag() { 0 } else { 1 };
        let (result, carry) = a.overflowing_sub(value);
        let (result, carry2) = result.overflowing_sub(carry_flag);


        self.a = self.set_zn(result);
        self.set_overflow_flag(((a ^ value) & 0x80 == 0x80) && ((a ^ result) & 0x80 == 0x80));
        self.set_carry_flag(carry || carry2);
    }

    fn sec(&mut self) {
        self.set_carry_flag(true);
    }

    fn sed(&mut self) {
        self.set_decimal_mode(true);
    }

    fn sei(&mut self) {
        self.set_interrupt_disable(true);
    }

    fn sta(&self, interconnect: &mut Interconnect, addr: u16) -> bool {
        self.write_word(interconnect, addr, self.a)
    }

    fn sty(&self, interconnect: &mut Interconnect, addr: u16) -> bool {
        self.write_word(interconnect, addr, self.y)
    }

    fn stx(&self, interconnect: &mut Interconnect, addr: u16) -> bool {
        self.write_word(interconnect, addr, self.x)
    }

    fn tax(&mut self) {
        let a = self.a;
        self.x = self.set_zn(a);
    }

    fn tay(&mut self) {
        let a = self.a;
        self.y = self.set_zn(a);
    }

    fn tsx(&mut self) {
        let sp = self.sp;
        self.x = self.set_zn(sp);
    }

    fn txa(&mut self) {
        let x = self.x;
        self.a = self.set_zn(x);
    }

    fn tya(&mut self) {
        let y = self.y;
        self.a = self.set_zn(y);
    }

    fn txs(&mut self) {
        self.sp = self.x;
    }

    fn arithmetic_shift_left(&mut self, value: u8) -> u8 {
        self.set_carry_flag(value & 0x80 != 0);
        self.set_zn(value << 1)
    }

    fn logical_shift_right(&mut self, value: u8) -> u8 {
        self.set_carry_flag(value & 0x01 != 0);
        self.set_zn(value >> 1)
    }

    fn rotate_left(&mut self, value: u8) -> u8 {
        let carry_flag = self.carry_flag();
        self.set_carry_flag(value & 0x80 != 0);
        self.set_zn((value << 1) + carry_flag as u8)
    }

    fn rotate_right(&mut self, value: u8) -> u8 {
        let carry_flag = self.carry_flag();
        self.set_carry_flag(value & 0x01 != 0);
        self.set_zn((value >> 1) + ((carry_flag as u8) << 7))
    }

    fn push_word(&mut self, interconnect: &mut Interconnect, value: u8) {
        interconnect.write_word(STACK_END + self.sp as u16, value);
        self.sp -= 1;
    }

    fn push_double(&mut self, interconnect: &mut Interconnect, value: u16) {
        interconnect.write_double(STACK_END + self.sp as u16 - 1, value);
        self.sp -= 2;
    }

    fn pop_word(&mut self, interconnect: &mut Interconnect) -> u8 {
        self.sp += 1;
        interconnect.read_word(STACK_END + self.sp as u16)
    }

    fn pop_double(&mut self, interconnect: &mut Interconnect) -> u16 {
        self.sp += 2;
        interconnect.read_double(STACK_END + self.sp as u16 - 1)
    }
}

#[cfg(test)]
mod tests {
    use cpu::*;
    use interconnect::Interconnect;

    const RESET_ADDR: u16 = 0xc000;
    const BREAK_ADDR: u16 = 0xd000;

    struct TestInterconnect {
        mem: [u8; 65536],
    }

    impl TestInterconnect {
        fn new() -> TestInterconnect {
            TestInterconnect { mem: [0; 65536] }
        }
    }

    impl Interconnect for TestInterconnect {
        fn read_double(&mut self, addr: u16) -> u16 {
            ((self.read_word(addr + 1) as u16) << 8) + self.read_word(addr) as u16
        }

        fn read_word(&mut self, addr: u16) -> u8 {
            self.mem[addr as usize]
        }

        fn write_word(&mut self, addr: u16, value: u8) {
            self.mem[addr as usize] = value;
        }

        fn write_double(&mut self, addr: u16, value: u16) {
            self.mem[addr as usize] = value as u8;
            self.mem[addr as usize + 1] = (value >> 8) as u8;
        }
    }

    macro_rules! test_prg {
        ($prg:expr, $verify:expr) => ({
            let mut interconnect = TestInterconnect::new();
            let mut cpu = Cpu::new();

            interconnect.write_double(RESET_VECTOR, RESET_ADDR);
            interconnect.write_double(BREAK_VECTOR, BREAK_ADDR);

            for (i, v) in $prg.iter().flat_map(|cmd| cmd).enumerate() {
                interconnect.write_word(RESET_ADDR + i as u16, *v);
            }
            cpu.reset(&mut interconnect);

            for _ in $prg.iter() {
                cpu.step(&mut interconnect);
            }

            $verify(&mut interconnect, cpu);
        })
    }

    #[test]
    fn test_adc() {
        test_prg!(vec![vec![0xa9, 0x01] /* LDA #$01 */, vec![0x69, 0x01] /* ADC #$01 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 2);
                      assert_eq!(cpu.p, 0);
                  });


        test_prg!(vec![vec![0xa9, 0x01] /* LDA #$01 */, vec![0x69, 0xff] /* ADC #$ff */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0);
                      assert_eq!(cpu.p, CARRY_FLAG + ZERO_FLAG);
                  });

        test_prg!(vec![vec![0xa9, 0x7f] /* LDA #$7f */, vec![0x69, 0x01] /* ADC #$01 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0x80);
                      assert_eq!(cpu.p, OVERFLOW_FLAG + NEGATIVE_FLAG);
                  });

        test_prg!(vec![vec![0xa9, 0x80] /* LDA #$80 */, vec![0x69, 0xff] /* ADC #$ff */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0x7f);
                      assert_eq!(cpu.p, CARRY_FLAG + OVERFLOW_FLAG);
                  });

        test_prg!(vec![vec![0x38], // SEC
                       vec![0xa9, 0x00], // LDA #$00
                       vec![0x69, 0x00] /* ADC #$00 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 1);
                      assert_eq!(cpu.p, 0);
                  });
    }

    #[test]
    fn test_and() {
        test_prg!(vec![vec![0xa9, 0x01] /* LDA #$01 */, vec![0x29, 0x01] /* AND #$01 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 1);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0xa9, 0x01] /* LDA #$01 */, vec![0x29, 0x00] /* AND #$00 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0);
                      assert_eq!(cpu.p, ZERO_FLAG);
                  });

        test_prg!(vec![vec![0xa9, 0x80] /* LDA #$80 */, vec![0x29, 0x80] /* AND #$80 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0x80);
                      assert_eq!(cpu.p, NEGATIVE_FLAG);
                  });
    }

    #[test]
    fn test_asl() {
        test_prg!(vec![vec![0x0e, 0x03, 0xc0, 0x04]],
                  |interconnect: &mut TestInterconnect, cpu: Cpu| {
                      assert_eq!(interconnect.read_word(0xc003), 8);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0x0e, 0x03, 0xc0, 0x40]],
                  |interconnect: &mut TestInterconnect, cpu: Cpu| {
                      assert_eq!(interconnect.read_word(0xc003), 0x80);
                      assert_eq!(cpu.p, NEGATIVE_FLAG);
                  });

        test_prg!(vec![vec![0x0e, 0x03, 0xc0, 0x80]],
                  |interconnect: &mut TestInterconnect, cpu: Cpu| {
                      assert_eq!(interconnect.read_word(0xc003), 0);
                      assert_eq!(cpu.p, CARRY_FLAG + ZERO_FLAG);
                  });
    }

    #[test]
    fn test_bcc() {
        test_prg!(vec![vec![0x18] /* CLC */, vec![0x90, 0x04] /* BCC *+4 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.pc, RESET_ADDR + 7);
                  });

        test_prg!(vec![vec![0x38] /* SEC */, vec![0x90, 0x04] /* BCC *+4 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.pc, RESET_ADDR + 3);
                  });
    }

    #[test]
    fn test_bcs() {
        test_prg!(vec![vec![0x38] /* SEC */, vec![0xb0, 0x04] /* BCS *+4 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.pc, RESET_ADDR + 7);
                  });

        test_prg!(vec![vec![0x18] /* CLC */, vec![0xb0, 0x04] /* BCS *+4 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.pc, RESET_ADDR + 3);
                  });
    }

    #[test]
    fn test_beq() {
        test_prg!(vec![vec![0xa2, 0x01], // LDX #$01
                       vec![0xe0, 0x01], // CPX #$01
                       vec![0xf0, 0x04] /* BEQ *+4 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.pc, RESET_ADDR + 10);
                  });

        test_prg!(vec![vec![0xa2, 0x00], // LDX #$00
                       vec![0xe0, 0x01], // CPX #$01
                       vec![0xf0, 0x04] /* BEQ *+4 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.pc, RESET_ADDR + 6);
                  });
    }

    #[test]
    fn test_bit() {
        test_prg!(vec![vec![0xa9, 0x0f], // LDA #$0f
                       vec![0x2c, 0x05, 0xc0, 0x0f] /* BIT $c005 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0xa9, 0x0f], // LDA #$0f
                       vec![0x2c, 0x05, 0xc0, 0xf0] /* BIT $c005 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.p, ZERO_FLAG + OVERFLOW_FLAG + NEGATIVE_FLAG);
                  });
    }

    #[test]
    fn test_bmi() {
        test_prg!(vec![vec![0xa9, 0x80] /* LDA #$80 */, vec![0x30, 0x04] /* BMI *+4 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.pc, RESET_ADDR + 8);
                  });

        test_prg!(vec![vec![0xa9, 0x01] /* LDA #$01 */, vec![0x30, 0x04] /* BMI *+4 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.pc, RESET_ADDR + 4);
                  });
    }

    #[test]
    fn test_bne() {
        test_prg!(vec![vec![0xa2, 0x00], // LDX #$00
                       vec![0xe0, 0x01], // CPX #$01
                       vec![0xd0, 0x04] /* BNE *+4 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.pc, RESET_ADDR + 10);
                  });

        test_prg!(vec![vec![0xa2, 0x01], // LDX #$01
                       vec![0xe0, 0x01], // CPX #$01
                       vec![0xd0, 0x04] /* BNE *+4 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.pc, RESET_ADDR + 6);
                  });
    }

    #[test]
    fn test_bpl() {
        test_prg!(vec![vec![0xa9, 0x01] /* LDA #$01 */, vec![0x10, 0x04] /* BPL *+4 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.pc, RESET_ADDR + 8);
                  });

        test_prg!(vec![vec![0xa9, 0x80] /* LDA #$80 */, vec![0x10, 0x04] /* BPL *+4 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.pc, RESET_ADDR + 4);
                  });

        test_prg!(vec![vec![0xa9, 0x01] /* LDA #$01 */, vec![0x10, 0xfc] /* BPL *-4 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.pc, RESET_ADDR);
                  });
    }

    #[test]
    fn test_brk() {
        test_prg!(vec![vec![0xa2, 0xff], // LDX #$ff
                       vec![0x9a], // TXS
                       vec![0x00] /* BRK */],
                  |interconnect: &mut TestInterconnect, cpu: Cpu| {
                      assert_eq!(cpu.pc, BREAK_ADDR);
                      assert_eq!(cpu.p, NEGATIVE_FLAG + BREAK_COMMAND);
                      assert_eq!(interconnect.read_double(STACK_END + 0xfe), RESET_ADDR + 4);
                      assert_eq!(interconnect.read_word(STACK_END + 0xfd), NEGATIVE_FLAG);
                  });
    }

    #[test]
    fn test_bvc() {
        test_prg!(vec![vec![0x50, 0x04] /* BVC *+4 */], |_, cpu: Cpu| {
            assert_eq!(cpu.pc, RESET_ADDR + 6);
        });

        test_prg!(vec![vec![0xa9, 0x80], // LDA #$80
                       vec![0x69, 0xff], // ADC #$ff
                       vec![0x50, 0x04] /* BVC *+4 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.pc, RESET_ADDR + 6);
                  });
    }

    #[test]
    fn test_bvs() {
        test_prg!(vec![vec![0x70, 0x04] /* BVS *+4 */], |_, cpu: Cpu| {
            assert_eq!(cpu.pc, RESET_ADDR + 2);
        });

        test_prg!(vec![vec![0xa9, 0x80], // LDA #$80
                       vec![0x69, 0xff], // ADC #$ff
                       vec![0x70, 0x04] /* BVS *+4 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.pc, RESET_ADDR + 10);
                  });
    }

    #[test]
    fn test_clc() {
        test_prg!(vec![vec![0x38] /* SEC */, vec![0x18] /* CLC */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.p, 0);
                  });
    }

    #[test]
    fn test_cld() {
        test_prg!(vec![vec![0xf8] /* SED */, vec![0xd8] /* CLD */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.p, 0);
                  });
    }

    #[test]
    fn test_cli() {
        test_prg!(vec![vec![0x78] /* SEI */, vec![0x58] /* CLI */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.p, 0);
                  });
    }

    #[test]
    fn test_clv() {
        test_prg!(vec![vec![0xa9, 0x80], // LDA #$80
                       vec![0x69, 0xff], // ADC #$ff
                       vec![0xb8] /* CLV */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.p, CARRY_FLAG);
                  });
    }

    #[test]
    fn test_cmp() {
        test_prg!(vec![vec![0xa9, 0x01] /* LDA #$01 */, vec![0xc9, 0x00] /* CMP #$00 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.p, CARRY_FLAG);
                  });

        test_prg!(vec![vec![0xa9, 0x01] /* LDA #$01 */, vec![0xc9, 0x01] /* CMP #$01 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.p, CARRY_FLAG + ZERO_FLAG);
                  });

        test_prg!(vec![vec![0xa9, 0x01] /* LDA #$01 */, vec![0xc9, 0x02] /* CMP #$02 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.p, NEGATIVE_FLAG);
                  });
    }

    #[test]
    fn test_cpx() {
        test_prg!(vec![vec![0xa2, 0x01] /* LDX #$01 */, vec![0xe0, 0x00] /* CPX #$00 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.p, CARRY_FLAG);
                  });

        test_prg!(vec![vec![0xa2, 0x01] /* LDX #$01 */, vec![0xe0, 0x01] /* CPX #$01 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.p, ZERO_FLAG + CARRY_FLAG);
                  });


        test_prg!(vec![vec![0xa2, 0x00] /* LDX #$00 */, vec![0xe0, 0x01] /* CPX #$01 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.p, NEGATIVE_FLAG);
                  });
    }

    #[test]
    fn test_cpy() {
        test_prg!(vec![vec![0xa0, 0x01] /* LDY #$01 */, vec![0xc0, 0x00] /* CPY #$00 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.p, CARRY_FLAG);
                  });

        test_prg!(vec![vec![0xa0, 0x01] /* LDY #$01 */, vec![0xc0, 0x01] /* CPY #$01 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.p, ZERO_FLAG + CARRY_FLAG);
                  });


        test_prg!(vec![vec![0xa0, 0x00] /* LDY #$00 */, vec![0xc0, 0x01] /* CPY #$01 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.p, NEGATIVE_FLAG);
                  });
    }

    #[test]
    fn test_dec() {
        test_prg!(vec![vec![0xa9, 0x02], // LDA #$02
                       vec![0x8d, 0x00, 0x20], // STA $2000
                       vec![0xce, 0x00, 0x20]], // DEC $2000
                  |interconnect: &mut TestInterconnect, cpu: Cpu| {
                      assert_eq!(interconnect.read_word(0x2000), 1);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0xa9, 0x00], // LDA #$00
                       vec![0x8d, 0x00, 0x20], // STA $2000
                       vec![0xce, 0x00, 0x20]], // DEC $2000
                  |interconnect: &mut TestInterconnect, cpu: Cpu| {
                      assert_eq!(interconnect.read_word(0x2000), 0xff);
                      assert_eq!(cpu.p, NEGATIVE_FLAG);
                  });

        test_prg!(vec![vec![0xa9, 0x80], // LDA #$80
                       vec![0x8d, 0x00, 0x20], // STA $2000
                       vec![0xce, 0x00, 0x20]], // DEC $2000
                  |interconnect: &mut TestInterconnect, cpu: Cpu| {
                      assert_eq!(interconnect.read_word(0x2000), 0x7f);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0xa9, 0x01], // LDA #$01
                       vec![0x8d, 0x00, 0x20], // STA $2000
                       vec![0xce, 0x00, 0x20]], // DEC $2000
                  |interconnect: &mut TestInterconnect, cpu: Cpu| {
                      assert_eq!(interconnect.read_word(0x2000), 0);
                      assert_eq!(cpu.p, ZERO_FLAG);
                  });
    }

    #[test]
    fn test_dex() {
        test_prg!(vec![vec![0xa2, 0x02] /* LDX #$02 */, vec![0xca]], // DEX
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.x, 1);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0xa2, 0x00] /* LDX #$00 */, vec![0xca]], // DEX
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.x, 0xff);
                      assert_eq!(cpu.p, NEGATIVE_FLAG);
                  });

        test_prg!(vec![vec![0xa2, 0x80] /* LDX #$80 */, vec![0xca]], // DEX
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.x, 0x7f);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0xa2, 0x01] /* LDX #$01 */, vec![0xca]], // DEX
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.x, 0);
                      assert_eq!(cpu.p, ZERO_FLAG);
                  });
    }

    #[test]
    fn test_dey() {
        test_prg!(vec![vec![0xa0, 0x02] /* LDY #$02 */, vec![0x88]], // DEY
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.y, 1);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0xa0, 0x00] /* LDY #$00 */, vec![0x88]], // DEY
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.y, 0xff);
                      assert_eq!(cpu.p, NEGATIVE_FLAG);
                  });

        test_prg!(vec![vec![0xa0, 0x80] /* LDY #$80 */, vec![0x88]], // DEY
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.y, 0x7f);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0xa0, 0x01] /* LDY #$01 */, vec![0x88]], // DEY
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.y, 0);
                      assert_eq!(cpu.p, ZERO_FLAG);
                  });
    }

    #[test]
    fn test_eor() {
        test_prg!(vec![vec![0xa9, 0xcc] /* LDA #$cc */, vec![0x49, 0xaa] /* EOR #$aa */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0x66);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0xa9, 0x80] /* LDA #$80 */, vec![0x49, 0x00] /* EOR #$00 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0x80);
                      assert_eq!(cpu.p, NEGATIVE_FLAG);
                  });

        test_prg!(vec![vec![0xa9, 0xff] /* LDA #$ff */, vec![0x49, 0xff] /* EOR #$ff */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0);
                      assert_eq!(cpu.p, ZERO_FLAG);
                  });
    }

    #[test]
    fn test_inc() {
        test_prg!(vec![vec![0xa9, 0x01], // LDA #$01
                       vec![0x8d, 0x00, 0x20], // STA $2000
                       vec![0xee, 0x00, 0x20]], // INC $2000
                  |interconnect: &mut TestInterconnect, cpu: Cpu| {
                      assert_eq!(interconnect.read_word(0x2000), 2);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0xa9, 0x7f], // LDA #$7f
                       vec![0x8d, 0x00, 0x20], // STA $2000
                       vec![0xee, 0x00, 0x20]], // INC $2000
                  |interconnect: &mut TestInterconnect, cpu: Cpu| {
                      assert_eq!(interconnect.read_word(0x2000), 0x80);
                      assert_eq!(cpu.p, NEGATIVE_FLAG);
                  });

        test_prg!(vec![vec![0xa9, 0xff], // LDA #$ff
                       vec![0x8d, 0x00, 0x20], // STA $2000
                       vec![0xee, 0x00, 0x20]], // INC $2000
                  |interconnect: &mut TestInterconnect, cpu: Cpu| {
                      assert_eq!(interconnect.read_word(0x2000), 0);
                      assert_eq!(cpu.p, ZERO_FLAG);
                  });
    }

    #[test]
    fn test_inx() {
        test_prg!(vec![vec![0xa2, 0x01] /* LDX #$01 */, vec![0xe8]], // INX
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.x, 2);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0xa2, 0x7f] /* LDX #$7f */, vec![0xe8]], // INX
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.x, 0x80);
                      assert_eq!(cpu.p, NEGATIVE_FLAG);
                  });

        test_prg!(vec![vec![0xa2, 0xff] /* LDX #$ff */, vec![0xe8]], // INX
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.x, 0);
                      assert_eq!(cpu.p, ZERO_FLAG);
                  });
    }

    #[test]
    fn test_iny() {
        test_prg!(vec![vec![0xa0, 0x01] /* LDY #$01 */, vec![0xc8]], // INY
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.y, 2);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0xa0, 0x7f] /* LDY #$7f */, vec![0xc8]], // INY
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.y, 0x80);
                      assert_eq!(cpu.p, NEGATIVE_FLAG);
                  });

        test_prg!(vec![vec![0xa0, 0xff] /* LDY #$ff */, vec![0xc8]], // INY
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.y, 0);
                      assert_eq!(cpu.p, ZERO_FLAG);
                  });
    }

    #[test]
    fn test_jmp() {
        test_prg!(vec![vec![0x4c, 0x00, 0x20] /* JMP $2000 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.pc, 0x2000);
                  });
    }

    #[test]
    fn test_jsr() {
        test_prg!(vec![vec![0xa2, 0xff], // LDX #$ff
                       vec![0x9a], // TXS
                       vec![0x20, 0x00, 0x20] /* JSR $2000 */],
                  |interconnect: &mut TestInterconnect, cpu: Cpu| {
                      assert_eq!(cpu.pc, 0x2000);
                      assert_eq!(cpu.sp, 0xfd);
                      assert_eq!(interconnect.read_double(STACK_END + 0xfe), RESET_ADDR + 5);
                  });
    }

    #[test]
    fn test_lda() {
        test_prg!(vec![vec![0xa9, 0x01] /* LDA #$01 */], |_, cpu: Cpu| {
            assert_eq!(cpu.a, 1);
            assert_eq!(cpu.p, 0);
        });

        test_prg!(vec![vec![0xa9, 0x00] /* LDA #$00 */], |_, cpu: Cpu| {
            assert_eq!(cpu.a, 0x00);
            assert_eq!(cpu.p, ZERO_FLAG);
        });

        test_prg!(vec![vec![0xa9, 0x80] /* LDA #$80 */], |_, cpu: Cpu| {
            assert_eq!(cpu.a, 0x80);
            assert_eq!(cpu.p, NEGATIVE_FLAG);
        });
    }

    #[test]
    fn test_ldx() {
        test_prg!(vec![vec![0xa2, 0x01] /* LDX #$01 */], |_, cpu: Cpu| {
            assert_eq!(cpu.x, 1);
            assert_eq!(cpu.p, 0);
        });

        test_prg!(vec![vec![0xa2, 0x00] /* LDX #$00 */], |_, cpu: Cpu| {
            assert_eq!(cpu.x, 0x00);
            assert_eq!(cpu.p, ZERO_FLAG);
        });

        test_prg!(vec![vec![0xa2, 0x80] /* LDX #$80 */], |_, cpu: Cpu| {
            assert_eq!(cpu.x, 0x80);
            assert_eq!(cpu.p, NEGATIVE_FLAG);
        });
    }

    #[test]
    fn test_ldy() {
        test_prg!(vec![vec![0xa0, 0x01] /* LDY #$01 */], |_, cpu: Cpu| {
            assert_eq!(cpu.y, 1);
            assert_eq!(cpu.p, 0);
        });

        test_prg!(vec![vec![0xa0, 0x00] /* LDY #$00 */], |_, cpu: Cpu| {
            assert_eq!(cpu.y, 0x00);
            assert_eq!(cpu.p, ZERO_FLAG);
        });

        test_prg!(vec![vec![0xa0, 0x80] /* LDY #$80 */], |_, cpu: Cpu| {
            assert_eq!(cpu.y, 0x80);
            assert_eq!(cpu.p, NEGATIVE_FLAG);
        });
    }

    #[test]
    fn test_lsr() {
        test_prg!(vec![vec![0xa9, 0x02] /* LDA #$02 */, vec![0x4a] /* LSR A */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 1);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0xa9, 0x03] /* LDA #$03 */, vec![0x4a] /* LSR A */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 1);
                      assert_eq!(cpu.p, CARRY_FLAG);
                  });

        test_prg!(vec![vec![0xa9, 0x01] /* LDA #$01 */, vec![0x4a] /* LSR A */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0);
                      assert_eq!(cpu.p, ZERO_FLAG + CARRY_FLAG);
                  });
    }

    #[test]
    fn test_nop() {
        test_prg!(vec![vec![0xea] /* NOP */], |_, cpu: Cpu| {
            assert_eq!(cpu.pc, RESET_ADDR + 1);
        });
    }

    #[test]
    fn test_ora() {
        test_prg!(vec![vec![0xa9, 0x02] /* LDA #$02 */, vec![0x09, 0x01] /* ORA #$01 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 3);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0xa9, 0xf0] /* LDA #$f0 */, vec![0x09, 0x0f] /* ORA #$0f */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0xff);
                      assert_eq!(cpu.p, NEGATIVE_FLAG);
                  });

        test_prg!(vec![vec![0xa9, 0x00] /* LDA #$00 */, vec![0x09, 0x00] /* ORA #$00 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0);
                      assert_eq!(cpu.p, ZERO_FLAG);
                  });
    }

    #[test]
    fn test_pha() {
        test_prg!(vec![vec![0xa9, 0xff] /* LDA #$ff */, vec![0x48] /* PHA */],
                  |interconnect: &mut TestInterconnect, cpu: Cpu| {
                      assert_eq!(interconnect.read_word(STACK_END + cpu.sp as u16 + 1), 0xff);
                  });
    }

    #[test]
    fn test_php() {
        test_prg!(vec![vec![0xa9, 0xff] /* LDA #$ff */, vec![0x08] /* PHP */],
                  |interconnect: &mut TestInterconnect, cpu: Cpu| {
                      assert_eq!(interconnect.read_word(STACK_END + cpu.sp as u16 + 1),
                                 NEGATIVE_FLAG);
                  });
    }

    #[test]
    fn test_pla() {
        test_prg!(vec![vec![0xa9, 0xff], // LDA #$ff
                       vec![0x48], // PHA
                       vec![0xa9, 0x00], // LDA #$00
                       vec![0x68] /* PLA */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0xff);
                      assert_eq!(cpu.p, NEGATIVE_FLAG);
                  });
    }

    #[test]
    fn test_plp() {
        test_prg!(vec![vec![0xa9, 0xff], // LDA #$ff
                       vec![0x08], // PHP
                       vec![0xa9, 0x00], // LDA #$00
                       vec![0x28] /* PLP */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.p, NEGATIVE_FLAG);
                  });
    }

    #[test]
    fn test_rol() {
        test_prg!(vec![vec![0xa9, 0x01] /* LDA #$01 */, vec![0x2a] /* ROL A */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 2);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0x38], // SEC
                       vec![0xa9, 0x01], // LDA #$01
                       vec![0x2a] /* ROL A */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 3);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0xa9, 0x80] /* LDA #$80 */, vec![0x2a] /* ROL A */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0);
                      assert_eq!(cpu.p, ZERO_FLAG + CARRY_FLAG);
                  });
    }

    #[test]
    fn test_ror() {
        test_prg!(vec![vec![0xa9, 0x04] /* LDA #$04 */, vec![0x6a] /* ROR A */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 2);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0xa9, 0x01] /* LDA #$01 */, vec![0x6a] /* ROR A */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0);
                      assert_eq!(cpu.p, CARRY_FLAG + ZERO_FLAG);
                  });

        test_prg!(vec![vec![0x38], // SEC
                       vec![0xa9, 0x01], // LDA #$01
                       vec![0x6a] /* ROR A */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0x80);
                      assert_eq!(cpu.p, CARRY_FLAG + NEGATIVE_FLAG);
                  });
    }

    #[test]
    fn test_rti() {
        let break_addr = RESET_ADDR + 13;
        let l_break_vector = BREAK_VECTOR as u8;
        let h_break_vector = (BREAK_VECTOR >> 8) as u8;

        test_prg!(vec![vec![0xa9, break_addr as u8], // LDA #$xx,
                       vec![0x8d, l_break_vector, h_break_vector], // STA $xxxx,
                       vec![0xa9, (break_addr >> 8) as u8], // LDA #$xx,
                       vec![0x8d, l_break_vector + 1, h_break_vector], // STA $xx,
                       vec![0xa9, 0x00], // LDA #$00
                       vec![0x00], // BRK
                       vec![0xa9, 0x01], // LDA #$01
                       vec![0x40] /* RTI */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.p, ZERO_FLAG);
                      assert_eq!(cpu.pc, 0xc00d);
                  });
    }

    #[test]
    fn test_rts() {
        test_prg!(vec![vec![0xa2, 0xff], // LDX #$ff
                       vec![0x9a], // TXS
                       vec![0x20, 0x0a, 0xc0, 0x00, 0x00, 0x00, 0x00], // JSR $c00a
                       vec![0x60]],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.pc, 0xc006);
                      assert_eq!(cpu.sp, 0xff);
                  });
    }

    #[test]
    fn test_sbc() {
        test_prg!(vec![vec![0xa9, 0x03] /* LDA #$03 */, vec![0xe9, 0x01] /* SBC #$01 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 1);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0xa9, 0x02] /* LDA #$02 */, vec![0xe9, 0x01] /* SBC #$01 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0);
                      assert_eq!(cpu.p, ZERO_FLAG);
                  });

        test_prg!(vec![vec![0x38], // SEC
                       vec![0xa9, 0x02], // LDA #$02
                       vec![0xe9, 0x01] /* SBC #$01 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 1);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0x38], // SEC
                       vec![0xa9, 0x00], // LDA #$00
                       vec![0xe9, 0x01] /* SBC #$01 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0xff);
                      assert_eq!(cpu.p, CARRY_FLAG + NEGATIVE_FLAG);
                  });

        test_prg!(vec![vec![0x38], // SEC
                       vec![0xa9, 0x80], // LDA #$80
                       vec![0xe9, 0x01] /* SBC #$01 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0x7f);
                      assert_eq!(cpu.p, OVERFLOW_FLAG);
                  });

        test_prg!(vec![vec![0x38], // SEC
                       vec![0xa9, 0x7f], // LDA #$7f
                       vec![0xe9, 0xff] /* SBC #$ff */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0x80);
                      assert_eq!(cpu.p, NEGATIVE_FLAG + OVERFLOW_FLAG + CARRY_FLAG);
                  });
    }

    #[test]
    fn test_sec() {
        test_prg!(vec![vec![0x38] /* SEC */], |_, cpu: Cpu| {
            assert_eq!(cpu.p, CARRY_FLAG);
        });
    }

    #[test]
    fn test_sed() {
        test_prg!(vec![vec![0xf8] /* SED */], |_, cpu: Cpu| {
            assert_eq!(cpu.p, DECIMAL_MODE);
        });
    }

    #[test]
    fn test_sei() {
        test_prg!(vec![vec![0x78] /* SEI */], |_, cpu: Cpu| {
            assert_eq!(cpu.p, INTERUPT_DISABLE);
        });
    }

    #[test]
    fn test_sta() {
        test_prg!(vec![vec![0xa9, 0x01], // LDA #$01
                       vec![0x8d, 0x00, 0x20] /* STA $2000 */],
                  |interconnect: &mut TestInterconnect, _| {
                      assert_eq!(interconnect.read_word(0x2000), 1);
                  });
    }

    #[test]
    fn test_stx() {
        test_prg!(vec![vec![0xa2, 0x01], // LDX #$01
                       vec![0x8e, 0x00, 0x20] /* STX $2000 */],
                  |interconnect: &mut TestInterconnect, _| {
                      assert_eq!(interconnect.read_word(0x2000), 1);
                  });
    }

    #[test]
    fn test_sty() {
        test_prg!(vec![vec![0xa0, 0x01], // LDY #$01
                       vec![0x8c, 0x00, 0x20] /* STY $2000 */],
                  |interconnect: &mut TestInterconnect, _| {
                      assert_eq!(interconnect.read_word(0x2000), 1);
                  });
    }

    #[test]
    fn test_tax() {
        test_prg!(vec![vec![0xa9, 0x01] /* LDA #$01 */, vec![0xaa] /* TAX */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.x, 1);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0xa9, 0x80] /* LDA #$80 */, vec![0xaa] /* TAX */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.x, 0x80);
                      assert_eq!(cpu.p, NEGATIVE_FLAG);
                  });

        test_prg!(vec![vec![0xa9, 0x00] /* LDA #$00 */, vec![0xaa] /* TAX */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.x, 0x00);
                      assert_eq!(cpu.p, ZERO_FLAG);
                  });
    }

    #[test]
    fn test_tay() {
        test_prg!(vec![vec![0xa9, 0x01] /* LDA #$01 */, vec![0xa8] /* TAY */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.y, 1);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0xa9, 0x80] /* LDA #$80 */, vec![0xa8] /* TAY */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.y, 0x80);
                      assert_eq!(cpu.p, NEGATIVE_FLAG);
                  });

        test_prg!(vec![vec![0xa9, 0x00] /* LDA #$00 */, vec![0xa8] /* TAY */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.y, 0x00);
                      assert_eq!(cpu.p, ZERO_FLAG);
                  });
    }

    #[test]
    fn test_tsx() {
        test_prg!(vec![vec![0xba]], |_, cpu: Cpu| {
            assert_eq!(cpu.x, 0xfd);
            assert_eq!(cpu.p, NEGATIVE_FLAG);
        });
    }

    #[test]
    fn test_txa() {
        test_prg!(vec![vec![0xa2, 0x01] /* LDX #$01 */, vec![0x8a] /* TXA */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 1);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0xa2, 0x00] /* LDX #$00 */, vec![0x8a] /* TXA */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0);
                      assert_eq!(cpu.p, ZERO_FLAG);
                  });

        test_prg!(vec![vec![0xa2, 0x80] /* LDX #$80 */, vec![0x8a] /* TXA */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0x80);
                      assert_eq!(cpu.p, NEGATIVE_FLAG);
                  });
    }

    #[test]
    fn test_txs() {
        test_prg!(vec![vec![0xa2, 0x01] /* LDX #$01 */, vec![0x9a] /* TXS */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.sp, 1);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0xa2, 0x00] /* LDX #$00 */, vec![0x9a] /* TXS */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.sp, 0);
                      assert_eq!(cpu.p, ZERO_FLAG);
                  });

        test_prg!(vec![vec![0xa2, 0x80] /* LDX #$80 */, vec![0x9a] /* TXS */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.sp, 0x80);
                      assert_eq!(cpu.p, NEGATIVE_FLAG);
                  });
    }

    #[test]
    fn test_tya() {
        test_prg!(vec![vec![0xa0, 0x01] /* LDY #$01 */, vec![0x98] /* TYA */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 1);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0xa0, 0x00] /* LDY #$00 */, vec![0x98] /* TYA */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0);
                      assert_eq!(cpu.p, ZERO_FLAG);
                  });

        test_prg!(vec![vec![0xa0, 0x80] /* LDY #$80 */, vec![0x98] /* TYA */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0x80);
                      assert_eq!(cpu.p, NEGATIVE_FLAG);
                  });
    }
}
