use std::process::Child;

use libsyncrpc_mmap_channel::MmapIPCChannel;
use napi::{bindgen_prelude::Result, Error};
use tempfile::TempDir;

use crate::ipc_handler::IPCHandler;

pub(crate) struct MmapIPC {
  child: Child,
  channel: MmapIPCChannel,
  // We just keep this around so the tempdir gets dropped properly
  #[allow(dead_code)]
  tmp: TempDir,
}

impl MmapIPC {
  pub(crate) fn new(exe: String, args: Vec<String>) -> Result<Self> {
    let tmp = TempDir::new()?;
    let child = std::process::Command::new(exe).args(args).spawn()?;

    Ok(Self {
      child,
      channel: MmapIPCChannel::new(
        tmp.path().join("read").as_os_str(),
        tmp.path().join("write").as_os_str(),
      ),
      tmp,
    })
  }
}

impl IPCHandler for MmapIPC {
  fn read_message(&mut self) -> Option<Result<String>> {
    Some(
      self
        .channel
        .read()
        .map_err(|e| Error::from_reason(format!("{e}"))),
    )
  }

  fn write_message(&mut self, ty: &str, name: &str, payload: &str) -> Result<()> {
    let data = format!("{}\t{}\t{}", ty, name, payload);
    self
      .channel
      .write(data)
      .map_err(|e| Error::from_reason(format!("{e}")))
  }

  fn close(&mut self) -> Result<()> {
    self.child.kill()?;
    Ok(())
  }
}
