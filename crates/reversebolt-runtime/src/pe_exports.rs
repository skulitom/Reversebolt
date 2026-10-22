//! Bounded file-data inspection. No DLL mapping, entry points or target access.
use object::{
    LittleEndian as LE, pe,
    read::{
        ReadCache, ReadRef,
        pe::{ExportTable, ExportTarget, ImageNtHeaders},
    },
};
use std::{cell::Cell, collections::BTreeMap, ops::Range, path::Path};

pub const READ_BUDGET: u64 = 8 * 1024 * 1024;
type Result<T> = std::result::Result<T, String>;

#[derive(Debug, serde::Serialize)]
pub struct Exports {
    pub image_size: u32,
    pub functions: BTreeMap<String, u32>,
    pub forwarded: Vec<String>,
    /// Requested bytes, including repeated reads, capped before cache allocation.
    pub read_bytes: u64,
}
impl Exports {
    pub fn contains_all(&self, names: &[&str]) -> bool {
        names.iter().all(|name| self.functions.contains_key(*name))
    }
}

#[derive(Clone, Copy)]
struct Bounded<'a, R> {
    inner: R,
    remaining: &'a Cell<u64>,
}
impl<'a, R: ReadRef<'a>> ReadRef<'a> for Bounded<'_, R> {
    fn len(self) -> std::result::Result<u64, ()> {
        self.inner.len()
    }
    fn read_bytes_at(self, offset: u64, size: u64) -> std::result::Result<&'a [u8], ()> {
        let remaining = self.remaining.get().checked_sub(size).ok_or(())?;
        self.remaining.set(remaining);
        self.inner.read_bytes_at(offset, size)
    }
    fn read_bytes_at_until(
        self,
        range: Range<u64>,
        delimiter: u8,
    ) -> std::result::Result<&'a [u8], ()> {
        // Bound both the allocation and the scan, rather than trusting an unterminated string.
        let size = range.end.checked_sub(range.start).ok_or(())?.min(4096);
        let bytes = self.read_bytes_at(range.start, size)?;
        let end = bytes.iter().position(|byte| *byte == delimiter).ok_or(())?;
        Ok(&bytes[..end])
    }
}

fn inspect<'a>(data: impl ReadRef<'a>, names: &[&str]) -> Result<Exports> {
    if names.is_empty() || names.len() > 128 || names.iter().any(|n| n.is_empty() || n.len() > 128)
    {
        return Err("Export query exceeds its bounds".into());
    }
    let remaining = Cell::new(READ_BUDGET);
    let data = Bounded {
        inner: data,
        remaining: &remaining,
    };
    let parse = || -> Result<Exports> {
        let dos = pe::ImageDosHeader::parse(data).map_err(|e| e.to_string())?;
        let mut offset = u64::from(dos.nt_headers_offset());
        let (nt, directories) =
            pe::ImageNtHeaders64::parse(data, &mut offset).map_err(|e| e.to_string())?;
        if nt.file_header.machine.get(LE) != pe::IMAGE_FILE_MACHINE_AMD64 {
            return Err("Requires a Windows x64 PE image".into());
        }
        let sections = nt.sections(data, offset).map_err(|e| e.to_string())?;
        let file_size = data.len().map_err(|_| "File size unavailable")?;
        let image_size = nt.optional_header.size_of_image.get(LE);
        let mut result = Exports {
            image_size,
            functions: BTreeMap::new(),
            forwarded: vec![],
            read_bytes: 0,
        };
        let Some(directory) = directories.get(pe::IMAGE_DIRECTORY_ENTRY_EXPORT) else {
            return Ok(result);
        };
        let export_data = directory.data(data, &sections).map_err(|e| e.to_string())?;
        let export_base = directory.virtual_address.get(LE);
        let table = ExportTable::parse(export_data, export_base).map_err(|e| e.to_string())?;
        if table.name_pointers().len() > 65_536 {
            return Err("Export name count exceeds its bounds".into());
        }
        for (pointer, index) in table.name_iter() {
            let start = pointer
                .checked_sub(export_base)
                .ok_or("Invalid export name pointer")? as usize;
            let tail = export_data
                .get(start..)
                .ok_or("Invalid export name pointer")?;
            // Longer names cannot equal a requested name; never scan an arbitrary file-sized string.
            let Some(end) = tail.iter().take(129).position(|byte| *byte == 0) else {
                continue;
            };
            let name = &tail[..end];
            let Some(name) = names.iter().find(|requested| requested.as_bytes() == name) else {
                continue;
            };
            if result.functions.contains_key(*name) || result.forwarded.iter().any(|n| n == name) {
                return Err("Duplicate requested export".into());
            }
            match table.target_by_index(index).map_err(|e| e.to_string())? {
                ExportTarget::Address(rva) => {
                    // Only direct functions backed by executable file bytes are callable.
                    let executable = sections.iter().any(|section| {
                        let start = section.virtual_address.get(LE);
                        let size = section
                            .virtual_size
                            .get(LE)
                            .min(section.size_of_raw_data.get(LE));
                        rva.checked_sub(start).is_some_and(|delta| {
                            delta < size
                                && u64::from(section.pointer_to_raw_data.get(LE)) + u64::from(delta)
                                    < file_size
                        }) && section
                            .characteristics
                            .get(LE)
                            .contains(pe::IMAGE_SCN_MEM_EXECUTE)
                    });
                    if rva == 0 || rva >= image_size || !executable {
                        return Err("Requested export is outside executable image data".into());
                    }
                    result.functions.insert((*name).to_owned(), rva);
                }
                _ => result.forwarded.push((*name).to_owned()),
            }
        }
        Ok(result)
    };
    let mut result = parse().map_err(|e| format!("PE export inspection failed: {e}"))?;
    result.read_bytes = READ_BUDGET - remaining.get();
    Ok(result)
}

