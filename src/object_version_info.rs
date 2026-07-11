use anyhow::bail;
use anyhow::ensure;
use anyhow::Context;
use object::pe::RT_VERSION;
use object::read::pe::ResourceDirectory;
use object::read::pe::ResourceDirectoryTable;
use object::read::pe::SectionTable;
use object::LittleEndian as LE;
use object::U16;
use object::U32;

fn read_padding<'data, R>(reader: R, offset: &mut u64) -> anyhow::Result<()>
where
    R: object::read::ReadRef<'data>,
{
    let padding_size = 4 - (*offset % 4);
    if padding_size != 4 {
        let padding = reader
            .read_bytes(offset, padding_size)
            .ok()
            .context("Failed to read padding")?;
        ensure!(padding.iter().all(|b| *b == 0));
    }

    Ok(())
}

fn read_utf16_nul_string<'data, R>(reader: R, offset: &mut u64) -> anyhow::Result<String>
where
    R: object::read::ReadRef<'data>,
{
    let mut raw = Vec::new();
    while raw.is_empty() || *raw.last().unwrap() != 0 {
        let value: U16<LE> = *reader
            .read(offset)
            .ok()
            .context("Failed to read wide char")?;
        raw.push(value.get(LE));
    }

    let value = String::from_utf16(&raw)?;

    Ok(value)
}

#[derive(Debug)]
pub struct FixedFileInfo {
    pub struct_version: u32,
    pub file_version: u64,
    pub product_version: u64,
    pub file_flags_mask: u32,
    pub file_flags: u32,
    pub file_os: u32,
    pub file_type: u32,
    pub file_subtype: u32,
    pub file_date: u64,
}

impl FixedFileInfo {
    fn parse<'data, R>(reader: R, offset: &mut u64) -> anyhow::Result<Self>
    where
        R: object::read::ReadRef<'data>,
    {
        let signature: U32<LE> = *reader
            .read(offset)
            .ok()
            .context("Failed to read signature")?;
        ensure!(signature.get(LE) == 0xFEEF04BD);

        let struct_version: U32<LE> = *reader
            .read(offset)
            .ok()
            .context("Failed to read struct version")?;
        let struct_version = struct_version.get(LE);

        let file_version_ms: U32<LE> = *reader
            .read(offset)
            .ok()
            .context("Failed to read file version ms")?;
        let file_version_ls: U32<LE> = *reader
            .read(offset)
            .ok()
            .context("Failed to read file version ls")?;
        let file_version =
            (u64::from(file_version_ms.get(LE)) << 32) | u64::from(file_version_ls.get(LE));

        let product_version_ms: U32<LE> = *reader
            .read(offset)
            .ok()
            .context("Failed to read product version ms")?;
        let product_version_ls: U32<LE> = *reader
            .read(offset)
            .ok()
            .context("Failed to read product version ls")?;
        let product_version =
            (u64::from(product_version_ms.get(LE)) << 32) | u64::from(product_version_ls.get(LE));

        let file_flags_mask: U32<LE> = *reader
            .read(offset)
            .ok()
            .context("Failed to read file flags mask")?;
        let file_flags_mask = file_flags_mask.get(LE);

        let file_flags: U32<LE> = *reader
            .read(offset)
            .ok()
            .context("Failed to read file flags")?;
        let file_flags = file_flags.get(LE);

        let file_os: U32<LE> = *reader.read(offset).ok().context("Failed to read file os")?;
        let file_os = file_os.get(LE);

        let file_type: U32<LE> = *reader
            .read(offset)
            .ok()
            .context("Failed to read file type")?;
        let file_type = file_type.get(LE);

        let file_subtype: U32<LE> = *reader
            .read(offset)
            .ok()
            .context("Failed to read file subtype")?;
        let file_subtype = file_subtype.get(LE);

        let file_date_ms: U32<LE> = *reader
            .read(offset)
            .ok()
            .context("Failed to read file date ms")?;

        let file_date_ls: U32<LE> = *reader
            .read(offset)
            .ok()
            .context("Failed to read file date ls")?;
        let file_date = (u64::from(file_date_ms.get(LE)) << 32) | u64::from(file_date_ls.get(LE));

        Ok(Self {
            struct_version,
            file_version,
            product_version,
            file_flags_mask,
            file_flags,
            file_os,
            file_type,
            file_subtype,
            file_date,
        })
    }
}

#[derive(Debug)]
pub struct StringStruct {
    pub key: String,
    pub value: Vec<u16>,
}

