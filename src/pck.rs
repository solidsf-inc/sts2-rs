use anyhow::{bail, Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use md5::{Digest, Md5};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const GODOT_PCK_MAGIC: u32 = 0x43504447; // "GDPC" in Little Endian

#[derive(Debug)]
pub struct PckEntry {
    pub path_parts: Vec<String>,
    pub offset: u64,
    pub size: u64,
    pub digest: [u8; 16],
}

pub fn extract_localization(pck_path: &Path, root_dir: &Path) -> Result<usize> {
    let mut file = File::open(pck_path)
        .with_context(|| format!("Failed to open PCK file at {}", pck_path.display()))?;

    let magic = file.read_u32::<LittleEndian>()?;
    if magic != GODOT_PCK_MAGIC {
        bail!(
            "Invalid Godot PCK magic: 0x{:08X}, expected 0x{:08X}",
            magic,
            GODOT_PCK_MAGIC
        );
    }

    let version = file.read_u32::<LittleEndian>()?;
    if version != 2 && version != 3 {
        bail!("Unsupported PCK version: {}, expected 2 or 3", version);
    }

    let _v_major = file.read_u32::<LittleEndian>()?;
    let _v_minor = file.read_u32::<LittleEndian>()?;
    let _v_patch = file.read_u32::<LittleEndian>()?;
    let flags = file.read_u32::<LittleEndian>()?;
    if (flags & 1) != 0 {
        bail!("Encrypted PCK packages are not supported");
    }

    let base_offset = file.read_u64::<LittleEndian>()?;

    if version == 3 {
        let directory_offset = file.read_u64::<LittleEndian>()?;
        file.seek(SeekFrom::Start(directory_offset))?;
    } else {
        file.seek(SeekFrom::Current(64))?;
    }

    let count = file.read_u32::<LittleEndian>()?;
    let mut matched_entries = Vec::new();

    for _ in 0..count {
        let name_len = file.read_u32::<LittleEndian>()? as usize;
        let mut name_buf = vec![0u8; name_len];
        file.read_exact(&mut name_buf)?;
        let name = String::from_utf8_lossy(&name_buf)
            .trim_end_matches('\0')
            .to_string();

        let offset = file.read_u64::<LittleEndian>()?;
        let size = file.read_u64::<LittleEndian>()?;
        let mut digest = [0u8; 16];
        file.read_exact(&mut digest)?;
        let entry_flags = file.read_u32::<LittleEndian>()?;

        let stripped = name.strip_prefix("res://").unwrap_or(&name);
        let parts: Vec<String> = stripped.split('/').map(|s| s.to_string()).collect();

        if parts.len() == 3
            && parts[0] == "localization"
            && (parts[1] == "eng" || parts[1] == "zhs")
            && parts[2].ends_with(".json")
        {
            if entry_flags != 0 {
                bail!("Unsupported PCK entry flags: {} on {}", entry_flags, name);
            }
            matched_entries.push(PckEntry {
                path_parts: parts,
                offset: base_offset + offset,
                size,
                digest,
            });
        }
    }

    if matched_entries.is_empty() {
        bail!("No English/Chinese localization tables found in PCK");
    }

    let mut contents: Vec<(PathBuf, Vec<u8>)> = Vec::new();
    for entry in matched_entries {
        file.seek(SeekFrom::Start(entry.offset))?;
        let mut data = vec![0u8; entry.size as usize];
        file.read_exact(&mut data)?;

        let mut hasher = Md5::new();
        hasher.update(&data);
        let hash = hasher.finalize();
        if hash[..] != entry.digest[..] {
            bail!("PCK checksum mismatch for {:?}", entry.path_parts);
        }

        // Verify JSON validity
        let _: serde_json::Value = serde_json::from_slice(&data)
            .with_context(|| format!("Invalid JSON in PCK table: {:?}", entry.path_parts))?;

        let lang_folder = format!("localization_{}", entry.path_parts[1]);
        let target_path = root_dir.join(lang_folder).join(&entry.path_parts[2]);
        contents.push((target_path, data));
    }

    let count = contents.len();
    for (path, data) in contents {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, data)?;
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::WriteBytesExt;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_extract_localization_roundtrip() -> Result<()> {
        let dir = tempdir()?;
        let pck_path = dir.path().join("game.pck");
        let output_dir = dir.path().join("out");

        let json_data = br#"{"STRIKE.title":"Strike","DEFEND.title":"Defend"}"#;
        let mut hasher = Md5::new();
        hasher.update(json_data);
        let digest: [u8; 16] = hasher.finalize().into();

        let file_name = b"localization/eng/cards.json\0";
        let mut directory = Vec::new();
        directory.write_u32::<LittleEndian>(1)?; // count
        directory.write_u32::<LittleEndian>(file_name.len() as u32)?;
        directory.write_all(file_name)?;
        directory.write_u64::<LittleEndian>(0)?; // relative offset
        directory.write_u64::<LittleEndian>(json_data.len() as u64)?;
        directory.write_all(&digest)?;
        directory.write_u32::<LittleEndian>(0)?; // flags

        let header_size: u64 = 96;
        let base = header_size + directory.len() as u64;

        let mut pck = File::create(&pck_path)?;
        pck.write_u32::<LittleEndian>(GODOT_PCK_MAGIC)?;
        pck.write_u32::<LittleEndian>(2)?; // version 2
        pck.write_u32::<LittleEndian>(4)?;
        pck.write_u32::<LittleEndian>(5)?;
        pck.write_u32::<LittleEndian>(1)?;
        pck.write_u32::<LittleEndian>(0)?; // flags
        pck.write_u64::<LittleEndian>(base)?;
        pck.write_all(&[0u8; 64])?; // padding to 96 bytes

        pck.write_all(&directory)?;
        pck.write_all(json_data)?;
        drop(pck);

        let extracted = extract_localization(&pck_path, &output_dir)?;
        assert_eq!(extracted, 1);

        let target_file = output_dir.join("localization_eng").join("cards.json");
        assert!(target_file.exists());
        let read_content = fs::read(target_file)?;
        assert_eq!(read_content, json_data);

        Ok(())
    }
}