pub fn from_bytes(bytes: &[u8], names: &[&str]) -> Result<Exports> {
    inspect(bytes, names)
}

pub fn from_file(path: &Path, names: &[&str]) -> Result<Exports> {
    use std::os::windows::fs::OpenOptionsExt;
    // Deny replacement/writes for this short read; do not load third-party code.
    let file = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(path)
        .map_err(|e| e.to_string())?;
    let cache = ReadCache::new(file);
    inspect(&cache, names)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn u16_at(b: &mut [u8], at: usize, v: u16) {
        b[at..at + 2].copy_from_slice(&v.to_le_bytes());
    }
    fn u32_at(b: &mut [u8], at: usize, v: u32) {
        b[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }
    // Two sections: an export directory at RVA 0x1000 and code at RVA 0x2000.
    // Deliberately different raw offsets catch accidental RVA-as-file-offset reads.
    fn fixture() -> Vec<u8> {
        let mut b = vec![0; 0x800];
        b[..2].copy_from_slice(b"MZ");
        u32_at(&mut b, 0x3c, 0x80);
        b[0x80..0x84].copy_from_slice(b"PE\0\0");
        u16_at(&mut b, 0x84, 0x8664);
        u16_at(&mut b, 0x86, 2);
        u16_at(&mut b, 0x94, 0xf0);
        u16_at(&mut b, 0x96, 0x2022);
        u16_at(&mut b, 0x98, 0x20b);
        u32_at(&mut b, 0xd0, 0x3000);
        u32_at(&mut b, 0xd4, 0x200);
        u32_at(&mut b, 0x104, 16);
        u32_at(&mut b, 0x108, 0x1000);
        u32_at(&mut b, 0x10c, 0x400);
        for (section, va, raw, size, flags) in [
            (0x188, 0x1000, 0x200, 0x400, 0x40000040),
            (0x1b0, 0x2000, 0x600, 0x200, 0x60000020),
        ] {
            u32_at(&mut b, section + 8, size);
            u32_at(&mut b, section + 12, va);
            u32_at(&mut b, section + 16, size);
            u32_at(&mut b, section + 20, raw);
            u32_at(&mut b, section + 36, flags);
        }
        u32_at(&mut b, 0x210, 1); // ordinal base
        u32_at(&mut b, 0x214, 1); // address count
        u32_at(&mut b, 0x218, 1); // name count
        u32_at(&mut b, 0x21c, 0x1040); // addresses
        u32_at(&mut b, 0x220, 0x1050); // names
        u32_at(&mut b, 0x224, 0x1060); // ordinals
        u32_at(&mut b, 0x240, 0x2000);
        u32_at(&mut b, 0x250, 0x1080);
        b[0x280..0x284].copy_from_slice(b"Run\0");
        b
    }
    #[test]
    fn reads_direct_exports_and_distinguishes_missing_and_forwarded() {
        let mut b = fixture();
        let found = from_bytes(&b, &["Run", "Missing"]).unwrap();
        assert_eq!(found.image_size, 0x3000);
        assert_eq!(found.functions["Run"], 0x2000);
        assert!(found.contains_all(&["Run"]));
        assert!(!found.contains_all(&["Run", "Missing"]));
        assert!(found.read_bytes < 2048);
        u32_at(&mut b, 0x240, 0x10a0);
        b[0x2a0..0x2a9].copy_from_slice(b"other.Run");
        let found = from_bytes(&b, &["Run"]).unwrap();
        assert!(found.functions.is_empty());
        assert_eq!(found.forwarded, ["Run"]);
        u32_at(&mut b, 0x108, 0);
        u32_at(&mut b, 0x10c, 0);
        assert!(from_bytes(&b, &["Run"]).unwrap().functions.is_empty());
    }
    #[test]
    fn malformed_or_non_callable_exports_are_rejected() {
        let b = fixture();
        for (offset, value) in [
            (0x3c, u32::MAX),
            (0x10c, u32::MAX),
            (0x218, u32::MAX),
            (0x250, u32::MAX),
            (0x240, 0),
            (0x240, 0x3000),
            (0x240, 0x1800),
            (0x1d4, 0x40000040),
        ] {
            let mut changed = b.clone();
            u32_at(&mut changed, offset, value);
            assert!(from_bytes(&changed, &["Run"]).is_err(), "offset {offset:x}");
        }
        for (offset, value) in [(0x84, 0xaa64), (0x98, 0x10b), (0x260, 7)] {
            let mut changed = b.clone();
            u16_at(&mut changed, offset, value);
            assert!(from_bytes(&changed, &["Run"]).is_err(), "offset {offset:x}");
        }
        for end in 0..0x600 {
            assert!(from_bytes(&b[..end], &["Run"]).is_err());
        }
        assert!(from_bytes(&b[..0x600], &["Run"]).is_err());
        let mut long_name = b.clone();
        long_name[0x280..0x600].fill(b'R');
        assert!(
            from_bytes(&long_name, &["Run"])
                .unwrap()
                .functions
                .is_empty()
        );
        let mut duplicate = b;
        u32_at(&mut duplicate, 0x218, 2);
        u32_at(&mut duplicate, 0x254, 0x1080);
        assert!(from_bytes(&duplicate, &["Run"]).is_err());
    }
    #[test]
    fn budget_is_checked_before_any_underlying_read_or_allocation() {
        #[derive(Clone, Copy)]
        struct NoReads;
        impl<'a> ReadRef<'a> for NoReads {
            fn len(self) -> std::result::Result<u64, ()> {
                Ok(u64::MAX)
            }
            fn read_bytes_at(self, _: u64, _: u64) -> std::result::Result<&'a [u8], ()> {
                panic!("must not reach reader")
            }
            fn read_bytes_at_until(
                self,
                _: Range<u64>,
                _: u8,
            ) -> std::result::Result<&'a [u8], ()> {
                panic!("must not reach reader")
            }
        }
        let remaining = Cell::new(READ_BUDGET);
        let bounded = Bounded {
            inner: NoReads,
            remaining: &remaining,
        };
        assert!(bounded.read_bytes_at(0, READ_BUDGET + 1).is_err());
        assert_eq!(remaining.get(), READ_BUDGET);
        remaining.set(0);
        assert!(bounded.read_bytes_at_until(0..u64::MAX, 0).is_err());
        let remaining = Cell::new(4);
        let bounded = Bounded {
            inner: &[1, 2, 3, 4][..],
            remaining: &remaining,
        };
        assert!(bounded.read_bytes_at(0, 3).is_ok());
        assert!(bounded.read_bytes_at(0, 2).is_err());
    }
}
