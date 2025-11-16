pub enum Opcode {
    ClearScreen,
    Return,
    Jump { addr: u16 },
    Call { addr: u16 },
    LoadImm { x: usize, kk: u8 },
    AddImm { x: usize, kk: u8 },
}

impl Opcode {
    pub fn decode(raw: u16) -> Self {
        let nnn = raw & 0x0FFF;
        let nn  = (raw & 0x00FF) as u8;
        let n   = (raw & 0x000F) as u8;
        let x   = ((raw & 0x0F00) >> 8) as usize;
        let y   = ((raw & 0x00F0) >> 4) as usize;

        match raw & 0xF000 {
            0x0000 => match nn {
                0xE0 => Opcode::ClearScreen,
                0xEE => Opcode::Return,
                _ => unimplemented!(),
            },
            0x1000 => Opcode::Jump { addr: nnn },
            0x2000 => Opcode::Call { addr: nnn },
            0x6000 => Opcode::LoadImm { x, kk: nn },
            0x7000 => Opcode::AddImm { x, kk: nn },
            _ => unimplemented!("Unknown opcode {:04X}", raw),
        }
    }
}