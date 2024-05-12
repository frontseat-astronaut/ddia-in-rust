use crate::{
    error::Error,
    kvdb::KVDb,
    segmented_db::{SegmentCreationPolicy, SegmentedDb},
};

use self::segment_file::{Factory, File};

mod segment_file;

pub struct SegmentedLogsWithIndicesDb {
    segmented_db: SegmentedDb<File, Factory>,
}

impl KVDb for SegmentedLogsWithIndicesDb {
    fn set(&mut self, key: &str, value: &str) -> Result<(), Error> {
        self.segmented_db.set(key, value)
    }
    fn delete(&mut self, key: &str) -> Result<(), Error> {
        self.segmented_db.delete(key)
    }
    fn get(&self, key: &str) -> Result<Option<String>, Error> {
        self.segmented_db.get(key)
    }
}

impl SegmentedLogsWithIndicesDb {
    pub fn new(
        dir_path: &str,
        file_size_threshold: u64,
        merging_threshold: u64,
    ) -> Result<SegmentedLogsWithIndicesDb, Error> {
        Ok(SegmentedLogsWithIndicesDb {
            segmented_db: SegmentedDb::<File, Factory>::new(
                dir_path,
                merging_threshold,
                SegmentCreationPolicy::Automatic,
                Factory {
                    dir_path: dir_path.to_owned(),
                    file_size_threshold,
                },
            )?,
        })
    }
}
