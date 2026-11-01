use std::fs;
use std::io::{self, Read};
use std::path;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    FourScreen,
    SingleLower,
    SingleUpper,
}

#[derive(Clone)]
pub struct Rom {
    pub prg_rom: Vec<Vec<u8>>,
    pub chr_rom: Vec<Vec<u8>>,
    pub mapper: u16,
    pub submapper: u8,
    pub mirroring: Mirroring,
    pub prg_ram_size: usize,
    pub chr_ram_size: usize,
    pub nes2: bool,
    pub prg_nvram_size: usize,
    pub chr_nvram_size: usize,
    pub region: ::region::Region,
}

impl Rom {
    pub fn load<P: AsRef<path::Path>>(filename: P) -> io::Result<Rom> {
        let mut file = fs::File::open(filename)?;

        Rom::read(&mut file)
    }

    pub fn read<R: Read>(file: &mut R) -> io::Result<Rom> {
        let mut header = [0u8; 16];
        file.read_exact(&mut header)?;

        if header[0..4] != *b"NES\x1a" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid ROM"));
        }

        let nes2 = header[7] & 0x0c == 0x08;
        if header[7] & 0x0c != 0 && !nes2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unsupported ROM header format",
            ));
        }
        // The core stores fixed-size banks. Extended/exponent sizes need a
        // separate decoder; reject them rather than silently ignoring byte 9.
        if nes2 && header[9] != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unsupported NES 2.0 ROM size",
            ));
        }

        if header[6] & 0x04 != 0 {
            let mut trainer = [0u8; 512];
            file.read_exact(&mut trainer)?;
        }

        let prg_rom = (0..header[4])
            .map(|_| {
                let mut buf = vec![0u8; 16384];
                file.read_exact(&mut buf)?;
                Ok(buf)
            })
            .collect::<io::Result<Vec<Vec<u8>>>>()?;

        let chr_rom = (0..header[5])
            .map(|_| {
                let mut buf = vec![0u8; 8192];
                file.read_exact(&mut buf)?;
                Ok(buf)
            })
            .collect::<io::Result<Vec<Vec<u8>>>>()?;

        let mapper = (header[7] & 0xf0) as u16
            | (header[6] >> 4) as u16
            | if nes2 {
                ((header[8] & 0x0f) as u16) << 8
            } else {
                0
            };
        let ram_size = |shift: u8| if shift == 0 { 0 } else { 64usize << shift };
        let prg_ram_size = if nes2 {
            ram_size(header[10] & 15) + ram_size(header[10] >> 4)
        } else {
            header[8].max(1) as usize * 8192
        };
        let chr_ram_size = if nes2 {
            ram_size(header[11] & 15) + ram_size(header[11] >> 4)
        } else if header[5] != 0 {
            0
        } else if mapper == 13 {
            16384
        } else {
            8192
        };

        Ok(Rom {
            prg_rom: prg_rom,
            chr_rom: chr_rom,
            mapper,
            prg_ram_size,
            chr_ram_size,
            nes2,
            prg_nvram_size: if nes2 {
                ram_size(header[10] >> 4)
            } else if header[6] & 2 != 0 {
                prg_ram_size
            } else {
                0
            },
            chr_nvram_size: if nes2 { ram_size(header[11] >> 4) } else { 0 },
            region: if nes2 {
                match header[12] & 3 {
                    1 => ::region::Region::Pal,
                    3 => ::region::Region::Dendy,
                    _ => ::region::Region::Ntsc,
                }
            } else if header[9] & 1 != 0 {
                ::region::Region::Pal
            } else {
                ::region::Region::Ntsc
            },
            submapper: if nes2 { header[8] >> 4 } else { 0 },
            mirroring: if header[6] & 0x08 != 0 {
                Mirroring::FourScreen
            } else if header[6] & 0x01 != 0 {
                Mirroring::Vertical
            } else {
                Mirroring::Horizontal
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn ram_sizes_distinguish_legacy_defaults_and_nes2_volatile_and_battery_memory() {
        let mut bytes = vec![0; 16 + 32768];
        bytes[..5].copy_from_slice(b"NES\x1a\x02");
        let legacy = Rom::read(&mut Cursor::new(&bytes)).unwrap();
        assert_eq!((legacy.prg_ram_size, legacy.chr_ram_size), (8192, 8192));
        bytes[6] = 0xd0; // CPROM's 16 KiB CHR RAM is implicit in legacy files.
        assert_eq!(
            Rom::read(&mut Cursor::new(&bytes)).unwrap().chr_ram_size,
            16384
        );
        bytes[6] = 0x10;
        bytes[7] = 8;
        bytes[10] = 0x77;
        bytes[11] = 7;
        let nes2 = Rom::read(&mut Cursor::new(&bytes)).unwrap();
        assert!(nes2.nes2);
        assert_eq!((nes2.prg_ram_size, nes2.chr_ram_size), (16384, 8192));
        bytes[10] = 0;
        bytes[11] = 0;
        let absent = Rom::read(&mut Cursor::new(bytes)).unwrap();
        assert_eq!((absent.prg_ram_size, absent.chr_ram_size), (0, 0));
    }

    #[test]
    fn ines_mirroring_flags_reach_the_mapped_ppu() {
        use interconnect::{Interconnect, MemoryMappingInterconnect};
        for &(flags, mirroring) in &[
            (0x20, Mirroring::Horizontal),
            (0x21, Mirroring::Vertical),
            (0x28, Mirroring::FourScreen),
            (0x29, Mirroring::FourScreen),
        ] {
            let mut bytes = vec![0; 16 + 16384];
            bytes[..4].copy_from_slice(b"NES\x1a");
            bytes[4] = 1;
            bytes[6] = flags;
            let rom = Rom::read(&mut Cursor::new(bytes)).unwrap();
            assert_eq!(rom.mirroring, mirroring);
            assert_eq!(rom.mapper, 2);
            let mut bus = MemoryMappingInterconnect::new(rom);
            // Write a nametable byte through CPU ports, then read each logical
            // table through PPUDATA's real delayed-read interface.
            bus.write_word(0x2006, 0x20);
            bus.write_word(0x2006, 0);
            bus.write_word(0x2007, 0xab);
            for high in &[0x20, 0x24, 0x28, 0x2c] {
                bus.write_word(0x2006, *high);
                bus.write_word(0x2006, 0);
                bus.read_word(0x2007);
                let actual = bus.read_word(0x2007);
                let aliases_zero = *high == 0x20
                    || (mirroring == Mirroring::Horizontal && *high == 0x24)
                    || (mirroring == Mirroring::Vertical && *high == 0x28);
                assert_eq!(actual, if aliases_zero { 0xab } else { 0 });
            }
        }
    }

    #[test]
    fn nes2_mapper_extension_and_submapper_are_decoded_separately() {
        let mut bytes = vec![0; 16 + 16384];
        bytes[..4].copy_from_slice(b"NES\x1a");
        bytes[4] = 1;
        bytes[6] = 0x20;
        bytes[7] = 0x08;
        bytes[8] = 0x21; // Mapper 258, submapper 2; must not alias mapper 2.
        let rom = Rom::read(&mut Cursor::new(&bytes)).unwrap();
        assert_eq!(rom.mapper, 258);
        assert_eq!(rom.submapper, 2);
        bytes[7] = 0; // The same legacy reserved byte has no submapper meaning.
        let rom = Rom::read(&mut Cursor::new(bytes)).unwrap();
        assert_eq!(rom.mapper, 2);
        assert_eq!(rom.submapper, 0);
    }

    #[test]
    fn trainer_is_skipped_before_reading_prg_and_chr_banks() {
        let mut bytes = vec![0; 16 + 512 + 16384 + 8192];
        bytes[..4].copy_from_slice(b"NES\x1a");
        bytes[4] = 1;
        bytes[5] = 1;
        bytes[6] = 0x25;
        bytes[16] = 0x11;
        bytes[16 + 512] = 0xab;
        bytes[16 + 512 + 16384] = 0xcd;
        let rom = Rom::read(&mut Cursor::new(bytes)).unwrap();
        assert_eq!(rom.mirroring, Mirroring::Vertical);
        assert_eq!(rom.prg_rom[0][0], 0xab);
        assert_eq!(rom.chr_rom[0][0], 0xcd);
    }
}

impl ::state::Snapshot for Mirroring {
    fn save(&self, w: &mut ::state::Writer) {
        use state::Snapshot;
        (*self as u8).save(w);
    }
    fn load(&mut self, r: &mut ::state::Reader) -> ::state::Result<()> {
        *self = match r.take(1)?[0] {
            0 => Mirroring::Horizontal,
            1 => Mirroring::Vertical,
            2 => Mirroring::FourScreen,
            3 => Mirroring::SingleLower,
            4 => Mirroring::SingleUpper,
            _ => return Err("Invalid mirroring state".into()),
        };
        Ok(())
    }
}
impl Default for Mirroring {
    fn default() -> Self {
        Mirroring::Horizontal
    }
}
