use xdvdfs::layout::{DirectoryEntryNode, DirectoryEntryTable};

use crate::image::Image;

const MAX_VISITS: u32 = 65_536;
const MAX_ENTRIES: usize = 1_000_000;

pub(crate) struct Entry {
    pub(crate) rel: String,
    pub(crate) data_offset: u64,
    pub(crate) data_len: u64,
    pub(crate) is_dir: bool,
}

pub(crate) fn collect(image: &mut Image, root: (u64, u32)) -> anyhow::Result<Vec<Entry>> {
    let mut tables = vec![(String::new(), root)];
    let mut seen_tables: std::collections::HashSet<(u64, u32)> = std::collections::HashSet::new();
    let mut out = Vec::new();
    let mut lower: std::collections::HashSet<String> = std::collections::HashSet::new();
    while let Some((parent, (offset, size))) = tables.pop() {
        if !seen_tables.insert((offset, size)) {
            anyhow::bail!("corrupt XDVDFS directory (table cycle)");
        }
        if out.len() > MAX_ENTRIES {
            anyhow::bail!("corrupt XDVDFS directory (too many entries)");
        }
        let table = DirectoryEntryTable::new(size, offset_sector(offset)?);
        for node in walk_table(image, table, offset)? {
            let name = node_name(&node)?;
            let rel = if parent.is_empty() {
                name
            } else {
                format!("{parent}/{name}")
            };
            let key = rel.to_ascii_lowercase();
            if !lower.insert(key) {
                anyhow::bail!("case-colliding ISO names at {rel:?}");
            }
            let sector = node.node.dirent.data.sector;
            let entry_size = node.node.dirent.data.size;
            if node.node.dirent.is_directory() {
                if entry_size != 0 {
                    let (child_offset, child_size) = image.table_location(sector, entry_size)?;
                    tables.push((rel.clone(), (child_offset, child_size)));
                }
                out.push(Entry {
                    rel,
                    data_offset: 0,
                    data_len: 0,
                    is_dir: true,
                });
            } else {
                let (data_offset, data_len) = image.data_location(sector, entry_size)?;
                out.push(Entry {
                    rel,
                    data_offset,
                    data_len,
                    is_dir: false,
                });
            }
        }
    }
    out.sort_by(|a, b| a.rel.cmp(&b.rel));
    Ok(out)
}

fn offset_sector(offset: u64) -> anyhow::Result<u32> {
    if !offset.is_multiple_of(2048) {
        anyhow::bail!("unaligned XDVDFS table");
    }
    u32::try_from(offset / 2048).map_err(|_| anyhow::anyhow!("table sector overflow"))
}

fn node_name(node: &DirectoryEntryNode) -> anyhow::Result<String> {
    let len = usize::from(node.node.dirent.filename_length);
    if len == 0 {
        anyhow::bail!("corrupt XDVDFS file name length {len}");
    }
    if node.name[..len].contains(&0) {
        anyhow::bail!("bad ISO path (NUL byte)");
    }
    let text =
        core::str::from_utf8(&node.name[..len]).map_err(|_| anyhow::anyhow!("bad ISO name"))?;
    for c in text.chars() {
        if c.is_control() || "<>:\"/\\|?*".contains(c) {
            anyhow::bail!("bad ISO path component {text:?}");
        }
    }
    Ok(text.to_owned())
}

fn walk_table(
    image: &mut Image,
    table: DirectoryEntryTable,
    base: u64,
) -> anyhow::Result<Vec<DirectoryEntryNode>> {
    if table.region.size == 0 {
        return Ok(Vec::new());
    }
    let mut nodes = Vec::new();
    let mut stack = vec![0_u16];
    let mut seen: std::collections::HashSet<u16> = std::collections::HashSet::new();
    let mut visits = 0_u32;
    while let Some(index) = stack.pop() {
        if !seen.insert(index) {
            anyhow::bail!("corrupt XDVDFS directory (cycle)");
        }
        visits += 1;
        if visits > MAX_VISITS {
            anyhow::bail!("corrupt XDVDFS directory (too many entries)");
        }
        if nodes.len() > MAX_ENTRIES {
            anyhow::bail!("corrupt XDVDFS directory (too many entries)");
        }
        let slot = u32::from(index) * 4;
        if table
            .region
            .size
            .checked_sub(slot)
            .is_none_or(|rest| rest < 14)
        {
            continue;
        }
        let offset = table
            .offset::<std::io::Error>(slot)
            .map_err(|e| anyhow::anyhow!("bad XDVDFS table ({e})"))?;
        let Some(node) = read_node(image, base, table.region.size, offset)? else {
            continue;
        };
        for child in [node.node.left_entry_offset, node.node.right_entry_offset] {
            if child == 0 || child == 0xffff {
                continue;
            }
            let child_slot = u32::from(child) * 4;
            if child_slot >= table.region.size {
                anyhow::bail!("XDVDFS child pointer outside table");
            }
            stack.push(child);
        }
        nodes.push(node);
    }
    Ok(nodes)
}

