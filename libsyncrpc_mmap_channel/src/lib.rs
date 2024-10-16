use std::{ffi::OsStr, path::PathBuf, time::Duration};

use bytecheck::CheckBytes;
use mmap_sync::synchronizer::Synchronizer;
use rkyv::{Archive, Deserialize, Serialize};

pub use mmap_sync::synchronizer::SynchronizerError;

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[archive_attr(derive(CheckBytes))]
struct MmapMsg(String);

pub struct MmapIPCChannel {
  reader: Synchronizer,
  writer: Synchronizer,
}

impl MmapIPCChannel {
  pub fn new(read_from: &OsStr, write_to: &OsStr) -> Self {
    let reader = Synchronizer::new(read_from);
    let writer = Synchronizer::new(write_to);
    Self { reader, writer }
  }

  pub fn read(&mut self) -> Result<String, SynchronizerError> {
    unsafe {
      self
        .reader
        .read::<MmapMsg>(true)
        .map(|line| format!("{}", line.0))
    }
  }

  pub fn write(&mut self, msg: String) -> Result<(), SynchronizerError> {
    let data = MmapMsg(msg);
    self.writer.write(&data, Duration::from_secs(1))?;
    Ok(())
  }
}
