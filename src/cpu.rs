use interconnect::Interconnect;

const CARRY_FLAG: u8 = 0x01;
const ZERO_FLAG: u8 = 0x02;
const INTERUPT_DISABLE: u8 = 0x04;
const BREAK_COMMAND: u8 = 0x10;
const OVERFLOW_FLAG: u8 = 0x40;
const NEGATIVE_FLAG: u8 = 0x80;

pub struct Cpu {
    a: u8,
    p: u8,
    pc: u16,
    sp: u8,
    x: u8,
    y: u8,
}

#[derive(Debug)]
enum AddressingMode {
    Implicit,
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Relative,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Indirect,
    IndirectX,
    IndirectY,
}

#[derive(Debug)]
enum Op {
    Adc,
    And,
    Asl,
    Bcc,
    Bcs,
    Beq,
    Bit,
    Bmi,
    Bne,
    Bpl,
    Brk,
    Bvc,
    Bvs,
    Clc,
    Cld,
    Cli,
    Clv,
    Cmp,
    Cpx,
    Cpy,
    Dec,
    Dex,
    Dey,
    Eor,
    Inc,
    Inx,
    Iny,
    Jmp,
    Jsr,
    Lda,
    Ldx,
    Ldy,
    Lsr,
    Nop,
    Ora,
    Pha,
    Php,
    Pla,
    Plp,
    Rol,
    Ror,
    Rti,
    Rts,
    Sbc,
    Sec,
    Sed,
    Sei,
    Sta,
    Stx,
    Sty,
    Tax,
    Tay,
    Tsx,
    Txa,
    Txs,
    Tya,
}

