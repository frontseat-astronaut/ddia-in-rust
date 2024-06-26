use crate::{
    error::Error,
    in_memory_db::InMemoryDb,
    kv_file::KVFile,
    kvdb::KeyStatus,
    segmented_files_db::segment_file::{
        SegmentFile, SegmentFileFactory, SegmentReader, SegmentReaderFactory,
    },
};
use KeyStatus::{Deleted, Present};

pub struct Reader<'a> {
    kvfile: KVFile,
    index: &'a InMemoryDb<KeyStatus<u64>>,
}

impl<'a> SegmentReader<'a> for Reader<'a> {
    fn get_status(&mut self, key: &str) -> Result<Option<KeyStatus<String>>, Error> {
        get_status(self.index, &mut self.kvfile, key)
    }
}

pub struct File {
    kvfile: KVFile,
    index: InMemoryDb<KeyStatus<u64>>,
    file_size_threshold: u64,
}

impl SegmentFile for File {
    type Reader<'a> = Reader<'a>;

    fn get_status(&mut self, key: &str) -> Result<Option<KeyStatus<String>>, Error> {
        get_status(&self.index, &mut self.kvfile, key)
    }
    fn ready_to_be_archived(&self) -> Result<bool, Error> {
        Ok(self.kvfile.size()? > self.file_size_threshold)
    }
    fn set_status(&mut self, key: &str, status: &KeyStatus<String>) -> Result<(), Error> {
        self.kvfile.append_line(key, &status).and_then(|offset| {
            Ok(self.index.set(
                key,
                &match status {
                    Present(_) => Present(offset),
                    Deleted => Deleted,
                },
            ))
        })
    }
    fn absorb<'a>(&mut self, other: &mut Self::Reader<'a>) -> Result<(), Error> {
        for key in other.index.keys() {
            if self.index.get(key).is_none() {
                self.set_status(key.as_str(), &other.get_status(key)?.unwrap())?;
            }
        }
        Ok(())
    }
    fn rename(&mut self, new_file_name: &str) -> Result<(), Error> {
        self.kvfile.rename(new_file_name)
    }
    fn delete(mut self) -> Result<(), Error> {
        self.kvfile.delete()
    }
}

pub struct ReaderFactory {}

impl SegmentReaderFactory<File> for ReaderFactory {
    fn new<'a>(&self, file: &'a File) -> Result<<File as SegmentFile>::Reader<'a>, Error> {
        return Ok(Reader {
            kvfile: KVFile::copy(&file.kvfile)?,
            index: &file.index,
        });
    }
}

pub struct Factory {
    pub dir_path: String,
    pub file_size_threshold: u64,
}

impl SegmentFileFactory<File> for Factory {
    fn new(&self, file_name: &str) -> Result<File, Error> {
        let kvfile = KVFile::new(&self.dir_path, file_name)?;
        let index = InMemoryDb::new();
        Ok(File {
            kvfile,
            index,
            file_size_threshold: self.file_size_threshold,
        })
    }
    fn from_disk(&self, file_name: &str) -> Result<File, Error> {
        let mut kvfile = KVFile::new(&self.dir_path, file_name)?;
        let mut index = InMemoryDb::new();
        for line_result in kvfile.iter()? {
            let line = line_result?;
            match line.status {
                Present(_) => index.set(&line.key, &Present(line.offset)),
                Deleted => index.set(&line.key, &Deleted),
            }
        }
        Ok(File {
            kvfile,
            index,
            file_size_threshold: self.file_size_threshold,
        })
    }
}

fn get_status(
    index: &InMemoryDb<KeyStatus<u64>>,
    kvfile: &mut KVFile,
    key: &str,
) -> Result<Option<KeyStatus<String>>, Error> {
    match index.get(key) {
        Some(Present(offset)) => kvfile
            .read_at_offset(offset)
            .and_then(|maybe_value| Ok(maybe_value.and_then(|value| Some(Present(value))))),
        Some(Deleted) => Ok(Some(Deleted)),
        None => Ok(None),
    }
}
