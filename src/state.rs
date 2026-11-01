//! Portable, versioned state payloads. ROM bytes and host/device handles stay out.
use std::cell::Cell;

pub type Result<T> = ::std::result::Result<T, String>;
pub struct Writer(pub Vec<u8>);
pub struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl Writer {
    pub fn new() -> Self {
        Writer(Vec::new())
    }
    pub fn bytes(&mut self, bytes: &[u8]) {
        self.0.extend_from_slice(bytes);
    }
}
impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Reader { bytes, position: 0 }
    }
    pub fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let end = self
            .position
            .checked_add(count)
            .ok_or("Invalid state length")?;
        if end > self.bytes.len() {
            return Err("Truncated state".into());
        }
        let bytes = &self.bytes[self.position..end];
        self.position = end;
        Ok(bytes)
    }
    pub fn finished(&self) -> bool {
        self.position == self.bytes.len()
    }
}

pub trait Snapshot {
    fn save(&self, writer: &mut Writer);
    fn load(&mut self, reader: &mut Reader) -> Result<()>;
}
macro_rules! state_fields {
    ($ty:ty, $($field:ident),+ $(,)?) => {
        state_fields!($ty, $($field),+; |_: &$ty| true);
    };
    ($ty:ty, $($field:ident),+; $valid:expr) => {
        impl ::state::Snapshot for $ty {
            fn save(&self, w: &mut ::state::Writer) { $(::state::Snapshot::save(&self.$field, w);)+ }
            fn load(&mut self, r: &mut ::state::Reader) -> ::state::Result<()> {
                $(::state::Snapshot::load(&mut self.$field, r)?;)+
                if ($valid)(self) { Ok(()) } else { Err("Invalid state values".into()) }
            }
        }
    };
}
macro_rules! number {
    ($($ty:ty),+) => { $(
        impl Snapshot for $ty {
            fn save(&self, w: &mut Writer) { w.bytes(&self.to_le_bytes()); }
            fn load(&mut self, r: &mut Reader) -> Result<()> {
                let mut bytes = [0; ::std::mem::size_of::<$ty>()];
                bytes.copy_from_slice(r.take(::std::mem::size_of::<$ty>())?);
                *self = <$ty>::from_le_bytes(bytes); Ok(())
            }
        }
    )+ };
}
number!(u8, u16, u32, u64, i16, i32);
impl Snapshot for bool {
    fn save(&self, w: &mut Writer) {
        (*self as u8).save(w);
    }
    fn load(&mut self, r: &mut Reader) -> Result<()> {
        let value = r.take(1)?[0];
        if value > 1 {
            return Err("Invalid state flag".into());
        }
        *self = value != 0;
        Ok(())
    }
}
impl Snapshot for usize {
    fn save(&self, w: &mut Writer) {
        (*self as u32).save(w);
    }
    fn load(&mut self, r: &mut Reader) -> Result<()> {
        let mut value = 0u32;
        value.load(r)?;
        if value > 16 * 1024 * 1024 {
            return Err("Invalid state index".into());
        }
        *self = value as usize;
        Ok(())
    }
}
impl<T: Snapshot, const N: usize> Snapshot for [T; N] {
    fn save(&self, w: &mut Writer) {
        for value in self {
            value.save(w);
        }
    }
    fn load(&mut self, r: &mut Reader) -> Result<()> {
        for value in self {
            value.load(r)?;
        }
        Ok(())
    }
}
impl<T: Snapshot> Snapshot for Vec<T> {
    fn save(&self, w: &mut Writer) {
        self.len().save(w);
        for value in self {
            value.save(w);
        }
    }
    fn load(&mut self, r: &mut Reader) -> Result<()> {
        let mut length = 0usize;
        length.load(r)?;
        // A cartridge's memory sizes are fixed by its ROM, never by a save file.
        if length != self.len() {
            return Err("State memory size does not match cartridge".into());
        }
        for value in self {
            value.load(r)?;
        }
        Ok(())
    }
}
impl<T: Snapshot + Default> Snapshot for Option<T> {
    fn save(&self, w: &mut Writer) {
        self.is_some().save(w);
        if let Some(value) = self {
            value.save(w);
        }
    }
    fn load(&mut self, r: &mut Reader) -> Result<()> {
        let mut present = false;
        present.load(r)?;
        *self = if present {
            let mut value = T::default();
            value.load(r)?;
            Some(value)
        } else {
            None
        };
        Ok(())
    }
}
impl<T: Snapshot + Copy> Snapshot for Cell<T> {
    fn save(&self, w: &mut Writer) {
        self.get().save(w);
    }
    fn load(&mut self, r: &mut Reader) -> Result<()> {
        let mut value = self.get();
        value.load(r)?;
        self.set(value);
        Ok(())
    }
}

pub fn checksum(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb88320 & (0u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}
