use std::io::{self, Read};
use std::path;
use std::fs;

pub struct Rom {
    pub prg_rom: Vec<Vec<u8>>,
    pub chr_rom: Vec<Vec<u8>>,
}

impl Rom {
    pub fn load<P: AsRef<path::Path>>(filename: P) -> io::Result<Rom> {
        let mut file = fs::File::open(filename)?;

        let mut header = [0u8; 16];
        file.read_exact(&mut header)?;

        if header[0..4] != *b"NES\x1a" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid ROM"));
        }

        let prg_rom = (0..header[4]).map(|_| {
                let mut buf = vec![0u8; 16384];
                file.read_exact(&mut buf)?;
                Ok(buf)
            })
            .collect::<io::Result<Vec<Vec<u8>>>>()?;

        let chr_rom = (0..header[5]).map(|_| {
                let mut buf = vec![0u8; 8192];
                file.read_exact(&mut buf)?;
                Ok(buf)
            })
            .collect::<io::Result<Vec<Vec<u8>>>>()?;

        Ok(Rom {
            prg_rom: prg_rom,
            chr_rom: chr_rom,
        })
    }
}
