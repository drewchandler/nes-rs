use interconnect::Interconnect;
use instruction::{Op, AddressingMode, Instruction};

pub const CARRY_FLAG: u8 = 0x01;
pub const ZERO_FLAG: u8 = 0x02;
pub const INTERUPT_DISABLE: u8 = 0x04;
pub const DECIMAL_MODE: u8 = 0x08;
pub const BREAK_COMMAND: u8 = 0x10;
pub const OVERFLOW_FLAG: u8 = 0x40;
pub const NEGATIVE_FLAG: u8 = 0x80;

pub const RESET_VECTOR: u16 = 0xfffc;

pub const STACK_END: u16 = 0x100;

pub struct Cpu {
    pub a: u8,
    pub p: u8,
    pub pc: u16,
    pub sp: u8,
    pub x: u8,
    pub y: u8,
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

    pub fn reset(&mut self, interconnect: &Interconnect) {
        self.pc = interconnect.read_double(RESET_VECTOR);
    }

    pub fn nmi(&mut self, interconnect: &mut Interconnect) {
        let pc = self.pc;
        self.push_double(interconnect, pc);
        let p = self.p;
        self.push_word(interconnect, p);
        self.pc = interconnect.read_double(0xfffa);
    }

    pub fn step(&mut self, interconnect: &mut Interconnect) {
        let Instruction(op, am) = Instruction::from_opcode(self.read_pc(interconnect));
        println!("Executing {:?} {:?}", op, am);

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

        match op {
            Op::Adc => with_value!(|value| self.adc(value)),
            Op::And => with_value!(|value| self.and(value)),
            Op::Asl => self.asl(interconnect, am),
            Op::Bcc => with_addr!(|addr| self.bcc(addr)),
            Op::Bcs => with_addr!(|addr| self.bcs(addr)),
            Op::Beq => with_addr!(|addr| self.beq(addr)),
            Op::Bne => with_addr!(|addr| self.bne(addr)),
            Op::Bpl => with_addr!(|addr| self.bpl(addr)),
            Op::Brk => self.brk(interconnect),
            Op::Clc => self.clc(),
            Op::Cld => self.cld(),
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
            Op::Ora => with_value!(|value| self.ora(value)),
            Op::Pha => self.pha(interconnect),
            Op::Php => self.php(interconnect),
            Op::Pla => self.pla(interconnect),
            Op::Plp => self.plp(interconnect),
            Op::Rts => self.rts(interconnect),
            Op::Sbc => with_value!(|value| self.sbc(value)),
            Op::Sec => self.sec(),
            Op::Sei => self.sei(),
            Op::Sta => with_addr!(|addr| self.sta(interconnect, addr)),
            Op::Stx => with_addr!(|addr| self.stx(interconnect, addr)),
            Op::Sty => with_addr!(|addr| self.sty(interconnect, addr)),
            Op::Tax => self.tax(),
            Op::Txa => self.txa(),
            Op::Tya => self.tya(),
            Op::Txs => self.txs(),
            _ => panic!("Unimplemented operation: {:?}", op),
        }
    }

