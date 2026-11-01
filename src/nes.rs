use cpu::Cpu;
use interconnect::MemoryMappingInterconnect;
use joypad::ButtonState;
use rom::Rom;

pub struct Nes {
    rom: Rom,
    pub rom_id: [u8; 32],
    pub interconnect: MemoryMappingInterconnect,
    pub cpu: Cpu,
}

impl Nes {
    pub fn new(rom: Rom) -> Nes {
        let mut identity = Vec::new();
        for bank in rom.prg_rom.iter().chain(rom.chr_rom.iter()) {
            identity.extend_from_slice(bank);
        }
        identity.extend_from_slice(&(rom.prg_rom.len() as u32).to_le_bytes());
        identity.extend_from_slice(&(rom.chr_rom.len() as u32).to_le_bytes());
        identity.push(rom.nes2 as u8);
        identity.extend_from_slice(&rom.mapper.to_le_bytes());
        identity.push(rom.submapper);
        identity.push(rom.region as u8);
        identity.push(rom.mirroring as u8);
        for size in [
            rom.prg_ram_size,
            rom.chr_ram_size,
            rom.prg_nvram_size,
            rom.chr_nvram_size,
        ] {
            identity.extend_from_slice(&(size as u32).to_le_bytes());
        }
        let rom_id = ::digest::sha256(&identity);
        Nes {
            rom_id,
            rom: rom.clone(),
            interconnect: MemoryMappingInterconnect::new(rom),
            cpu: Cpu::new(),
        }
    }

    pub fn reset(&mut self) {
        self.cpu.reset(&mut self.interconnect);
    }

    pub fn run_frame(&mut self, joypad1_state: ButtonState) -> &[u32; 256 * 240] {
        self.run_frame_two(joypad1_state, ButtonState::default())
    }
    pub fn run_frame_two(&mut self, p1: ButtonState, p2: ButtonState) -> &[u32; 256 * 240] {
        self.interconnect.joypad1.set_state(p1);
        self.interconnect.joypad2.set_state(p2);

        while !self.interconnect.take_frame_ready() {
            self.cpu.step(&mut self.interconnect);
        }

        &self.interconnect.ppu.screen
    }
}

impl Nes {
    pub fn fps(&self) -> f64 {
        self.rom.region.fps()
    }
    pub fn battery(&self) -> Vec<u8> {
        self.interconnect.battery()
    }
    pub fn load_battery(&mut self, bytes: &[u8]) -> ::state::Result<()> {
        self.interconnect.load_battery(bytes)
    }
    pub fn save_state(&self) -> Vec<u8> {
        use state::Snapshot;
        let mut w = ::state::Writer::new();
        w.bytes(b"NESRS\x00\x01\x00");
        w.bytes(&self.rom_id);
        self.cpu.save(&mut w);
        self.interconnect.save(&mut w);
        let crc = ::state::checksum(&w.0);
        crc.save(&mut w);
        w.0
    }
    pub fn load_state(&mut self, bytes: &[u8]) -> ::state::Result<()> {
        use state::Snapshot;
        if bytes.len() < 44 || bytes.len() > 4 * 1024 * 1024 {
            return Err("Invalid state file size".into());
        }
        if &bytes[..8] != b"NESRS\x00\x01\x00" {
            return Err("Unsupported state file version".into());
        }
        if bytes[8..40] != self.rom_id {
            return Err("This state belongs to a different ROM or region".into());
        }
        let end = bytes.len() - 4;
        if bytes[end..] != ::state::checksum(&bytes[..end]).to_le_bytes() {
            return Err("State file checksum failed".into());
        }
        // Parse into a fresh machine; an invalid import never damages the running game.
        let mut restored = Nes::new(self.rom.clone());
        let mut r = ::state::Reader::new(&bytes[40..end]);
        restored.cpu.load(&mut r)?;
        restored.interconnect.load(&mut r)?;
        if !r.finished() {
            return Err("Unexpected state file data".into());
        }
        *self = restored;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