#[derive(Debug)]
struct Instruction(Op, AddressingMode);

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            a: 0,
            p: 0,
            pc: 0,
            sp: 0,
            x: 0,
            y: 0,
        }
    }

    pub fn reset(&mut self, interconnect: &Interconnect) {
        self.pc = interconnect.read_double(0xfffc);
    }

    pub fn step(&mut self, interconnect: &mut Interconnect) {
        let instruction = self.read_instruction(interconnect);
        self.execute(interconnect, instruction)
    }

    fn read_instruction(&mut self, interconnect: &Interconnect) -> Instruction {
        match self.read_pc(interconnect) {
            0x00 => Instruction(Op::Brk, AddressingMode::Implicit),
            0x01 => Instruction(Op::Ora, AddressingMode::IndirectX),
            0x05 => Instruction(Op::Ora, AddressingMode::ZeroPage),
            0x06 => Instruction(Op::Asl, AddressingMode::ZeroPage),
            0x08 => Instruction(Op::Php, AddressingMode::Implicit),
            0x09 => Instruction(Op::Ora, AddressingMode::Immediate),
            0x20 => Instruction(Op::Jsr, AddressingMode::Absolute),
            0x28 => Instruction(Op::Plp, AddressingMode::Implicit),
            0x29 => Instruction(Op::And, AddressingMode::Immediate),
            0x31 => Instruction(Op::And, AddressingMode::IndirectY),
            0x48 => Instruction(Op::Pha, AddressingMode::Implicit),
            0x4c => Instruction(Op::Jmp, AddressingMode::Absolute),
            0x5d => Instruction(Op::Eor, AddressingMode::AbsoluteX),
            0x60 => Instruction(Op::Rts, AddressingMode::Implicit),
            0x68 => Instruction(Op::Pla, AddressingMode::Implicit),
            0x78 => Instruction(Op::Sei, AddressingMode::Implicit),
            0x84 => Instruction(Op::Sty, AddressingMode::ZeroPage),
            0x85 => Instruction(Op::Sta, AddressingMode::ZeroPage),
            0x86 => Instruction(Op::Stx, AddressingMode::ZeroPage),
            0x8a => Instruction(Op::Txa, AddressingMode::Implicit),
            0x8d => Instruction(Op::Sta, AddressingMode::Absolute),
            0x90 => Instruction(Op::Bcc, AddressingMode::Relative),
            0x94 => Instruction(Op::Sty, AddressingMode::ZeroPageX),
            0x95 => Instruction(Op::Sta, AddressingMode::ZeroPageX),
            0x9a => Instruction(Op::Txs, AddressingMode::Implicit),
            0x9d => Instruction(Op::Sta, AddressingMode::AbsoluteX),
            0x10 => Instruction(Op::Bpl, AddressingMode::Immediate),
            0xa0 => Instruction(Op::Ldy, AddressingMode::Immediate),
            0xa2 => Instruction(Op::Ldx, AddressingMode::Immediate),
            0xa5 => Instruction(Op::Lda, AddressingMode::ZeroPage),
            0xa6 => Instruction(Op::Ldx, AddressingMode::ZeroPage),
            0xa9 => Instruction(Op::Lda, AddressingMode::Immediate),
            0xaa => Instruction(Op::Tax, AddressingMode::Implicit),
            0xad => Instruction(Op::Lda, AddressingMode::Absolute),
            0xb0 => Instruction(Op::Bcs, AddressingMode::Relative),
            0xb9 => Instruction(Op::Lda, AddressingMode::AbsoluteY),
            0xbd => Instruction(Op::Lda, AddressingMode::AbsoluteX),
            0xc4 => Instruction(Op::Cpy, AddressingMode::ZeroPage),
            0xc5 => Instruction(Op::Cmp, AddressingMode::ZeroPage),
            0xc9 => Instruction(Op::Cmp, AddressingMode::Immediate),
            0xca => Instruction(Op::Dex, AddressingMode::Implicit),
            0xd0 => Instruction(Op::Bne, AddressingMode::Relative),
            0xdd => Instruction(Op::Cmp, AddressingMode::AbsoluteX),
            0xe0 => Instruction(Op::Cpx, AddressingMode::Immediate),
            0xe4 => Instruction(Op::Cpx, AddressingMode::ZeroPage),
            0xe6 => Instruction(Op::Inc, AddressingMode::ZeroPage),
            0xe9 => Instruction(Op::Sbc, AddressingMode::Immediate),
            0xf0 => Instruction(Op::Beq, AddressingMode::Relative),
            opcode => panic!("Unimplemented instruction: {:x}", opcode),
        }
    }

    fn execute(&mut self, interconnect: &mut Interconnect, instruction: Instruction) {
        println!("Executing {:?}", instruction);
        let Instruction(op, am) = instruction;

        macro_rules! with_value {
            ($f:expr) => ({
                let value = self.value_for(interconnect, am);
                $f(value)
            })
        }

        macro_rules! with_addr {
            ($f:expr) => ({
                let addr = self.addr_for(interconnect, am);
                $f(addr)
            })
        }

        match op {
            Op::Sei => self.sei(),
            Op::Lda => with_value!(|value| self.lda(value)),
            Op::Sta => with_addr!(|addr| self.sta(interconnect, addr)),
            Op::Jmp => with_addr!(|addr| self.jmp(addr)),
            Op::Ldx => with_value!(|value| self.ldx(value)),
            Op::Txs => self.txs(),
            Op::Bpl => with_addr!(|addr| self.bpl(addr)),
            Op::And => with_value!(|value| self.and(value)),
            Op::Sty => with_addr!(|addr| self.sty(interconnect, addr)),
            Op::Jsr => with_addr!(|addr| self.jsr(interconnect, addr)),
            Op::Cpx => with_value!(|value| self.cpx(value)),
            Op::Beq => with_addr!(|addr| self.beq(addr)),
            Op::Inc => with_addr!(|addr| self.inc(interconnect, addr)),
            Op::Rts => self.rts(interconnect),
            Op::Cpy => with_value!(|value| self.cpy(value)),
            Op::Ora => with_value!(|value| self.ora(value)),
            Op::Cmp => with_value!(|value| self.cmp(value)),
            Op::Bne => with_addr!(|addr| self.bne(addr)),
            Op::Dex => self.dex(),
            Op::Eor => with_value!(|value| self.eor(value)),
            Op::Php => self.php(interconnect),
            Op::Pha => self.pha(interconnect),
            Op::Txa => self.txa(),
            Op::Bcc => with_addr!(|addr| self.bcc(addr)),
            Op::Sbc => with_value!(|value| self.sbc(value)),
            Op::Tax => self.tax(),
            Op::Pla => self.pla(interconnect),
            Op::Plp => self.plp(interconnect),
            Op::Ldy => with_value!(|value| self.ldy(value)),
            Op::Bcs => with_addr!(|addr| self.bcs(addr)),
            Op::Stx => with_addr!(|addr| self.stx(interconnect, addr)),
            Op::Brk => self.brk(interconnect),
            _ => panic!("Unimplemented operation: {:?}", op),
        }
    }

    fn value_for(&mut self, interconnect: &Interconnect, am: AddressingMode) -> u8 {
        match am {
            AddressingMode::Immediate => self.read_pc(interconnect),
            AddressingMode::Absolute |
            AddressingMode::AbsoluteX |
            AddressingMode::AbsoluteY |
            AddressingMode::ZeroPage |
            AddressingMode::IndirectY => {
                let addr = self.addr_for(interconnect, am);
                interconnect.read_word(addr)
            }
            _ => panic!("Unimplemented addressing mode: {:?}", am),
        }
    }

    fn addr_for(&mut self, interconnect: &Interconnect, am: AddressingMode) -> u16 {
        match am {
            AddressingMode::Absolute => {
                let lower = self.read_pc(interconnect);
                let higher = self.read_pc(interconnect);
                ((higher as u16) << 8) + lower as u16
            }
            AddressingMode::IndirectY => {
                let zero_page_addr = self.read_pc(interconnect);
                let addr = interconnect.read_double(zero_page_addr as u16);
                addr + self.y as u16
            }
            AddressingMode::ZeroPage => self.read_pc(interconnect) as u16,
            AddressingMode::ZeroPageX => {
                let zero_page_addr = self.read_pc(interconnect);
                (zero_page_addr + self.x) as u16
            }
            AddressingMode::AbsoluteX => {
                self.addr_for(interconnect, AddressingMode::Absolute) + self.x as u16
            }
            AddressingMode::AbsoluteY => {
                self.addr_for(interconnect, AddressingMode::Absolute) + self.y as u16
            }
            AddressingMode::Relative => self.read_pc(interconnect) as u16 + self.pc,
            _ => panic!("Unimplemented addressing mode: {:?}", am),
        }
    }


    fn read_pc(&mut self, interconnect: &Interconnect) -> u8 {
        let value = interconnect.read_word(self.pc);
        self.pc += 1;
        value
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
        self.set_negative_flag(value & 0b10000000 != 0);
        value
    }

    fn compare(&mut self, a: u8, b: u8) {
        self.set_zero_flag(a == b);
        self.set_negative_flag(a < b);
        self.set_carry_flag(a >= b);
    }

    fn and(&mut self, value: u8) {
        let a = self.a;
        self.a = self.set_zn(a & value);
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
        self.pc = interconnect.read_double(0xfffe);
        self.set_break_command(true);
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

    fn dex(&mut self) {
        let x = self.x;
        self.x = self.set_zn(x - 1);
    }

    fn eor(&mut self, value: u8) {
        let a = self.a;
        self.a = self.set_zn(a ^ value);
    }

    fn inc(&mut self, interconnect: &mut Interconnect, addr: u16) {
        let value = self.set_zn(interconnect.read_word(addr) + 1);
        interconnect.write_word(addr, value);
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
        self.a = self.pop_word(interconnect);
    }

    fn plp(&mut self, interconnect: &mut Interconnect) {
        self.p = self.pop_word(interconnect);
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
        self.set_overflow_flag(((a ^ value) & 0x08 == 0x08) && ((a ^ result) & 0x08 == 0x08));
        self.set_carry_flag(!(carry || carry2));
    }

    fn sei(&mut self) {
        self.set_interrupt_disable(true);
    }

    fn sta(&self, interconnect: &mut Interconnect, addr: u16) {
        interconnect.write_word(addr, self.a);
    }

    fn sty(&self, interconnect: &mut Interconnect, addr: u16) {
        interconnect.write_word(addr, self.y);
    }

    fn stx(&self, interconnect: &mut Interconnect, addr: u16) {
        interconnect.write_word(addr, self.x);
    }

    fn tax(&mut self) {
        let a = self.a;
        self.x = self.set_zn(a);
    }

    fn txa(&mut self) {
        let x = self.x;
        self.a = self.set_zn(x);
    }

    fn txs(&mut self) {
        self.sp = self.x;
    }

    fn push_word(&mut self, interconnect: &mut Interconnect, value: u8) {
        interconnect.write_word(0x100 + self.sp as u16 - 1, value);
        self.sp -= 1;
    }

    fn push_double(&mut self, interconnect: &mut Interconnect, value: u16) {
        interconnect.write_double(0x100 + self.sp as u16 - 1, value);
        self.sp -= 2;
    }

    fn pop_word(&mut self, interconnect: &mut Interconnect) -> u8 {
        self.sp += 1;
        interconnect.read_word(0x100 + self.sp as u16 - 1)
    }

    fn pop_double(&mut self, interconnect: &mut Interconnect) -> u16 {
        self.sp += 2;
        interconnect.read_double(0x100 + self.sp as u16 - 1)
    }
}
