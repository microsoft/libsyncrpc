use std::io::{self, BufRead, Result, Write};

#[cfg(feature = "mmap")]
use std::fs::File;

#[cfg(feature = "mmap")]
use memmap2::MmapMut;

pub type MessageComponents = (Vec<u8>, Vec<u8>, Vec<u8>);

#[cfg(feature = "mmap")]
static INITIAL_MMAP_SIZE: usize = 1024 * 1024;
#[cfg(feature = "mmap")]
static MAX_MMAP_SIZE: usize = isize::MAX as usize;

/// Lower-level wrapper around RPC-related messaging and process management.
pub struct RpcConnection<R: BufRead, W: Write> {
  reader: R,
  writer: W,
  #[cfg(feature = "mmap")]
  tmp: File,
  #[cfg(feature = "mmap")]
  mmap_size: usize,
  #[cfg(feature = "mmap")]
  mmap: MmapMut,
}

impl<R: BufRead, W: Write> RpcConnection<R, W> {
  pub fn new(reader: R, writer: W) -> Result<Self> {
    #[cfg(feature = "mmap")]
    let tmp = tempfile::tempfile()?;
    #[cfg(feature = "mmap")]
    tmp.set_len(INITIAL_MMAP_SIZE as u64)?;
    #[cfg(feature = "mmap")]
    let mmap = unsafe { MmapMut::map_mut(&tmp)? };
    Ok(Self {
      reader,
      writer,
      #[cfg(feature = "mmap")]
      tmp,
      #[cfg(feature = "mmap")]
      mmap,
      #[cfg(feature = "mmap")]
      mmap_size: INITIAL_MMAP_SIZE,
    })
  }

  pub fn write(&mut self, ty: &[u8], name: &[u8], payload: &[u8]) -> Result<()> {
    #[cfg(feature = "mmap")]
    let payload_len = payload.len();
    #[cfg(feature = "mmap")]
    if payload_len > self.mmap_size {
      // eprintln!("Resizing from {} to {}", self.mmap_size, payload_len);
      self.resize_mmap(payload_len)?;
    }
    self.writer.write_all(ty)?;
    self.writer.write_all(b"\t")?;
    self.writer.write_all(name)?;
    self.writer.write_all(b"\t")?;
    // eprintln!("Payload: {payload:?}");
    #[cfg(feature = "mmap")]
    self.mmap[..payload_len].copy_from_slice(payload);
    self
      .writer
      .write_all(&(payload.len() as u32).to_le_bytes())?;
    #[cfg(not(feature = "mmap"))]
    self.writer.write_all(payload)?;
    self.writer.flush()?;
    Ok(())
  }

  pub fn read(&mut self) -> Result<Option<MessageComponents>> {
    let (mut ty, mut name, mut payload_len) = (vec![], vec![], [0u8; 4]);
    if self.reader.read_until(b'\t', &mut ty)? == 0 {
      return Ok(None);
    }
    if self.reader.read_until(b'\t', &mut name)? == 0 {
      return Ok(None);
    }
    self.reader.read_exact(&mut payload_len)?;
    let payload_len = u32::from_le_bytes(payload_len) as usize;
    let mut payload = vec![0; payload_len];
    #[cfg(feature = "mmap")]
    {
      //   if payload_len > self.mmap_size {
      //     let Some((ty, name, payload)) = self.read()? else {
      //       return Err(io::Error::other("oops, connection died"));
      //     };
      //     if &ty == b"mmap" && &name == b"resize" {
      //       self.resize_mmap_ack(usize::from_le_bytes(
      //         payload
      //           .try_into()
      //           .map_err(|_| io::Error::other("Failed to convert usize."))?,
      //       ))?;
      //     } else {
      //       return Err(io::Error::other(
      //         "Unexpected message when mmap should have resized",
      //       ));
      //     }
      //   }
      payload.copy_from_slice(&self.mmap[..payload_len]);
    }
    #[cfg(not(feature = "mmap"))]
    self.reader.read_exact(&mut payload)?;
    // slice off the tabs
    ty.truncate(ty.len() - 1);
    name.truncate(name.len() - 1);
    Ok(Some((ty, name, payload)))
  }

  // Helper method to create an error
  pub fn create_error(&self, name: &str, payload: Vec<u8>, expected_method: &str) -> io::Error {
    if name == expected_method {
      let payload = match String::from_utf8(payload) {
        Ok(payload) => payload,
        Err(err) => return io::Error::other(format!("{err}")),
      };
      io::Error::other(payload)
    } else {
      io::Error::other(format!(
        "name mismatch for response: expected `{expected_method}`, got `{name}`"
      ))
    }
  }

  #[cfg(feature = "mmap")]
  pub fn resize_mmap(&mut self, new_size: usize) -> io::Result<()> {
    if new_size > MAX_MMAP_SIZE {
      return Err(io::Error::other(format!("Max message payload size is {MAX_MMAP_SIZE}, but attempted to send a payload of size {new_size}.")));
    }
    let mut new_mmap_size = self.mmap_size;
    while new_mmap_size < new_size {
      new_mmap_size *= 2;
    }
    // eprintln!("Telling child to resize to {new_mmap_size}");
    self.tmp.set_len(new_mmap_size as u64)?;
    self.mmap = unsafe { MmapMut::map_mut(&self.tmp)? };
    self.write(b"mmap", b"resize", &new_mmap_size.to_le_bytes())?;
    // eprintln!("Waiting for child response...");
    let Some((ty, name, _)) = self.read()? else {
      return Err(io::Error::other(
        "Failed to resize mmap: child disconnected.",
      ));
    };
    if !(&ty == b"mmap" && &name == b"resize-suceeded") {
      return Err(io::Error::other(
        "Failed to resize mmap on the child side: unexpected response from child.",
      ));
    }
    self.mmap_size = new_mmap_size;
    // eprintln!("Child responded that it resized properly.");
    Ok(())
  }

  #[cfg(feature = "mmap")]
  pub fn resize_mmap_ack(&mut self, new_size: usize) -> io::Result<()> {
    // eprintln!("Received resize request to {new_size}. Acking.");
    self.mmap_size = new_size;
    self.mmap = unsafe { MmapMut::map_mut(&self.tmp)? };
    self.write(b"mmap", b"resize-succeeded", b"")?;
    Ok(())
  }
}
