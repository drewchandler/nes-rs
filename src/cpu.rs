use interconnect::Interconnect;

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
            sp: 0,
            x: 0,
            y: 0,
        }
    }

    pub fn reset(&mut self, interconnect: &Interconnect) {
        self.pc = interconnect.read_double(0xfffc);
    }

    pub fn step(&mut self, interconnect: &mut Interconnect) {
        let opcode = self.read_pc(interconnect);
        println!("Executing {:x}", opcode);

        match opcode {
            0x20 => {
                let addr = self.absolute(interconnect);
                self.jsr(interconnect, addr);
            }
            0x31 => {
                let addr = self.indirect_y(interconnect);
                let value = interconnect.read_word(addr);
                self.and(value);
            }
            0x4c => {
                let addr = self.absolute(interconnect);
                self.jmp(addr);
            }
            0x78 => self.sei(),
            0x8d => {
                let addr = self.absolute(interconnect);
                self.sta(interconnect, addr);
            }
            0x94 => {
                let addr = self.zero_page_x(interconnect);
                self.sty(interconnect, addr);
            }
            0x9a => {
                self.txs();
            }
            0x9d => {
                let addr = self.absolute_x(interconnect);
                self.sta(interconnect, addr);
            }
            0x10 => {
                let offset = self.immediate(interconnect);
                self.bpl(offset);
            }
            0xa2 => {
                let value = self.immediate(interconnect);
                self.ldx(value);
            }
            0xa6 => {
                let addr = self.zero_page(interconnect);
                let value = interconnect.read_word(addr);
                self.ldx(value);
            }
            0xa9 => {
                let value = self.immediate(interconnect);
                self.lda(value);
            }
            0xad => {
                let addr = self.absolute(interconnect);
                let value = interconnect.read_word(addr);
                self.lda(value);
            }
            0xe0 => {
                let value = self.immediate(interconnect);
                self.cpx(value);
            }
            0xf0 => {
                let offset = self.immediate(interconnect);
                self.beq(offset);
            }
            _ => panic!("Unimplemented instruction: {:x}", opcode),
        };
    }

    fn read_pc(&mut self, interconnect: &Interconnect) -> u8 {
        let value = interconnect.read_word(self.pc);
        self.pc += 1;
        value
    }

    fn immediate(&mut self, interconnect: &Interconnect) -> u8 {
        self.read_pc(interconnect)
    }

    fn absolute(&mut self, interconnect: &Interconnect) -> u16 {
        let lower = self.read_pc(interconnect);
        let higher = self.read_pc(interconnect);
        ((higher as u16) << 8) + lower as u16
    }

    fn absolute_x(&mut self, interconnect: &Interconnect) -> u16 {
        self.absolute(interconnect) + self.x as u16
    }

    fn indirect_y(&mut self, interconnect: &Interconnect) -> u16 {
        let zero_page_addr = self.read_pc(interconnect);
        let addr = interconnect.read_double(zero_page_addr as u16);
        addr + self.y as u16
    }

    fn zero_page(&mut self, interconnect: &Interconnect) -> u16 {
        self.read_pc(interconnect) as u16
    }

    fn zero_page_x(&mut self, interconnect: &Interconnect) -> u16 {
        let zero_page_addr = self.read_pc(interconnect);
        (zero_page_addr + self.x) as u16
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

    fn and(&mut self, value: u8) {
        let new_a = self.a & value;
        self.a = new_a;
        self.set_zero_flag(new_a == 0);
        self.set_negative_flag(new_a & 0b10000000 != 0);
    }

    fn beq(&mut self, offset: u8) {
        if self.zero_flag() {
            self.pc += offset as u16;
        }
    }

    fn bpl(&mut self, offset: u8) {
        if !self.negative_flag() {
            self.pc += offset as u16;
        }
    }

    fn cpx(&mut self, value: u8) {
        let x = self.x;
        self.set_zero_flag(x == value);
        self.set_negative_flag(x < value);
        self.set_carry_flag(x >= value);
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
}
