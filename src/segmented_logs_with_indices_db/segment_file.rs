use crate::{
    error::Error,
    in_memory_db::InMemoryDb,
    kv_file::KVFile,
    kvdb::KVEntry,
    segmented_db::segment_file::{SegmentFile, SegmentFileFactory},
};
use KVEntry::{Deleted, Present};

pub struct File {
    kvfile: KVFile,
    index: InMemoryDb<KVEntry<u64>>,
    file_size_threshold: u64,
}

impl SegmentFile for File {
    fn get_entry(&self, key: &str) -> Result<Option<KVEntry<String>>, Error> {
        match self.index.get(key) {
            Some(Present(offset)) => self
                .kvfile
                .get_at_offset(offset)
                .and_then(|maybe_value| Ok(maybe_value.and_then(|value| Some(Present(value))))),
            Some(Deleted) => Ok(Some(Deleted)),
            None => Ok(None),
        }
    }
    fn should_replace(&self) -> Result<bool, Error> {
        Ok(self.kvfile.size()? >= self.file_size_threshold)
    }
    fn add_entry(&mut self, key: &str, entry: &KVEntry<String>) -> Result<(), Error> {
        self.kvfile.append_line(key, &entry).and_then(|offset| {
            Ok(self.index.set(
                key,
                &match entry {
                    Present(_) => Present(offset),
                    Deleted => Deleted,
                },
            ))
        })
    }
    fn absorb(&mut self, other: &Self) -> Result<(), Error> {
        for key in other.index.keys() {
            if self.index.get(key).is_none() {
                if let Some(Present(value)) = other.get_entry(&key)? {
                    self.add_entry(key.as_str(), &Present(value))?;
                }
            }
        }
        Ok(())
    }
    fn rename(&mut self, new_file_name: &str) -> Result<(), Error> {
        self.kvfile.rename(new_file_name)
    }
    fn delete(self) -> Result<(), Error> {
        self.kvfile.delete()
    }
}

pub struct Factory {
    pub dir_path: String,
    pub file_size_threshold: u64,
}

impl SegmentFileFactory<File> for Factory {
    fn new(&self, file_name: &str) -> Result<File, Error> {
        let kvfile = KVFile::new(&self.dir_path, file_name);
        let index = InMemoryDb::new();
        Ok(File {
            kvfile,
            index,
            file_size_threshold: self.file_size_threshold,
        })
    }
    fn from_disk(&self, file_name: &str) -> Result<File, Error> {
        let kvfile = KVFile::new(&self.dir_path, file_name);
        let mut index = InMemoryDb::new();
        kvfile.read_lines(&mut |key, entry, offset| {
            match entry {
                Present(_) => index.set(&key, &Present(offset)),
                Deleted => index.set(&key, &Deleted),
            };
            Ok(false)
        })?;
        Ok(File {
            kvfile,
            index,
            file_size_threshold: self.file_size_threshold,
        })
    }
}