use std::{io::{BufRead, BufReader, BufWriter, Lines, Write}, process::{Child, ChildStdin, ChildStdout}};

use napi::bindgen_prelude::Result;

use crate::ipc_handler::IPCHandler;

pub(crate) struct SocketLineIPC {
  child: Child,
  lines: Lines<BufReader<ChildStdout>>,
  writer: BufWriter<ChildStdin>,
}

impl SocketLineIPC {
  pub(crate) fn new(exe: String, args: Vec<String>) -> Result<Self> {
    let mut child = std::process::Command::new(exe)
      .stdin(std::process::Stdio::piped())
      .stdout(std::process::Stdio::piped())
      .args(args)
      .spawn()?;

    Ok(Self {
      lines: BufReader::new(child.stdout.take().unwrap()).lines(),
      writer: BufWriter::new(child.stdin.take().unwrap()),
      child,
    })
  }
}

impl IPCHandler for SocketLineIPC {
  fn read_message(&mut self) -> Option<Result<String>> {
    self.lines.next().map(|line| Ok(line?))
  }

  fn write_message(&mut self, ty: &str, name: &str, payload: &str) -> Result<()> {
    self.writer.write_all(ty.as_bytes())?;
    self.writer.write_all(b"\t")?;
    self.writer.write_all(name.as_bytes())?;
    self.writer.write_all(b"\t")?;
    self.writer.write_all(payload.as_bytes())?;
    self.writer.write_all(b"\n")?;
    self.writer.flush()?;
    Ok(())
  }

  fn close(&mut self) -> Result<()> {
    self.child.kill()?;
    Ok(())
  }
}
