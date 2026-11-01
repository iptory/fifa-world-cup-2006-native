use anyhow::Context;

const MAGIC: [u8; 4] = *b"XEX2";
const EXECUTION_ID: u32 = 0x0004_0006;
const MAX_HEADERS: u32 = 256;

pub(crate) fn execution_ids(bytes: &[u8]) -> anyhow::Result<(u32, u32, u32)> {
    if bytes.len() < 0x18 {
        anyhow::bail!("XEX too small ({})", bytes.len());
    }
    if bytes[0..4] != MAGIC {
        anyhow::bail!("not an XEX2 image");
    }
    let count = u32::from_be_bytes(
        bytes[0x14..0x18]
            .try_into()
            .map_err(|_| anyhow::anyhow!("corrupt XEX header count"))?,
    );
    if count == 0 || count > MAX_HEADERS {
        anyhow::bail!("corrupt XEX header count {count}");
    }
    let table_end = 0x18_usize
        .checked_add(
            (count as usize)
                .checked_mul(8)
                .context("XEX table overflow")?,
        )
        .context("XEX table overflow")?;
    if bytes.len() < table_end {
        anyhow::bail!("XEX header table truncated");
    }
    for index in 0..count {
        let base = 0x18_usize
            .checked_add(
                (index as usize)
                    .checked_mul(8)
                    .context("XEX entry overflow")?,
            )
            .context("XEX entry overflow")?;
        let id = u32::from_be_bytes(
            bytes[base..base + 4]
                .try_into()
                .map_err(|_| anyhow::anyhow!("corrupt XEX header id"))?,
        );
        let offset = u32::from_be_bytes(
            bytes[base + 4..base + 8]
                .try_into()
                .map_err(|_| anyhow::anyhow!("corrupt XEX header offset"))?,
        );
        if id != EXECUTION_ID {
            continue;
        }
        let start = usize::try_from(offset).map_err(|_| anyhow::anyhow!("bad XEX offset"))?;
        let end = start.checked_add(20).context("XEX execution-id overflow")?;
        if end > bytes.len() {
            anyhow::bail!("XEX execution-id outside file");
        }
        let media = u32::from_be_bytes(
            bytes[start..start + 4]
                .try_into()
                .map_err(|_| anyhow::anyhow!("corrupt XEX media id"))?,
        );
        let version = u32::from_be_bytes(
            bytes[start + 4..start + 8]
                .try_into()
                .map_err(|_| anyhow::anyhow!("corrupt XEX version"))?,
        );
        let title = u32::from_be_bytes(
            bytes[start + 12..start + 16]
                .try_into()
                .map_err(|_| anyhow::anyhow!("corrupt XEX title id"))?,
        );
        return Ok((title, media, version));
    }
    anyhow::bail!("XEX execution-id header missing")
}

#[cfg(test)]
mod tests {
    use super::EXECUTION_ID;

    fn header(body: Option<[u8; 20]>) -> Vec<u8> {
        let mut out = vec![0_u8; 0x18];
        out[..4].copy_from_slice(b"XEX2");
        out[0x14..0x18].copy_from_slice(&1_u32.to_be_bytes());
        out.extend_from_slice(&EXECUTION_ID.to_be_bytes());
        let offset: u32 = if body.is_some() { 0x20 } else { 0 };
        out.extend_from_slice(&offset.to_be_bytes());
        if let Some(exec) = body {
            out.resize(0x20, 0);
            out.extend_from_slice(&exec);
        }
        out
    }

    fn exec(media: u32, version: u32, title: u32) -> [u8; 20] {
        let mut body = [0_u8; 20];
        body[..4].copy_from_slice(&media.to_be_bytes());
        body[4..8].copy_from_slice(&version.to_be_bytes());
        body[12..16].copy_from_slice(&title.to_be_bytes());
        body
    }

    #[test]
    fn parses_supported_execution_id() -> anyhow::Result<()> {
        let bytes = header(Some(exec(0x4C1B_827B, 8, 0x4541_07E6)));
        let (title, media, version) = super::execution_ids(&bytes)?;
        assert!(title == 0x4541_07E6 && media == 0x4C1B_827B && version == 8);
        Ok(())
    }

    #[test]
    fn rejects_wrong_magic() {
        let mut bytes = header(None);
        bytes[0] = b'Q';
        assert!(super::execution_ids(&bytes).is_err());
    }

    #[test]
    fn rejects_truncated_table() {
        let mut bytes = header(None);
        bytes.truncate(0x1c);
        assert!(super::execution_ids(&bytes).is_err());
    }

    #[test]
    fn rejects_missing_execution_id() {
        let mut bytes = header(None);
        bytes[0x18..0x1c].copy_from_slice(&0xDEAD_BEEF_u32.to_be_bytes());
        bytes.resize(0x40, 0);
        assert!(super::execution_ids(&bytes).is_err());
    }

    #[test]
    fn rejects_execution_id_outside_file() {
        let mut bytes = header(None);
        bytes[0x1c..0x20].copy_from_slice(&0x0100_0000_u32.to_be_bytes());
        assert!(super::execution_ids(&bytes).is_err());
    }
}
