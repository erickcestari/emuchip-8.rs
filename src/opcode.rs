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
    ShiftRight { x: usize, y: usize },
    SubN { x: usize, y: usize },
    ShiftLeft { x: usize, y: usize },
    SkipNeqReq { x: usize, y: usize },
    LoadI { addr: u16 },
    SkipEqImm { x: usize, conditon: u8 },
    SkipDifImm { x: usize, conditon: u8 },
    SkipEq { x: usize, y: usize },
}

impl Opcode {
    pub fn decode(raw: u16) -> Self {
        let nnn = raw & 0x0FFF;
        let nn = (raw & 0x00FF) as u8;
        let n = (raw & 0x000F) as u8;
        let x = ((raw & 0x0F00) >> 8) as usize;
        let y = ((raw & 0x00F0) >> 4) as usize;

        match raw & 0xF000 {
            0x0000 => match nn {
                0xE0 => Opcode::ClearScreen,
                0xEE => Opcode::Return,
                _ => unreachable!(),
            },
            0x1000 => Opcode::Jump { addr: nnn },
            0x2000 => Opcode::Call { addr: nnn },
            0x3000 => Opcode::SkipEqImm { x, conditon: nn },
            0x4000 => Opcode::SkipDifImm { x, conditon: nn },
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
                0x6 => Opcode::ShiftRight { x, y },
                0x7 => Opcode::SubN { x, y },
                0xE => Opcode::ShiftLeft { x, y },
                _ => unreachable!(),
            },
            0x9000 => Opcode::SkipNeqReq { x, y },
            0xA000 => Opcode::LoadI { addr: nnn },
            _ => unimplemented!("Unknown opcode {:04X}", raw),
        }
    }
}

