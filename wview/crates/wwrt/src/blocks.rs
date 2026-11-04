//! A file read in blocks with a cache of a fixed size (`BlockFile`): a view of a file larger than
//! the memory it may take (a binary file, a long log) reads the blocks it shows, and the oldest
//! block goes when the cache is full.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

/// A file and the blocks of it read last.
pub struct BlockFile {
    file: File,
    len: u64,
    block: usize,
    budget: usize,
    cache: HashMap<u64, Cached>,
    tick: u64,
    /// The blocks read from the disk so far, for the tests and the status bar.
    pub reads: u64,
}

struct Cached {
    bytes: Vec<u8>,
    used: u64,
}

impl BlockFile {
    /// The file, read in blocks of `block` bytes with at most `cache` bytes of them in memory
    /// (at least one block).
    pub fn open(path: &Path, block: usize, cache: usize) -> io::Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        let block = block.max(1);
        Ok(Self {
            file,
            len,
            block,
            budget: (cache / block).max(1),
            cache: HashMap::new(),
            tick: 0,
            reads: 0,
        })
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The blocks in memory now.
    pub fn cached(&self) -> usize {
        self.cache.len()
    }

    /// Copies the bytes from `offset` into `out`, as many as the file has there; gives the count.
    pub fn read_at(&mut self, offset: u64, out: &mut [u8]) -> io::Result<usize> {
        let mut done = 0;
        while done < out.len() {
            let at = offset + done as u64;
            if at >= self.len {
                break;
            }
            let index = at / self.block as u64;
            let start = (at % self.block as u64) as usize;
            let bytes = self.block(index)?;
            let n = (bytes.len() - start).min(out.len() - done);
            out[done..done + n].copy_from_slice(&bytes[start..start + n]);
            done += n;
        }
        Ok(done)
    }

    /// A block from the cache, or from the disk after the block used longest ago left it.
    fn block(&mut self, index: u64) -> io::Result<&[u8]> {
        self.tick += 1;
        if !self.cache.contains_key(&index) {
            if self.cache.len() >= self.budget
                && let Some(oldest) = self
                    .cache
                    .iter()
                    .min_by_key(|(_, c)| c.used)
                    .map(|(k, _)| *k)
            {
                self.cache.remove(&oldest);
            }
            let start = index * self.block as u64;
            let size = (self.len - start).min(self.block as u64) as usize;
            let mut bytes = vec![0u8; size];
            self.file.seek(SeekFrom::Start(start))?;
            self.file.read_exact(&mut bytes)?;
            self.reads += 1;
            self.cache.insert(index, Cached { bytes, used: 0 });
        }
        let cached = self
            .cache
            .get_mut(&index)
            .expect("the block was just put in the cache");
        cached.used = self.tick;
        Ok(&cached.bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_read_by_blocks_within_its_cache() {
        let path = std::env::temp_dir().join(format!("wwrt-blocks-{}.bin", std::process::id()));
        let data: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&path, &data).unwrap();
        let mut file = BlockFile::open(&path, 65_536, 2 * 65_536).unwrap();
        assert_eq!(file.len(), 300_000);
        let mut out = vec![0u8; 100];
        // Across the end of block 0 and the start of block 1.
        assert_eq!(file.read_at(65_500, &mut out).unwrap(), 100);
        assert_eq!(out, data[65_500..65_600]);
        assert_eq!((file.reads, file.cached()), (2, 2));
        // Block 1 again from the cache, then block 4 pushes out block 0, the oldest.
        file.read_at(70_000, &mut out).unwrap();
        assert_eq!(file.reads, 2);
        let mut tail = vec![0u8; 100];
        assert_eq!(file.read_at(299_950, &mut tail).unwrap(), 50);
        assert_eq!(tail[..50], data[299_950..]);
        assert_eq!((file.reads, file.cached()), (3, 2));
        file.read_at(70_000, &mut out).unwrap();
        assert_eq!(file.reads, 3, "block 1 stayed, it was used after block 0");
        file.read_at(0, &mut out).unwrap();
        assert_eq!(file.reads, 4, "block 0 left the cache");
        assert_eq!(file.read_at(400_000, &mut out).unwrap(), 0);
        std::fs::remove_file(&path).unwrap();
    }
}
