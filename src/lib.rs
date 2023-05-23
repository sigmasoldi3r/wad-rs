use std::{fs::File, io::Read, mem::size_of, os::unix::prelude::FileExt, path::Path};

#[derive(Debug)]
pub struct Location(i32);

#[derive(Debug)]
pub struct Size(i32);

pub enum LumpNameError {
    TooLarge,
}

pub struct LumpName([u8; 8]);
impl LumpName {
    pub fn from_string(str: String) -> Result<LumpName, LumpNameError> {
        if str.len() > 8 {
            Err(LumpNameError::TooLarge)
        } else {
            use std::io::Write;
            let mut buf = [0; 8];
            let mut w: &mut [u8] = &mut buf;
            w.write(str.as_bytes()).unwrap();
            Ok(LumpName(buf))
        }
    }
}
impl ToString for LumpName {
    fn to_string(&self) -> String {
        String::from_utf8_lossy(&self.0)
            .trim_matches(char::from(0))
            .into()
    }
}
impl std::fmt::Debug for LumpName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("LumpName({})", self.to_string()).as_str())
    }
}

#[derive(Debug)]
pub struct EntryType(u8);

#[derive(Debug)]
pub struct Compression(u8);

#[derive(Debug)]
pub struct Entry {
    pub start: Location,
    pub size: Size,
    pub real_size: Size,
    pub kind: EntryType,
    pub compression: Compression,
    pub padding: i16,
    pub name: LumpName,
}

#[derive(Debug)]
pub struct Signature([u8; 4]);
impl ToString for Signature {
    fn to_string(&self) -> String {
        String::from_utf8_lossy(&self.0).into_owned()
    }
}

#[derive(Debug)]
pub enum WadDecodeError {
    FailedToOpenFile(std::io::Error),
    FailedToReadHeader(std::io::Error),
    CouldNotDecodeHeader,
    FailedToReadDirectory(std::io::Error),
    CouldNotDecodeDirectory,
}

#[derive(Debug)]
pub struct Wad {
    pub signature: Signature,
    pub directory: Vec<Entry>,
}
impl Wad {
    pub fn from_file_path<P>(wad_path: P) -> Result<Wad, WadDecodeError>
    where
        P: AsRef<Path>,
    {
        #[repr(C, packed)]
        struct RawHeader {
            signature: [u8; 4],
            count: i32,
            dir_loc: i32,
        }
        let mut fin = File::open(wad_path).map_err(|err| WadDecodeError::FailedToOpenFile(err))?;
        let mut raw_header: [u8; size_of::<RawHeader>()] = [0; size_of::<RawHeader>()];
        fin.read_exact(&mut raw_header)
            .map_err(|err| WadDecodeError::FailedToReadHeader(err))?;
        let (_, raw_header, _) = unsafe { raw_header.align_to::<RawHeader>() };
        let raw_header = raw_header.get(0);
        if raw_header.is_none() {
            return Err(WadDecodeError::CouldNotDecodeHeader);
        }
        let raw_header = raw_header.unwrap();
        let mut wad = Wad {
            signature: Signature(raw_header.signature.clone()),
            directory: vec![],
        };
        #[repr(C, packed)]
        struct RawEntry {
            file_pos: i32,
            size: i32,
            name: [u8; 8],
        }
        const ENTRY_SIZE: usize = size_of::<RawEntry>();
        let mut entry_buf: [u8; ENTRY_SIZE] = [0; ENTRY_SIZE];
        for i in 0..raw_header.count as u64 {
            fin.read_exact_at(
                &mut entry_buf,
                i * ENTRY_SIZE as u64 + (raw_header.dir_loc as u64),
            )
            .map_err(|err| WadDecodeError::FailedToReadDirectory(err))?;
            let (_, raw_entry, _) = unsafe { entry_buf.align_to::<RawEntry>() };
            let raw_entry = raw_entry.get(0);
            if let Some(entry) = raw_entry {
                wad.directory.push(Entry {
                    start: Location(entry.file_pos),
                    size: Size(entry.size),
                    real_size: Size(entry.size),
                    kind: EntryType(0_u8),
                    compression: Compression(0_u8),
                    padding: 0,
                    name: LumpName(entry.name),
                })
            } else {
                return Err(WadDecodeError::CouldNotDecodeDirectory);
            }
        }
        Ok(wad)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_doom_wad() {
        let wad = Wad::from_file_path("DOOM.WAD").unwrap();
        let e1m1 = wad.directory.get(6).unwrap().name.to_string();
        assert!(
            e1m1 == "E1M1".to_string(),
            "The 6th name was not E1M1, found: {:?}",
            e1m1
        );
    }
}
