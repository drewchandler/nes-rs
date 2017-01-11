#[derive(Debug)]
pub enum AddressingMode {
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
pub enum Op {
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
pub struct Instruction(pub Op, pub AddressingMode);

impl Instruction {
    pub fn from_opcode(opcode: u8) -> Instruction {
        match opcode {
            0x00 => Instruction(Op::Brk, AddressingMode::Implicit),
            0x01 => Instruction(Op::Ora, AddressingMode::IndirectX),
            0x05 => Instruction(Op::Ora, AddressingMode::ZeroPage),
            0x06 => Instruction(Op::Asl, AddressingMode::ZeroPage),
            0x08 => Instruction(Op::Php, AddressingMode::Implicit),
            0x09 => Instruction(Op::Ora, AddressingMode::Immediate),
            0x0a => Instruction(Op::Asl, AddressingMode::Accumulator),
            0x0d => Instruction(Op::Ora, AddressingMode::Absolute),
            0x0e => Instruction(Op::Asl, AddressingMode::Absolute),
            0x10 => Instruction(Op::Bpl, AddressingMode::Relative),
            0x11 => Instruction(Op::Ora, AddressingMode::IndirectY),
            0x15 => Instruction(Op::Ora, AddressingMode::ZeroPageX),
            0x16 => Instruction(Op::Asl, AddressingMode::ZeroPageX),
            0x18 => Instruction(Op::Clc, AddressingMode::Implicit),
            0x19 => Instruction(Op::Ora, AddressingMode::AbsoluteY),
            0x20 => Instruction(Op::Jsr, AddressingMode::Absolute),
            0x21 => Instruction(Op::And, AddressingMode::IndirectX),
            0x24 => Instruction(Op::Bit, AddressingMode::ZeroPage),
            0x25 => Instruction(Op::And, AddressingMode::ZeroPage),
            0x26 => Instruction(Op::Rol, AddressingMode::ZeroPage),
            0x28 => Instruction(Op::Plp, AddressingMode::Implicit),
            0x29 => Instruction(Op::And, AddressingMode::Immediate),
            0x2a => Instruction(Op::Rol, AddressingMode::Accumulator),
            0x2c => Instruction(Op::Bit, AddressingMode::Absolute),
            0x2d => Instruction(Op::And, AddressingMode::Absolute),
            0x2e => Instruction(Op::Rol, AddressingMode::Absolute),
            0x30 => Instruction(Op::Bmi, AddressingMode::Relative),
            0x31 => Instruction(Op::And, AddressingMode::IndirectY),
            0x35 => Instruction(Op::And, AddressingMode::ZeroPageX),
            0x36 => Instruction(Op::Rol, AddressingMode::ZeroPageX),
            0x38 => Instruction(Op::Sec, AddressingMode::Implicit),
            0x39 => Instruction(Op::And, AddressingMode::AbsoluteY),
            0x3d => Instruction(Op::And, AddressingMode::AbsoluteX),
            0x3e => Instruction(Op::Rol, AddressingMode::AbsoluteX),
            0x40 => Instruction(Op::Rti, AddressingMode::Implicit),
            0x41 => Instruction(Op::Eor, AddressingMode::IndirectX),
            0x45 => Instruction(Op::Eor, AddressingMode::ZeroPage),
            0x46 => Instruction(Op::Lsr, AddressingMode::ZeroPage),
            0x48 => Instruction(Op::Pha, AddressingMode::Implicit),
            0x49 => Instruction(Op::Eor, AddressingMode::Immediate),
            0x4a => Instruction(Op::Lsr, AddressingMode::Accumulator),
            0x4c => Instruction(Op::Jmp, AddressingMode::Absolute),
            0x4d => Instruction(Op::Eor, AddressingMode::Absolute),
            0x4e => Instruction(Op::Lsr, AddressingMode::Absolute),
            0x50 => Instruction(Op::Bvc, AddressingMode::Relative),
            0x51 => Instruction(Op::Eor, AddressingMode::IndirectY),
            0x55 => Instruction(Op::Eor, AddressingMode::ZeroPageX),
            0x56 => Instruction(Op::Lsr, AddressingMode::ZeroPageX),
            0x58 => Instruction(Op::Cli, AddressingMode::Implicit),
            0x59 => Instruction(Op::Eor, AddressingMode::AbsoluteY),
            0x5d => Instruction(Op::Eor, AddressingMode::AbsoluteX),
            0x5e => Instruction(Op::Lsr, AddressingMode::AbsoluteX),
            0x60 => Instruction(Op::Rts, AddressingMode::Implicit),
            0x61 => Instruction(Op::Adc, AddressingMode::IndirectX),
            0x65 => Instruction(Op::Adc, AddressingMode::ZeroPage),
            0x66 => Instruction(Op::Ror, AddressingMode::ZeroPage),
            0x68 => Instruction(Op::Pla, AddressingMode::Implicit),
            0x69 => Instruction(Op::Adc, AddressingMode::Immediate),
            0x6a => Instruction(Op::Ror, AddressingMode::Accumulator),
            0x6c => Instruction(Op::Jmp, AddressingMode::Indirect),
            0x6d => Instruction(Op::Adc, AddressingMode::Absolute),
            0x6e => Instruction(Op::Ror, AddressingMode::Absolute),
            0x70 => Instruction(Op::Bvs, AddressingMode::Relative),
            0x71 => Instruction(Op::Adc, AddressingMode::IndirectY),
            0x75 => Instruction(Op::Adc, AddressingMode::ZeroPageX),
            0x76 => Instruction(Op::Ror, AddressingMode::ZeroPageX),
            0x78 => Instruction(Op::Sei, AddressingMode::Implicit),
            0x79 => Instruction(Op::Adc, AddressingMode::AbsoluteY),
            0x7d => Instruction(Op::Adc, AddressingMode::AbsoluteX),
            0x7e => Instruction(Op::Ror, AddressingMode::AbsoluteX),
            0x81 => Instruction(Op::Sta, AddressingMode::IndirectX),
            0x84 => Instruction(Op::Sty, AddressingMode::ZeroPage),
            0x85 => Instruction(Op::Sta, AddressingMode::ZeroPage),
            0x86 => Instruction(Op::Stx, AddressingMode::ZeroPage),
            0x88 => Instruction(Op::Dey, AddressingMode::Implicit),
            0x8a => Instruction(Op::Txa, AddressingMode::Implicit),
            0x8c => Instruction(Op::Sty, AddressingMode::Absolute),
            0x8d => Instruction(Op::Sta, AddressingMode::Absolute),
            0x8e => Instruction(Op::Stx, AddressingMode::Absolute),
            0x90 => Instruction(Op::Bcc, AddressingMode::Relative),
            0x91 => Instruction(Op::Sta, AddressingMode::IndirectY),
            0x94 => Instruction(Op::Sty, AddressingMode::ZeroPageX),
            0x95 => Instruction(Op::Sta, AddressingMode::ZeroPageX),
            0x96 => Instruction(Op::Stx, AddressingMode::ZeroPageY),
            0x98 => Instruction(Op::Tya, AddressingMode::Implicit),
            0x99 => Instruction(Op::Sta, AddressingMode::AbsoluteY),
            0x9a => Instruction(Op::Txs, AddressingMode::Implicit),
            0x9d => Instruction(Op::Sta, AddressingMode::AbsoluteX),
            0xa0 => Instruction(Op::Ldy, AddressingMode::Immediate),
            0xa1 => Instruction(Op::Lda, AddressingMode::IndirectX),
            0xa2 => Instruction(Op::Ldx, AddressingMode::Immediate),
            0xa4 => Instruction(Op::Ldy, AddressingMode::ZeroPage),
            0xa5 => Instruction(Op::Lda, AddressingMode::ZeroPage),
            0xa6 => Instruction(Op::Ldx, AddressingMode::ZeroPage),
            0xa8 => Instruction(Op::Tay, AddressingMode::Implicit),
            0xa9 => Instruction(Op::Lda, AddressingMode::Immediate),
            0xaa => Instruction(Op::Tax, AddressingMode::Implicit),
            0xac => Instruction(Op::Ldy, AddressingMode::Absolute),
            0xad => Instruction(Op::Lda, AddressingMode::Absolute),
            0xae => Instruction(Op::Ldx, AddressingMode::Absolute),
            0xb0 => Instruction(Op::Bcs, AddressingMode::Relative),
            0xb1 => Instruction(Op::Lda, AddressingMode::IndirectY),
            0xb4 => Instruction(Op::Ldy, AddressingMode::ZeroPageX),
            0xb5 => Instruction(Op::Lda, AddressingMode::ZeroPageX),
            0xb6 => Instruction(Op::Ldx, AddressingMode::ZeroPageY),
            0xb8 => Instruction(Op::Clv, AddressingMode::Implicit),
            0xb9 => Instruction(Op::Lda, AddressingMode::AbsoluteY),
            0xba => Instruction(Op::Tsx, AddressingMode::Implicit),
            0xbc => Instruction(Op::Ldy, AddressingMode::AbsoluteX),
            0xbd => Instruction(Op::Lda, AddressingMode::AbsoluteX),
            0xbe => Instruction(Op::Ldx, AddressingMode::AbsoluteY),
            0xc0 => Instruction(Op::Cpy, AddressingMode::Immediate),
            0xc1 => Instruction(Op::Cmp, AddressingMode::IndirectX),
            0xc4 => Instruction(Op::Cpy, AddressingMode::ZeroPage),
            0xc5 => Instruction(Op::Cmp, AddressingMode::ZeroPage),
            0xc6 => Instruction(Op::Dec, AddressingMode::ZeroPage),
            0xc8 => Instruction(Op::Iny, AddressingMode::Implicit),
            0xc9 => Instruction(Op::Cmp, AddressingMode::Immediate),
            0xca => Instruction(Op::Dex, AddressingMode::Implicit),
            0xcc => Instruction(Op::Cpy, AddressingMode::Absolute),
            0xcd => Instruction(Op::Cmp, AddressingMode::Absolute),
            0xce => Instruction(Op::Dec, AddressingMode::Absolute),
            0xd0 => Instruction(Op::Bne, AddressingMode::Relative),
            0xd1 => Instruction(Op::Cmp, AddressingMode::IndirectY),
            0xd5 => Instruction(Op::Cmp, AddressingMode::ZeroPageX),
            0xd6 => Instruction(Op::Dec, AddressingMode::ZeroPageX),
            0xd8 => Instruction(Op::Cld, AddressingMode::Implicit),
            0xd9 => Instruction(Op::Cmp, AddressingMode::AbsoluteY),
            0xdd => Instruction(Op::Cmp, AddressingMode::AbsoluteX),
            0xde => Instruction(Op::Dec, AddressingMode::AbsoluteX),
            0xe0 => Instruction(Op::Cpx, AddressingMode::Immediate),
            0xe1 => Instruction(Op::Sbc, AddressingMode::IndirectX),
            0xe4 => Instruction(Op::Cpx, AddressingMode::ZeroPage),
            0xe5 => Instruction(Op::Sbc, AddressingMode::ZeroPage),
            0xe6 => Instruction(Op::Inc, AddressingMode::ZeroPage),
            0xe8 => Instruction(Op::Inx, AddressingMode::Implicit),
            0xe9 => Instruction(Op::Sbc, AddressingMode::Immediate),
            0xea => Instruction(Op::Nop, AddressingMode::Implicit),
            0xec => Instruction(Op::Cpx, AddressingMode::Absolute),
            0xed => Instruction(Op::Sbc, AddressingMode::Absolute),
            0xee => Instruction(Op::Inc, AddressingMode::Absolute),
            0xf0 => Instruction(Op::Beq, AddressingMode::Relative),
            0xf1 => Instruction(Op::Sbc, AddressingMode::IndirectY),
            0xf5 => Instruction(Op::Sbc, AddressingMode::ZeroPageX),
            0xf6 => Instruction(Op::Inc, AddressingMode::ZeroPageX),
            0xf8 => Instruction(Op::Sed, AddressingMode::Implicit),
            0xf9 => Instruction(Op::Sbc, AddressingMode::AbsoluteY),
            0xfd => Instruction(Op::Sbc, AddressingMode::AbsoluteX),
            0xfe => Instruction(Op::Inc, AddressingMode::AbsoluteX),
            opcode => panic!("Unimplemented instruction: {:x}", opcode),
        }
    }
}
