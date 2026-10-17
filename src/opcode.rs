#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    ClearScreen,
    Return,
    Jump { addr: u16 },
    Call { addr: u16 },
    LoadImm { x: usize, kk: u8 },
    AddImm { x: usize, kk: u8 },
    LoadReg { x: usize, y: usize },
    Or { x: usize, y: usize },
    And { x: usize, y: usize },
    Xor { x: usize, y: usize },
    AddReg { x: usize, y: usize },
    SubReg { x: usize, y: usize },
    ShiftRight { x: usize },
    SubN { x: usize, y: usize },
    ShiftLeft { x: usize },
    SkipNeqReg { x: usize, y: usize },
    LoadI { addr: u16 },
    SkipEqImm { x: usize, kk: u8 },
    SkipDifImm { x: usize, kk: u8 },
    SkipEq { x: usize, y: usize },
    JumpV0 { addr: u16 },
    Random { x: usize, nn: u8 },
    Draw { x: usize, y: usize, n: u8 },
    SkipKey { x: usize },
    SkipNotKey { x: usize },
    LoadDelayTimer { x: usize },
    GetKey { x: usize },
    SetDelayTimer { x: usize },
    SetSoundTimer { x: usize },
    AddI { x: usize },
    LoadSprite { x: usize },
    StoreBCD { x: usize },
    RegDump { x: usize },
    RegLoad { x: usize },
}

impl Opcode {
    /// Returns `None` for words outside the instruction set.
    pub fn decode(raw: u16) -> Option<Self> {
        let nnn = raw & 0x0FFF;
        let nn = (raw & 0x00FF) as u8;
        let n = (raw & 0x000F) as u8;
        let x = ((raw & 0x0F00) >> 8) as usize;
        let y = ((raw & 0x00F0) >> 4) as usize;

        let op = match raw & 0xF000 {
            0x0000 => match nn {
                0xE0 => Opcode::ClearScreen,
                0xEE => Opcode::Return,
                _ => return None,
            },
            0x1000 => Opcode::Jump { addr: nnn },
            0x2000 => Opcode::Call { addr: nnn },
            0x3000 => Opcode::SkipEqImm { x, kk: nn },
            0x4000 => Opcode::SkipDifImm { x, kk: nn },
            0x5000 => Opcode::SkipEq { x, y },
            0x6000 => Opcode::LoadImm { x, kk: nn },
            0x7000 => Opcode::AddImm { x, kk: nn },
            0x8000 => match n {
                0x0 => Opcode::LoadReg { x, y },
                0x1 => Opcode::Or { x, y },
                0x2 => Opcode::And { x, y },
                0x3 => Opcode::Xor { x, y },
                0x4 => Opcode::AddReg { x, y },
                0x5 => Opcode::SubReg { x, y },
                0x6 => Opcode::ShiftRight { x },
                0x7 => Opcode::SubN { x, y },
                0xE => Opcode::ShiftLeft { x },
                _ => return None,
            },
            0x9000 => Opcode::SkipNeqReg { x, y },
            0xA000 => Opcode::LoadI { addr: nnn },
            0xB000 => Opcode::JumpV0 { addr: nnn },
            0xC000 => Opcode::Random { x, nn },
            0xD000 => Opcode::Draw { x, y, n },
            0xE000 => match nn {
                0x9E => Opcode::SkipKey { x },
                0xA1 => Opcode::SkipNotKey { x },
                _ => return None,
            },
            0xF000 => match nn {
                0x07 => Opcode::LoadDelayTimer { x },
                0x0A => Opcode::GetKey { x },
                0x15 => Opcode::SetDelayTimer { x },
                0x18 => Opcode::SetSoundTimer { x },
                0x1E => Opcode::AddI { x },
                0x29 => Opcode::LoadSprite { x },
                0x33 => Opcode::StoreBCD { x },
                0x55 => Opcode::RegDump { x },
                0x65 => Opcode::RegLoad { x },
                _ => return None,
            },
            _ => return None,
        };
        Some(op)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_operands() {
        assert_eq!(
            Opcode::decode(0xD12F),
            Some(Opcode::Draw { x: 1, y: 2, n: 0xF })
        );
        assert_eq!(
            Opcode::decode(0x3A42),
            Some(Opcode::SkipEqImm { x: 0xA, kk: 0x42 })
        );
        assert_eq!(Opcode::decode(0xB123), Some(Opcode::JumpV0 { addr: 0x123 }));
    }

    #[test]
    fn rejects_words_outside_the_instruction_set() {
        for raw in [0x0000, 0x0123, 0x8008, 0xE000, 0xF0FF] {
            assert_eq!(Opcode::decode(raw), None, "{raw:#06X}");
        }
    }
}