impl StringStruct {
    fn parse<'data, R>(reader: R, offset: &mut u64) -> anyhow::Result<Self>
    where
        R: object::read::ReadRef<'data>,
    {
        let start_offset = *offset;

        let length: U16<LE> = *reader.read(offset).ok().context("Failed to read length")?;
        let length = length.get(LE);

        let value_length: U16<LE> = *reader
            .read(offset)
            .ok()
            .context("Failed to read value length")?;
        let value_length = value_length.get(LE);

        let type_: U16<LE> = *reader
            .read(offset)
            .ok()
            .context("Failed to read value length")?;
        let type_ = type_.get(LE);
        ensure!(type_ == 1, "Unsupported string struct type {type_}");

        let key = read_utf16_nul_string(reader, offset)?;

        read_padding(reader, offset)?;

        let value: &[U16<LE>] = reader
            .read_slice(offset, value_length.into())
            .ok()
            .context("Failed to read value")?;
        let value: Vec<u16> = value.iter().map(|value| value.get(LE)).collect();

        ensure!(*offset - start_offset == u64::from(length));

        Ok(Self { key, value })
    }
}

#[derive(Debug)]
pub struct StringTable {
    pub key: String,
    pub children: Vec<StringStruct>,
}

impl StringTable {
    fn parse<'data, R>(reader: R, offset: &mut u64) -> anyhow::Result<Self>
    where
        R: object::read::ReadRef<'data>,
    {
        let start_offset = *offset;

        let length: U16<LE> = *reader.read(offset).ok().context("Failed to read length")?;
        let length = length.get(LE);

        let value_length: U16<LE> = *reader
            .read(offset)
            .ok()
            .context("Failed to read value length")?;
        ensure!(value_length.get(LE) == 0);

        let type_: U16<LE> = *reader.read(offset).ok().context("Failed to read type")?;
        ensure!(type_.get(LE) == 1);

        let key: &[u16] = reader
            .read_slice(offset, 8)
            .ok()
            .context("Failed to read key")?;
        let key = String::from_utf16(key)?;
        ensure!(key.bytes().all(|b| b.is_ascii_hexdigit()));
        ensure!(key.len() == 8);

        read_padding(reader, offset)?;

        let mut children = Vec::new();
        loop {
            let string = StringStruct::parse(reader, offset)?;
            children.push(string);

            let current_length = *offset - start_offset;
            ensure!(current_length <= u64::from(length));
            if current_length == u64::from(length) {
                break;
            }

            read_padding(reader, offset)?;
        }

        Ok(Self { key, children })
    }

    /*
    /// Get the language code
    pub fn language(&self) -> u16 {
        u16::from_str_radix(&self.key[..4], 16).unwrap()
    }

    /// Get the code page
    pub fn code_page(&self) -> u16 {
        u16::from_str_radix(&self.key[4..], 16).unwrap()
    }
    */
}

#[derive(Debug)]
pub struct StringFileInfo {
    pub children: Vec<StringTable>,
}

#[derive(Debug)]
pub struct VersionInfo {
    pub fixed_file_info: Option<FixedFileInfo>,
    pub string_file_info: Option<StringFileInfo>,
}

impl VersionInfo {
    /// See: https://learn.microsoft.com/en-us/windows/win32/menurc/vs-versioninfo
    fn parse<'data, R>(reader: R, offset: &mut u64, expected_size: u64) -> anyhow::Result<Self>
    where
        R: object::read::ReadRef<'data>,
    {
        let start_offset = *offset;

        let _length: U16<LE> = *reader.read(offset).ok().context("Failed to read length")?;

        let value_length: U16<LE> = *reader
            .read(offset)
            .ok()
            .context("Failed to read value length")?;

        let type_: U16<LE> = *reader.read(offset).ok().context("Failed to read type")?;
        ensure!(type_.get(LE) == 0, "text version data is not supported");

        let expected_key = "VS_VERSION_INFO\0";
        let key: &[u16] = reader
            .read_slice(offset, expected_key.len())
            .ok()
            .context("Failed to read key")?;
        let key = String::from_utf16(key)?;
        ensure!(expected_key == key);

        read_padding(reader, offset)?;

        let value_length_u64 = u64::from(value_length.get(LE));
        let fixed_file_info = if value_length_u64 != 0 {
            ensure!(value_length.get(LE) == 52);
            Some(FixedFileInfo::parse(reader, offset)?)
        } else {
            None
        };

        let read_size = *offset - start_offset;
        ensure!(read_size <= expected_size);
        if read_size == expected_size {
            return Ok(Self {
                fixed_file_info,
                string_file_info: None,
            });
        }

        let mut maybe_string_file_info: Option<Option<StringFileInfo>> = None;
        let string_file_info_key = "StringFileInfo\0";
        let var_file_info_key = "VarFileInfo\0";
        let key_peek_len = std::cmp::min(string_file_info_key.len(), var_file_info_key.len());
        loop {
            read_padding(reader, offset)?;

            let start_offset = *offset;

            let length: U16<LE> = *reader.read(offset).ok().context("Failed to read length")?;
            let length = length.get(LE);

            let value_length: U16<LE> = *reader
                .read(offset)
                .ok()
                .context("Failed to read value length")?;
            ensure!(value_length.get(LE) == 0);

            let type_: U16<LE> = *reader.read(offset).ok().context("Failed to read type")?;
            ensure!(type_.get(LE) == 1);

            let key_bytes: &[u16] = reader
                .read_slice(offset, key_peek_len)
                .ok()
                .context("Failed to read key bytes")?;
            let key = String::from_utf16(key_bytes)?;
            if key == string_file_info_key[..key_peek_len] {
                ensure!(maybe_string_file_info.is_none());

                let remaining_key_bytes: &[u16] = reader
                    .read_slice(offset, string_file_info_key.len() - key_peek_len)
                    .ok()
                    .context("Failed to read remaining key bytes")?;
                let remaining_key_bytes = String::from_utf16(remaining_key_bytes)?;
                ensure!(string_file_info_key[key_peek_len..] == remaining_key_bytes);

                read_padding(reader, offset)?;

                let mut children = Vec::with_capacity(1);
                loop {
                    let table = StringTable::parse(reader, offset)?;
                    children.push(table);

                    let current_length = *offset - start_offset;
                    ensure!(current_length <= u64::from(length));
                    if current_length == u64::from(length) {
                        break;
                    }
                }

                let string_file_info = StringFileInfo { children };

                maybe_string_file_info = Some(Some(string_file_info));
            } else if key == var_file_info_key[..key_peek_len] {
                // TODO: Parse this
                break;
            } else {
                bail!("Unknown key \"{key}\"");
            }
        }
        // TODO: Is this right?
        let string_file_info = maybe_string_file_info.flatten();

        Ok(Self {
            fixed_file_info,
            string_file_info,
        })
    }
}

