use napi::bindgen_prelude::Result;

pub(crate) trait IPCHandler {
  fn read_message(&mut self) -> Option<Result<String>>;
  fn write_message(&mut self, ty: &str, name: &str, payload: &str) -> Result<()>;
  fn close(&mut self) -> Result<()>;
}
