use std::mem::replace;

use crate::{
    error::Error,
    kv_file::{KVFile, KVLine},
    kvdb::KeyStatus,
    segmented_files_db::segment_file::{
        SegmentFile, SegmentFileFactory, SegmentReader, SegmentReaderFactory,
    },
};

const TMP_FILE_NAME: &str = "merged_tmp_file.txt";

pub struct Reader<'a> {
    kvfile: KVFile,
    sparse_index: &'a Vec<(String, u64)>,
}

impl<'a> SegmentReader<'a> for Reader<'a> {
    fn get_status(&mut self, key: &str) -> Result<Option<KeyStatus<String>>, Error> {
        get_status(self.sparse_index, &mut self.kvfile, key)
    }
}

pub struct File {
    dir_path: String,
    sparsity: u64,
    kvfile: KVFile,
    sparse_index: Vec<(String, u64)>,
    last_indexed_offset: u64,
}

impl SegmentFile for File {
    type Reader<'a> = Reader<'a>;
    fn set_status(&mut self, key: &str, status: &KeyStatus<String>) -> Result<(), Error> {
        self.kvfile.append_line(key, status).and_then(|offset| {
            if self.sparse_index.is_empty() || offset - self.last_indexed_offset > self.sparsity {
                self.sparse_index.push((key.to_owned(), offset));
                self.last_indexed_offset = offset;
            }
            Ok(())
        })
    }
    fn get_status(&mut self, key: &str) -> Result<Option<KeyStatus<String>>, Error> {
        get_status(&self.sparse_index, &mut self.kvfile, key)
    }
    fn absorb<'a>(&mut self, other: &mut Reader<'a>) -> Result<(), Error> {
        let mut new_file = KVFile::new(&self.dir_path, TMP_FILE_NAME)?;
        let mut new_index = vec![];
        let mut last_indexed_offset = 0;

        let mut this_iter = self.kvfile.iter()?;
        let mut this_buf = this_iter.try_next()?;

        let mut other_iter = other.kvfile.iter()?;
        let mut other_buf = other_iter.try_next()?;

        let mut writer_buf = None;

        loop {
            let use_other = match &this_buf {
                None => true,
                Some(KVLine { key: this_key, .. }) => match &other_buf {
                    None => false,
                    Some(KVLine { key: other_key, .. }) => this_key >= other_key,
                },
            };
            let (buf, iter) = match use_other {
                true => (&mut other_buf, &mut other_iter),
                false => (&mut this_buf, &mut this_iter),
            };

            let prev_writer_buf = replace(&mut writer_buf, replace(buf, iter.try_next()?));
            match prev_writer_buf {
                None => {
                    if writer_buf.is_none() {
                        break;
                    }
                }
                Some(KVLine {
                    key: ref prev_key,
                    status: prev_status,
                    ..
                }) => {
                    let should_write = match writer_buf {
                        None => true,
                        Some(KVLine {
                            key: ref current_key,
                            ..
                        }) => current_key > prev_key,
                    };
                    if should_write {
                        let offset = new_file.append_line(&prev_key, &prev_status)?;
                        if new_index.is_empty() || offset - last_indexed_offset > self.sparsity {
                            new_index.push((prev_key.to_owned(), offset));
                            last_indexed_offset = offset;
                        }
                    }
                }
            };
        }

        let mut old_file = replace(&mut self.kvfile, new_file);
        let file_name = old_file.file_name.clone();
        old_file.delete()?;
        self.kvfile.rename(&file_name)?;
        self.sparse_index = new_index;

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
            sparse_index: &file.sparse_index,
        });
    }
}

pub struct Factory {
    pub dir_path: String,
    pub sparsity: u64,
}

impl SegmentFileFactory<File> for Factory {
    fn new(&self, file_name: &str) -> Result<File, Error> {
        let kvfile = KVFile::new(&self.dir_path, file_name)?;
        Ok(File {
            dir_path: self.dir_path.clone(),
            sparsity: self.sparsity,
            kvfile,
            sparse_index: vec![],
            last_indexed_offset: 0,
        })
    }
    fn from_disk(&self, file_name: &str) -> Result<File, Error> {
        let mut kvfile = KVFile::new(&self.dir_path, file_name)?;

        let mut last_indexed_offset = 0;
        let mut sparse_index = vec![];
        for line_result in kvfile.iter()? {
            let KVLine { key, offset, .. } = line_result?;
            if sparse_index.is_empty() || offset - last_indexed_offset > self.sparsity {
                sparse_index.push((key.to_owned(), offset));
                last_indexed_offset = offset;
            }
        }

        Ok(File {
            dir_path: self.dir_path.clone(),
            sparsity: self.sparsity,
            kvfile,
            sparse_index,
            last_indexed_offset,
        })
    }
}

fn get_status(
    sparse_index: &Vec<(String, u64)>,
    kvfile: &mut KVFile,
    key: &str,
) -> Result<Option<KeyStatus<String>>, Error> {
    let index = match sparse_index.binary_search_by(|(this_key, _)| this_key.cmp(&key.to_string()))
    {
        Ok(index) => index,
        Err(0) => return Ok(None),
        Err(index) => index - 1,
    };
    let (_, start_offset) = sparse_index.get(index).unwrap();

    let mut status = None;
    for line_result in kvfile.iter_from_offset(*start_offset)? {
        let line = line_result?;
        if line.key.as_str() > key {
            break;
        }
        if line.key == key {
            status = Some(line.status)
        }
    }
    Ok(status)
}
