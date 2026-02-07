use std::fs;
use std::io::{self, Read};
use std::path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    FourScreen,
}

pub struct Rom {
    pub prg_rom: Vec<Vec<u8>>,
    pub chr_rom: Vec<u8>,
    pub mapper: u8,
    pub mirroring: Mirroring,
    pub chr_ram_size: usize,
}

impl Rom {
    pub fn load<P: AsRef<path::Path>>(filename: P) -> io::Result<Rom> {
        let mut file = fs::File::open(filename)?;

        let mut header = [0u8; 16];
        file.read_exact(&mut header)?;

        if header[0..4] != *b"NES\x1a" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid ROM"));
        }

        let disk_dude = *b"DiskDude!";
        if header[7..16] == disk_dude {
            header[7] = 0;
            for byte in header[8..16].iter_mut() {
                *byte = 0;
            }
        }

        if header[6] & 0x04 != 0 {
            let mut trainer = [0u8; 512];
            file.read_exact(&mut trainer)?;
        }

        let mirroring = if header[6] & 0x08 != 0 {
            Mirroring::FourScreen
        } else if header[6] & 0x01 != 0 {
            Mirroring::Vertical
        } else {
            Mirroring::Horizontal
        };

        let mapper = (header[7] & 0xf0) | (header[6] >> 4);

        let prg_rom = (0..header[4])
            .map(|_| {
                let mut buf = vec![0u8; 16384];
                file.read_exact(&mut buf)?;
                Ok(buf)
            })
            .collect::<io::Result<Vec<Vec<u8>>>>()?;

        let chr_size = header[5] as usize * 8192;
        let mut chr_rom = vec![0u8; chr_size];
        if chr_size > 0 {
            file.read_exact(&mut chr_rom)?;
        }

        Ok(Rom {
            prg_rom: prg_rom,
            chr_rom: chr_rom,
            mapper: mapper,
            mirroring: mirroring,
            chr_ram_size: if chr_size == 0 { 8192 } else { 0 },
        })
    }
}