    fn value_for(&mut self, interconnect: &Interconnect, am: &AddressingMode) -> u8 {
        match *am {
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

    fn addr_for(&mut self, interconnect: &Interconnect, am: &AddressingMode) -> u16 {
        match *am {
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
                (zero_page_addr + self.x) as u16 & 0xff
            }
            AddressingMode::AbsoluteX => {
                self.addr_for(interconnect, &AddressingMode::Absolute) + self.x as u16
            }
            AddressingMode::AbsoluteY => {
                self.addr_for(interconnect, &AddressingMode::Absolute) + self.y as u16
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

    fn clc(&mut self) {
        self.set_carry_flag(false);
    }

    fn cld(&mut self) {
        self.set_decimal_mode(false);
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
        let value = self.set_zn(interconnect.read_word(addr) - 1);
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
        self.set_overflow_flag(((a ^ value) & 0x80 == 0x80) && ((a ^ result) & 0x80 == 0x80));
        self.set_carry_flag(!(carry || carry2));
    }

    fn sec(&mut self) {
        self.set_carry_flag(true);
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

    struct TestInterconnect {
        mem: [u8; 65535],
    }

    impl TestInterconnect {
        fn new() -> TestInterconnect {
            TestInterconnect { mem: [0; 65535] }
        }
    }

    impl Interconnect for TestInterconnect {
        fn read_double(&self, addr: u16) -> u16 {
            ((self.read_word(addr + 1) as u16) << 8) + self.read_word(addr) as u16
        }

        fn read_word(&self, addr: u16) -> u8 {
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

            for (i, v) in $prg.iter().flat_map(|cmd| cmd).enumerate() {
                interconnect.write_word(RESET_ADDR + i as u16, *v);
            }
            cpu.reset(&interconnect);

            for _ in $prg.iter() {
                cpu.step(&mut interconnect);
            }

            $verify(interconnect, cpu);
        })
    }

    #[test]
    fn test_adc() {
        test_prg!(vec![vec![0x18], // CLC
                       vec![0xa9, 0x01], // LDA #$01
                       vec![0x69, 0x01] /* ADX #$01 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 2);
                      assert_eq!(cpu.p, 0);
                  });


        test_prg!(vec![vec![0x18], // CLC
                       vec![0xa9, 0x01], // LDA #$01
                       vec![0x69, 0xff] /* ADX #$ff */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0);
                      assert_eq!(cpu.p, CARRY_FLAG + ZERO_FLAG);
                  });

        test_prg!(vec![vec![0x18], // CLC
                       vec![0xa9, 0x7f], // LDA #$7f
                       vec![0x69, 0x01] /* ADX #$01 */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0x80);
                      assert_eq!(cpu.p, OVERFLOW_FLAG + NEGATIVE_FLAG);
                  });

        test_prg!(vec![vec![0x18], // CLC
                       vec![0xa9, 0x80], // LDA #$80
                       vec![0x69, 0xff] /* ADX #$ff */],
                  |_, cpu: Cpu| {
                      assert_eq!(cpu.a, 0x7f);
                      assert_eq!(cpu.p, CARRY_FLAG + OVERFLOW_FLAG);
                  });

        test_prg!(vec![vec![0x38], // SEC
                       vec![0xa9, 0x00], // LDA #$00
                       vec![0x69, 0x00] /* ADX #$00 */],
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
                  |interconnect: TestInterconnect, cpu: Cpu| {
                      assert_eq!(interconnect.read_word(0xc003), 8);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0x0e, 0x03, 0xc0, 0x40]],
                  |interconnect: TestInterconnect, cpu: Cpu| {
                      assert_eq!(interconnect.read_word(0xc003), 0x80);
                      assert_eq!(cpu.p, NEGATIVE_FLAG);
                  });

        test_prg!(vec![vec![0x0e, 0x03, 0xc0, 0x80]],
                  |interconnect: TestInterconnect, cpu: Cpu| {
                      assert_eq!(interconnect.read_word(0xc003), 0);
                      assert_eq!(cpu.p, CARRY_FLAG + ZERO_FLAG);
                  });
    }

    #[test]
    #[ignore]
    fn test_bcc() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_bcs() {
        assert!(false, "Write me");
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
    #[ignore]
    fn test_bit() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_bmi() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_bne() {
        assert!(false, "Write me");
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
    }

    #[test]
    #[ignore]
    fn test_brk() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_bvc() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_bvs() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_clc() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_cld() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_cli() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_clv() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_cmp() {
        assert!(false, "Write me");
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
    #[ignore]
    fn test_cpy() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_dec() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_dex() {
        assert!(false, "Write me");
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
    #[ignore]
    fn test_eor() {
        assert!(false, "Write me");
    }

    #[test]
    fn test_inc() {
        test_prg!(vec![vec![0xa9, 0x01], // LDA #$01
                       vec![0x8d, 0x00, 0x20], // STA $2000
                       vec![0xee, 0x00, 0x20]], // INC $2000
                  |interconnect: TestInterconnect, cpu: Cpu| {
                      assert_eq!(interconnect.read_word(0x2000), 2);
                      assert_eq!(cpu.p, 0);
                  });

        test_prg!(vec![vec![0xa9, 0x7f], // LDA #$7f
                       vec![0x8d, 0x00, 0x20], // STA $2000
                       vec![0xee, 0x00, 0x20]], // INC $2000
                  |interconnect: TestInterconnect, cpu: Cpu| {
                      assert_eq!(interconnect.read_word(0x2000), 0x80);
                      assert_eq!(cpu.p, NEGATIVE_FLAG);
                  });

        test_prg!(vec![vec![0xa9, 0xff], // LDA #$ff
                       vec![0x8d, 0x00, 0x20], // STA $2000
                       vec![0xee, 0x00, 0x20]], // INC $2000
                  |interconnect: TestInterconnect, cpu: Cpu| {
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
    #[ignore]
    fn test_iny() {
        assert!(false, "Write me");
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
                  |interconnect: TestInterconnect, cpu: Cpu| {
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
    #[ignore]
    fn test_ldy() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_lsr() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_nop() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_ora() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_pha() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_php() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_pla() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_plp() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_rol() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_ror() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_rti() {
        assert!(false, "Write me");
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
    #[ignore]
    fn test_sbc() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_sec() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_sed() {
        assert!(false, "Write me");
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
                  |interconnect: TestInterconnect, _| {
                      assert_eq!(interconnect.read_word(0x2000), 1);
                  });
    }

    #[test]
    #[ignore]
    fn test_stx() {
        assert!(false, "Write me");
    }

    #[test]
    fn test_sty() {
        test_prg!(vec![vec![0xa0, 0x01], // LDY #$01
                       vec![0x8c, 0x00, 0x20] /* STY $2000 */],
                  |interconnect: TestInterconnect, _| {
                      assert_eq!(interconnect.read_word(0x2000), 1);
                  });
    }

    #[test]
    #[ignore]
    fn test_tax() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_tay() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_tsx() {
        assert!(false, "Write me");
    }

    #[test]
    #[ignore]
    fn test_txa() {
        assert!(false, "Write me");
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
    #[ignore]
    fn test_tya() {
        assert!(false, "Write me");
    }
}
