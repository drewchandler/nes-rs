// Fixed 8 KiB pattern memory used by NROM and UxROM.
pub struct ChrMemory {
    data: Vec<u8>,
    writable: bool,
}

impl ChrMemory {
    pub fn new(mut banks: Vec<Vec<u8>>) -> ChrMemory {
        assert!(banks.len() <= 1, "Expected at most 8 KiB CHR ROM");
        let writable = banks.is_empty();
        let data = if writable { vec![0; 8192] } else { banks.remove(0) };
        assert_eq!(data.len(), 8192);
        ChrMemory { data, writable }
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.data[(addr & 0x1fff) as usize]
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        if self.writable {
            self.data[(addr & 0x1fff) as usize] = value;
        }
    }
}