fn read_node(
    image: &mut Image,
    base: u64,
    size: u32,
    offset: u64,
) -> anyhow::Result<Option<DirectoryEntryNode>> {
    let end = base
        .checked_add(u64::from(size))
        .ok_or_else(|| anyhow::anyhow!("table overflow"))?;
    if offset < base || offset + 14 > end {
        anyhow::bail!("XDVDFS node outside table");
    }
    let mut head = [0_u8; 14];
    image.read_at(offset, &mut head)?;
    if head == [0xff; 14] || head == [0x00; 14] {
        return Ok(None);
    }
    let name_len = usize::from(head[13]);
    if name_len == 0 {
        anyhow::bail!("corrupt XDVDFS file name length {name_len}");
    }
    let name_end = offset
        .checked_add(14 + name_len as u64)
        .ok_or_else(|| anyhow::anyhow!("name overflow"))?;
    if name_end > end {
        anyhow::bail!("XDVDFS file name outside table");
    }
    let mut name = [0_u8; 256];
    image.read_at(offset + 14, &mut name[..name_len])?;
    let node = xdvdfs::layout::DirectoryEntryDiskNode {
        left_entry_offset: u16::from_le_bytes([head[0], head[1]]),
        right_entry_offset: u16::from_le_bytes([head[2], head[3]]),
        dirent: xdvdfs::layout::DirectoryEntryDiskData {
            data: xdvdfs::layout::DiskRegion {
                sector: u32::from_le_bytes([head[4], head[5], head[6], head[7]]),
                size: u32::from_le_bytes([head[8], head[9], head[10], head[11]]),
            },
            attributes: xdvdfs::layout::DirentAttributes(head[12]),
            filename_length: head[13],
        },
    };
    Ok(Some(DirectoryEntryNode { node, name, offset }))
}

#[cfg(test)]
mod tests {
    use super::node_name;

    fn node_with_name(name: &[u8]) -> xdvdfs::layout::DirectoryEntryNode {
        let mut raw = [0_u8; 256];
        raw[..name.len()].copy_from_slice(name);
        xdvdfs::layout::DirectoryEntryNode {
            node: xdvdfs::layout::DirectoryEntryDiskNode {
                left_entry_offset: 0,
                right_entry_offset: 0,
                dirent: xdvdfs::layout::DirectoryEntryDiskData {
                    data: xdvdfs::layout::DiskRegion { sector: 0, size: 0 },
                    attributes: xdvdfs::layout::DirentAttributes(0x20),
                    #[allow(clippy::cast_possible_truncation)]
                    filename_length: name.len() as u8,
                },
            },
            name: raw,
            offset: 0,
        }
    }

    #[test]
    fn rejects_nul_in_name() {
        assert!(node_name(&node_with_name(b"a\0b")).is_err());
    }

    #[test]
    fn rejects_separator_in_name() {
        assert!(node_name(&node_with_name(b"a/b")).is_err());
    }

    #[test]
    fn rejects_empty_name() {
        assert!(node_name(&node_with_name(b"")).is_err());
    }

    #[test]
    fn preserves_empty_directories_without_reading_a_child_table() -> anyhow::Result<()> {
        use std::io::Write as _;
        let mut bytes = vec![0_u8; 34 * 2048];
        let volume = &mut bytes[32 * 2048..33 * 2048];
        volume[..20].copy_from_slice(b"MICROSOFT*XBOX*MEDIA");
        volume[2028..].copy_from_slice(b"MICROSOFT*XBOX*MEDIA");
        volume[20..24].copy_from_slice(&33_u32.to_le_bytes());
        volume[24..28].copy_from_slice(&20_u32.to_le_bytes());
        let node = &mut bytes[33 * 2048..];
        node[12] = 0x10;
        node[13] = 5;
        node[14..19].copy_from_slice(b"empty");
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(&bytes)?;
        let mut image = crate::image::open(file.path())?;
        let root = image.read_volume_root()?;
        let entries = super::collect(&mut image, root)?;
        let actual: Vec<_> = entries
            .iter()
            .map(|entry| (entry.rel.as_str(), entry.is_dir, entry.data_len))
            .collect();
        assert_eq!(actual, [("empty", true, 0)]);
        Ok(())
    }
}
