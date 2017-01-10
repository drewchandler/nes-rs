use interconnect::Interconnect;

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
            0x31 => Instruction(Op::And, AddressingMode::IndirectY),
            0x4c => Instruction(Op::Jmp, AddressingMode::Absolute),
            0x60 => Instruction(Op::Rts, AddressingMode::Implicit),
            0x78 => Instruction(Op::Sei, AddressingMode::Implicit),
            0x85 => Instruction(Op::Sta, AddressingMode::ZeroPage),
            0x8d => Instruction(Op::Sta, AddressingMode::Absolute),
            0x94 => Instruction(Op::Sty, AddressingMode::ZeroPageX),
            0x9a => Instruction(Op::Txs, AddressingMode::Implicit),
            0x9d => Instruction(Op::Sta, AddressingMode::AbsoluteX),
            0x10 => Instruction(Op::Bpl, AddressingMode::Immediate),
            0xa2 => Instruction(Op::Ldx, AddressingMode::Immediate),
            0xa5 => Instruction(Op::Lda, AddressingMode::ZeroPage),
            0xa6 => Instruction(Op::Ldx, AddressingMode::ZeroPage),
            0xa9 => Instruction(Op::Lda, AddressingMode::Immediate),
            0xad => Instruction(Op::Lda, AddressingMode::Absolute),
            0xbd => Instruction(Op::Lda, AddressingMode::AbsoluteX),
            0xc4 => Instruction(Op::Cpy, AddressingMode::ZeroPage),
            0xc9 => Instruction(Op::Cmp, AddressingMode::Immediate),
            0xca => Instruction(Op::Dex, AddressingMode::Implicit),
            0xd0 => Instruction(Op::Bne, AddressingMode::Relative),
            0xe0 => Instruction(Op::Cpx, AddressingMode::Immediate),
            0xe6 => Instruction(Op::Inc, AddressingMode::ZeroPage),
            0xf0 => Instruction(Op::Beq, AddressingMode::Relative),
            opcode => panic!("Unimplemented instruction: {:x}", opcode),
        }
    }

    fn execute(&mut self, interconnect: &mut Interconnect, instruction: Instruction) {
        println!("Executing {:?}", instruction);

        macro_rules! with_value {
            ($am:expr, $f:expr) => ({
                let value = self.value_for(interconnect, $am);
                $f(value)
            })
        }

        macro_rules! with_addr {
            ($am:expr, $f:expr) => ({
                let addr = self.addr_for(interconnect, $am);
                $f(addr)
            })
        }

        match instruction {
            Instruction(Op::Sei, _) => self.sei(),
            Instruction(Op::Lda, am) => with_value!(am, |value| self.lda(value)),
            Instruction(Op::Sta, am) => with_addr!(am, |addr| self.sta(interconnect, addr)),
            Instruction(Op::Jmp, am) => with_addr!(am, |addr| self.jmp(addr)),
            Instruction(Op::Ldx, am) => with_value!(am, |value| self.ldx(value)),
            Instruction(Op::Txs, _) => self.txs(),
            Instruction(Op::Bpl, am) => with_value!(am, |value| self.bpl(value)),
            Instruction(Op::And, am) => with_value!(am, |value| self.and(value)),
            Instruction(Op::Sty, am) => with_addr!(am, |addr| self.sty(interconnect, addr)),
            Instruction(Op::Jsr, am) => with_addr!(am, |addr| self.jsr(interconnect, addr)),
            Instruction(Op::Cpx, am) => with_value!(am, |value| self.cpx(value)),
            Instruction(Op::Beq, am) => with_addr!(am, |addr| self.beq(addr)),
            Instruction(Op::Inc, am) => with_addr!(am, |addr| self.inc(interconnect, addr)),
            Instruction(Op::Rts, _) => self.rts(interconnect),
            Instruction(Op::Cpy, am) => with_value!(am, |value| self.cpy(value)),
            Instruction(Op::Ora, am) => with_value!(am, |value| self.ora(value)),
            Instruction(Op::Cmp, am) => with_value!(am, |value| self.cmp(value)),
            Instruction(Op::Bne, am) => with_addr!(am, |addr| self.bne(addr)),
            Instruction(Op::Dex, _) => self.dex(),
            _ => panic!("Unimplemented operation: {:?}", instruction.0),
        }
    }

    fn value_for(&mut self, interconnect: &Interconnect, am: AddressingMode) -> u8 {
        match am {
            AddressingMode::Immediate => self.read_pc(interconnect),
            AddressingMode::Absolute |
            AddressingMode::AbsoluteX |
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
            AddressingMode::Relative => self.read_pc(interconnect) as u16 + self.pc,
            _ => panic!("Unimplemented addressing mode: {:?}", am),
        }
    }


    fn read_pc(&mut self, interconnect: &Interconnect) -> u8 {
        let value = interconnect.read_word(self.pc);
        self.pc += 1;
        value
    }

    fn set_interrupt_disable(&mut self, value: bool) {
        self.p = if value {
            self.p | 0b100
        } else {
            self.p & 0b11111011
        };
    }

    fn zero_flag(&self) -> bool {
        self.p & 0b10 != 0
    }

    fn set_zero_flag(&mut self, value: bool) {
        self.p = if value {
            self.p | 0b10
        } else {
            self.p & 0b11111101
        };
    }

    fn negative_flag(&self) -> bool {
        self.p & 0b10000000 != 0
    }

    fn set_negative_flag(&mut self, value: bool) {
        self.p = if value {
            self.p | 0b10000000
        } else {
            self.p & 0b01111111
        };
    }

    fn set_carry_flag(&mut self, value: bool) {
        self.p = if value {
            self.p | 1
        } else {
            self.p & 0b11111110
        };
    }

    fn compare(&mut self, a: u8, b: u8) {
        self.set_zero_flag(a == b);
        self.set_negative_flag(a < b);
        self.set_carry_flag(a >= b);
    }

    fn and(&mut self, value: u8) {
        let new_a = self.a & value;
        self.a = new_a;
        self.set_zero_flag(new_a == 0);
        self.set_negative_flag(new_a & 0b10000000 != 0);
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

    fn bpl(&mut self, offset: u8) {
        if !self.negative_flag() {
            self.pc += offset as u16;
        }
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
        self.x -= 1;
        let x = self.x;
        self.set_zero_flag(x == 0);
        self.set_negative_flag(x & 0b10000000 != 0);
    }

    fn inc(&mut self, interconnect: &mut Interconnect, addr: u16) {
        let value = interconnect.read_word(addr) + 1;
        self.set_zero_flag(value == 0);
        self.set_negative_flag(value & 0b10000000 != 0);
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
        self.a = value;
        self.set_zero_flag(value == 0);
        self.set_negative_flag(value & 0b10000000 != 0);
    }

    fn ldx(&mut self, value: u8) {
        self.x = value;
        self.set_zero_flag(value == 0);
        self.set_negative_flag(value & 0b10000000 != 0);
    }

    fn ora(&mut self, value: u8) {
        let new_a = self.a | value;
        self.a = new_a;
        self.set_zero_flag(new_a == 0);
        self.set_negative_flag(new_a & 0b10000000 != 0);
    }

    fn rts(&mut self, interconnect: &mut Interconnect) {
        self.pc = self.pop_double(interconnect) + 1;
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

    fn txs(&mut self) {
        self.sp = self.x;
    }

    fn push_double(&mut self, interconnect: &mut Interconnect, value: u16) {
        interconnect.write_double(0x100 + self.sp as u16 - 1, value);
        self.sp -= 2;
    }

    fn pop_double(&mut self, interconnect: &mut Interconnect) -> u16 {
        self.sp += 2;
        interconnect.read_double(0x100 + self.sp as u16 - 1)
    }
}
