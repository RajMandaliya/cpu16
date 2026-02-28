/// FLAGS register — bit layout:
///   bit 0: Zero     (Z)
///   bit 1: Carry    (C)
///   bit 2: Negative (N)
///   bit 3: Overflow (V)
///   bit 7: Interrupt Enable (IE)
#[derive(Debug, Clone, Copy, Default)]
pub struct Flags(pub u8);

impl Flags {
    pub fn zero(&self)     -> bool { self.0 & (1 << 0) != 0 }
    pub fn carry(&self)    -> bool { self.0 & (1 << 1) != 0 }
    pub fn negative(&self) -> bool { self.0 & (1 << 2) != 0 }
    pub fn overflow(&self) -> bool { self.0 & (1 << 3) != 0 }
    pub fn int_enable(&self) -> bool { self.0 & (1 << 7) != 0 }

    pub fn set_zero(&mut self, v: bool)     { self.set_bit(0, v); }
    pub fn set_carry(&mut self, v: bool)    { self.set_bit(1, v); }
    pub fn set_negative(&mut self, v: bool) { self.set_bit(2, v); }
    pub fn set_overflow(&mut self, v: bool) { self.set_bit(3, v); }
    pub fn set_int_enable(&mut self, v: bool) { self.set_bit(7, v); }

    fn set_bit(&mut self, bit: u8, v: bool) {
        if v { self.0 |= 1 << bit; } else { self.0 &= !(1 << bit); }
    }

    /// Update Z, N, C flags from an arithmetic result.
    /// `result` is the 32-bit wide result to detect carry/overflow.
    pub fn update_arithmetic(&mut self, result: u32, a: u16, b: u16, is_sub: bool) {
        let r16 = result as u16;
        self.set_zero(r16 == 0);
        self.set_negative(r16 & 0x8000 != 0);
        self.set_carry(result > 0xFFFF);

        // Signed overflow: sign of inputs differs from sign of result
        let ov = if is_sub {
            (a ^ b) & 0x8000 != 0 && (a ^ r16) & 0x8000 != 0
        } else {
            (a ^ r16) & 0x8000 != 0 && (b ^ r16) & 0x8000 != 0
        };
        self.set_overflow(ov);
    }

    /// Update Z and N from a logical result (no carry).
    pub fn update_logical(&mut self, result: u16) {
        self.set_zero(result == 0);
        self.set_negative(result & 0x8000 != 0);
        self.set_carry(false);
    }
}

impl std::fmt::Display for Flags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[Z:{} C:{} N:{} V:{} IE:{}]",
            self.zero() as u8,
            self.carry() as u8,
            self.negative() as u8,
            self.overflow() as u8,
            self.int_enable() as u8,
        )
    }
}