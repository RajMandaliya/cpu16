/// 64 KB flat byte-addressable memory.
/// Words are stored little-endian.
pub struct Memory {
    data: Box<[u8; 65536]>,
}

impl Memory {
    pub fn new() -> Self {
        Self { data: Box::new([0u8; 65536]) }
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }

    pub fn write_byte(&mut self, addr: u16, val: u8) {
        self.data[addr as usize] = val;
    }

    pub fn read_word(&self, addr: u16) -> u16 {
        let lo = self.data[addr as usize] as u16;
        let hi = self.data[addr.wrapping_add(1) as usize] as u16;
        lo | (hi << 8)
    }

    pub fn write_word(&mut self, addr: u16, val: u16) {
        self.data[addr as usize]                   = (val & 0xFF) as u8;
        self.data[addr.wrapping_add(1) as usize]   = (val >> 8) as u8;
    }

    /// Load a binary blob starting at `start_addr`.
    pub fn load(&mut self, start_addr: u16, bytes: &[u8]) {
        let start = start_addr as usize;
        let end   = (start + bytes.len()).min(65536);
        self.data[start..end].copy_from_slice(&bytes[..end - start]);
    }

    /// Return a hex dump of a memory range (useful for debugging).
    pub fn hex_dump(&self, start: u16, len: u16) -> String {
        let mut out = String::new();
        for row in (0..len).step_by(16) {
            let addr = start.wrapping_add(row);
            out.push_str(&format!("  {:04X}: ", addr));
            for col in 0..16u16 {
                if row + col < len {
                    out.push_str(&format!("{:02X} ", self.data[addr.wrapping_add(col) as usize]));
                } else {
                    out.push_str("   ");
                }
            }
            out.push('\n');
        }
        out
    }
}

impl Default for Memory {
    fn default() -> Self { Self::new() }
}