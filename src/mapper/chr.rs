// Cartridge pattern memory. Bank controllers translate PPU addresses first.
pub struct ChrMemory {
    data: Vec<u8>,
    writable: bool,
}

impl ChrMemory {
    pub fn new(banks: Vec<Vec<u8>>) -> ChrMemory {
        ChrMemory::with_ram(banks, 8192)
    }

    pub fn with_ram(banks: Vec<Vec<u8>>, ram_size: usize) -> ChrMemory {
        let writable = banks.is_empty();
        let data = if writable { vec![0; ram_size] } else {
            banks.into_iter().flat_map(|bank| bank.into_iter()).collect()
        };
        assert!(!data.is_empty(), "Cartridge needs CHR ROM or RAM");
        ChrMemory { data, writable }
    }

    pub fn bytes(&self) -> &[u8] { &self.data }
    pub fn bytes_mut(&mut self) -> &mut [u8] { &mut self.data }
    pub fn is_ram(&self) -> bool { self.writable }

    pub fn read(&self, addr: usize) -> u8 { self.data[addr % self.data.len()] }
    pub fn write(&mut self, addr: usize, value: u8) {
        if self.writable {
            let index = addr % self.data.len();
            self.data[index] = value;
        }
    }
}

impl ::state::Snapshot for ChrMemory {
    fn save(&self, w: &mut ::state::Writer) { if self.writable { ::state::Snapshot::save(&self.data,w); } }
    fn load(&mut self, r: &mut ::state::Reader) -> ::state::Result<()> {
        if self.writable { ::state::Snapshot::load(&mut self.data,r)?; } Ok(())
    }
}
