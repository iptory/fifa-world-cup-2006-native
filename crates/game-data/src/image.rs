use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use anyhow::Context;
use xdvdfs::blockdev::{BlockDeviceRead as _, OffsetWrapper};

const PARTITION_BASES: &[u64] = &[0, 405_798_912, 265_879_552, 34_078_720];
const SECTOR: u64 = 2048;
const VOLUME_SECTOR: u64 = 32;
const MAX_TABLE_SIZE: u32 = 256 * 1024 * 1024;
const COPY_CHUNK: u64 = 1 << 20;

pub(crate) struct Image {
    dev: OffsetWrapper<BufReader<File>, std::io::Error>,
    len: u64,
    copy_buffer: Vec<u8>,
}

pub(crate) fn open(path: &Path) -> anyhow::Result<Image> {
    let base = probe_base(path)?;
    let file = File::open(path).with_context(|| format!("open ISO {}", path.display()))?;
    let total = file.metadata().context("read ISO metadata")?.len();
    let len = total
        .checked_sub(base)
        .context("ISO smaller than partition")?;
    let dev = OffsetWrapper::new(BufReader::new(file))
        .map_err(|e| anyhow::anyhow!("not an Xbox 360 XDVDFS image ({e})"))?;
    Ok(Image {
        dev,
        len,
        copy_buffer: Vec::new(),
    })
}

fn probe_base(path: &Path) -> anyhow::Result<u64> {
    let mut file = File::open(path).with_context(|| format!("open ISO {}", path.display()))?;
    let total = file.metadata().context("read ISO metadata")?.len();
    let mut sector = [0_u8; 2048];
    for base in PARTITION_BASES {
        let at = base
            .checked_add(VOLUME_SECTOR * SECTOR)
            .context("partition probe overflow")?;
        if at.checked_add(2048).is_none_or(|end| end > total) {
            continue;
        }
        file.seek(SeekFrom::Start(at)).context("seek ISO")?;
        if file.read_exact(&mut sector).is_err() {
            continue;
        }
        if is_volume(&sector) {
            return Ok(*base);
        }
    }
    anyhow::bail!("not an Xbox 360 XDVDFS image")
}

fn is_volume(sector: &[u8; 2048]) -> bool {
    sector[..20] == *b"MICROSOFT*XBOX*MEDIA" && sector[2048 - 20..] == *b"MICROSOFT*XBOX*MEDIA"
}

impl Image {
    pub(crate) fn read_volume_root(&mut self) -> anyhow::Result<(u64, u32)> {
        let volume = xdvdfs::read::read_volume(&mut self.dev)
            .map_err(|e| anyhow::anyhow!("bad XDVDFS volume ({e})"))?;
        let sector = volume.root_table.region.sector;
        let size = volume.root_table.region.size;
        self.table_location(sector, size)
    }

    pub(crate) fn table_location(&self, sector: u32, size: u32) -> anyhow::Result<(u64, u32)> {
        if size == 0 || size > MAX_TABLE_SIZE {
            anyhow::bail!("corrupt XDVDFS table (size {size})");
        }
        let offset = u64::from(sector)
            .checked_mul(SECTOR)
            .context("XDVDFS sector overflow")?;
        let end = offset
            .checked_add(u64::from(size))
            .context("XDVDFS table overflow")?;
        if end > self.len {
            anyhow::bail!("XDVDFS table outside image bounds");
        }
        Ok((offset, size))
    }

    pub(crate) fn data_location(&self, sector: u32, size: u32) -> anyhow::Result<(u64, u64)> {
        let size = u64::from(size);
        let offset = u64::from(sector)
            .checked_mul(SECTOR)
            .context("ISO file offset overflow")?;
        let end = offset
            .checked_add(size)
            .context("ISO file range overflow")?;
        if end > self.len {
            anyhow::bail!("ISO file outside image bounds");
        }
        Ok((offset, size))
    }

    pub(crate) fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> anyhow::Result<()> {
        let end = offset
            .checked_add(buf.len() as u64)
            .context("ISO read offset overflow")?;
        if end > self.len {
            anyhow::bail!("ISO read outside image bounds");
        }
        self.dev
            .read(offset, buf)
            .map_err(|e| anyhow::anyhow!("read ISO ({e})"))?;
        Ok(())
    }

    pub(crate) fn read_bytes(
        &mut self,
        offset: u64,
        size: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<u8>> {
        if size > limit {
            anyhow::bail!("ISO file too large ({size})");
        }
        let end = offset
            .checked_add(size)
            .context("ISO file range overflow")?;
        if end > self.len {
            anyhow::bail!("ISO file outside image bounds");
        }
        let mut buf =
            vec![0_u8; usize::try_from(size).map_err(|_| anyhow::anyhow!("file too large"))?];
        if !buf.is_empty() {
            self.read_at(offset, &mut buf)?;
        }
        Ok(buf)
    }

    pub(crate) fn stream_to(
        &mut self,
        offset: u64,
        size: u64,
        out: &mut impl std::io::Write,
        mut on_chunk: impl FnMut(u64),
    ) -> anyhow::Result<u64> {
        let end = offset
            .checked_add(size)
            .context("ISO file range overflow")?;
        if end > self.len {
            anyhow::bail!("ISO file outside image bounds");
        }
        let mut buf = std::mem::take(&mut self.copy_buffer);
        buf.resize(
            usize::try_from(COPY_CHUNK).map_err(|_| anyhow::anyhow!("chunk"))?,
            0,
        );
        let mut remaining = size;
        let mut at = offset;
        while remaining > 0 {
            let want = remaining.min(COPY_CHUNK);
            let slot = usize::try_from(want).map_err(|_| anyhow::anyhow!("chunk"))?;
            self.read_at(at, &mut buf[..slot])?;
            out.write_all(&buf[..slot]).context("write prepared file")?;
            at = at.checked_add(want).context("stream overflow")?;
            remaining -= want;
            on_chunk(want);
        }
        self.copy_buffer = buf;
        Ok(size)
    }
}