fn get_version_entry_common<'a, R>(
    reader: R,
    section_table: SectionTable<'a>,
    resource_directory: ResourceDirectory<'a>,
    root: &ResourceDirectoryTable<'a>,
) -> anyhow::Result<Option<VersionInfo>>
where
    R: object::read::ReadRef<'a>,
{
    let entry = root
        .entries
        .iter()
        .find(|entry| entry.name_or_id().id() == Some(RT_VERSION));
    let entry = match entry {
        Some(entry) => entry,
        None => return Ok(None),
    };

    let data = entry.data(resource_directory)?;
    let table = data.table().context("Object VERSION data is not a table")?;

    let data = table
        .entries
        .first()
        .context("Object VERSION table missing entry 0")?
        .data(resource_directory)?;
    let table = data
        .table()
        .context("Object VERSION table entry 0 is not a table")?;

    let data = table
        .entries
        .first()
        .context("Object VERSION table entry 0 table missing entry 0")?
        .data(resource_directory)?
        .data()
        .context("Object VERSION table entry 0 table entry 0 is not data")?;
    let offset = data.offset_to_data.get(LE);
    let size = usize::try_from(data.size.get(LE))?;
    // I'm not sure this applies here?
    // let code_page = data.code_page.get(LE);

    let (offset, _) = section_table
        .pe_file_range_at(offset)
        .context("Section missing version offset address")?;
    let mut offset = u64::from(offset);
    let version_info = VersionInfo::parse(reader, &mut offset, u64::try_from(size)?)?;

    Ok(Some(version_info))
}

pub trait GetVersionInfo {
    fn get_version_info(&self) -> anyhow::Result<Option<VersionInfo>>;
}

impl<'a, R> GetVersionInfo for object::read::pe::PeFile32<'a, R>
where
    R: object::read::ReadRef<'a>,
{
    fn get_version_info(&self) -> anyhow::Result<Option<VersionInfo>> {
        let section_table = self.section_table();
        let data_directories = self.data_directories();
        let resource_directory =
            data_directories.resource_directory(self.data(), &section_table)?;
        let resource_directory = match resource_directory {
            Some(resource_directory) => resource_directory,
            None => return Ok(None),
        };
        let root = resource_directory.root()?;
        get_version_entry_common(self.data(), section_table, resource_directory, &root)
    }
}

impl<'a, R> GetVersionInfo for object::read::pe::PeFile64<'a, R>
where
    R: object::read::ReadRef<'a>,
{
    fn get_version_info(&self) -> anyhow::Result<Option<VersionInfo>> {
        let section_table = self.section_table();
        let data_directories = self.data_directories();
        let resource_directory =
            data_directories.resource_directory(self.data(), &section_table)?;
        let resource_directory = match resource_directory {
            Some(resource_directory) => resource_directory,
            None => return Ok(None),
        };
        let root = resource_directory.root()?;
        get_version_entry_common(self.data(), section_table, resource_directory, &root)
    }
}

impl<'a, R> GetVersionInfo for object::read::File<'a, R>
where
    R: object::read::ReadRef<'a>,
{
    fn get_version_info(&self) -> anyhow::Result<Option<VersionInfo>> {
        match self {
            object::read::File::Pe32(file) => file.get_version_info(),
            object::read::File::Pe64(file) => file.get_version_info(),
            _ => bail!("Unsupported object file format {:?}", self.format()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use object::read::ReadCache;
    use std::fs::File;

    #[test]
    fn basic() {
        let file = File::open("test-data/Decrypter.exe").expect("Failed to open test file");
        let reader = ReadCache::new(file);

        let file = object::File::parse(&reader).expect("Failed to parse pe file");

        let version_info = file.get_version_info().expect("Failed to get version info");
        dbg!(version_info);
    }
}
