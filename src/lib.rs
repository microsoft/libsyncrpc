use std::{
  collections::HashMap, io::{BufRead, BufReader, BufWriter, Read, Write as _}, process::{Child, ChildStdin, ChildStdout}
};

use napi::{
  bindgen_prelude::{Function, FunctionRef, Result}, JsBuffer, Env, Error
};

#[macro_use]
extern crate napi_derive;

/// A synchronous RPC channel that allows JavaScript to synchronously call out
/// to a child process and get a response over a line-based protocol,
/// including handling of JavaScript-side callbacks before the call completes.
///
/// For details on the protocol, see the `README.md`.
#[napi]
pub struct SyncRpcChannel {
  child: Child,
  reader: BufReader<ChildStdout>,
  writer: BufWriter<ChildStdin>,
  callbacks: HashMap<String, FunctionRef<(String, String), String>>,
}

#[napi]
impl SyncRpcChannel {
  #[napi(constructor)]
  pub fn new(exe: String, args: Vec<String>) -> Result<Self> {
    let mut child = std::process::Command::new(exe)
      .stdin(std::process::Stdio::piped())
      .stdout(std::process::Stdio::piped())
      .args(args)
      .spawn()?;

    Ok(Self {
      reader: BufReader::new(child.stdout.take().unwrap()),
      writer: BufWriter::new(child.stdin.take().unwrap()),
      callbacks: HashMap::new(),
      child,
    })
  }

  #[napi]
  pub fn request_binary_sync(&mut self, env: Env, method: String, payload: String) -> Result<JsBuffer> {
    if payload.contains('\n') {
      return Err(Error::from_reason(
        "payload must not contain `\n` characters",
      ));
    }
    self.write_message("request-bin", &method, &payload)?;
    while let Ok(Some(line)) = self.read_line() {
      let mut parts = line.splitn(3, '\t');
      let (ty, name, size_or_payload) = (
        parts
          .next()
          .ok_or_else(|| Error::from_reason("Expected message type from child."))?
          .trim(),
        parts
          .next()
          .ok_or_else(|| Error::from_reason("Expected message name from child."))?
          .trim(),
        parts
          .next()
          .ok_or_else(|| Error::from_reason("Expected message size or payload from child."))?
          .trim(),
      );
      match ty {
        "response-bin" => {
          if name == method {
            // Parse the size from the response
            let size = size_or_payload.parse::<usize>()
              .map_err(|e| Error::from_reason(format!("Invalid binary size: {}", e)))?;
            
            // Create a buffer to hold the binary data
            let buffer = self.read_binary(size)?;

            
            // Convert to NAPI Buffer and return
            return Ok(env.create_buffer_with_data(buffer)?.into_raw());
          } else {
            return Err(Error::from_reason(format!(
              "name mismatch for response: expected `{method}`, got `{name}`"
            )));
          }
        }
        "error" => {
          return Err(self.create_error(name, size_or_payload, &method));
        }
        "call" => {
          self.handle_call(&env, name, size_or_payload)?;
        }
        _ => {
          return Err(Error::from_reason(format!(
            "Invalid message type from child: `{ty}`"
          )));
        }
      }
    }
    Err(Error::from_reason("No response from child/unexpected EOF."))
  }

  /// Send a request to the child process and wait for a response. The method
  /// will not return, synchronously, until a response is received or an error
  /// occurs.
  ///
  /// For details on the protocol, refer to `README.md`.
  #[napi]
  pub fn request_sync(&mut self, env: Env, method: String, payload: String) -> Result<String> {
    if payload.contains('\n') {
      return Err(Error::from_reason(
        "payload must not contain `\n` characters",
      ));
    }
    self.write_message("request", &method, &payload)?;
    // `while let` so we can still call `self.write_message()`, which needs `&mut self`.
    while let Ok(Some(line)) = self.read_line() {
      let mut parts = line.splitn(3, '\t');
      let (ty, name, payload) = (
        parts
          .next()
          .ok_or_else(|| Error::from_reason("Expected message type from child."))?
          .trim(),
        parts
          .next()
          .ok_or_else(|| Error::from_reason("Expected message name from child."))?
          .trim(),
        parts
          .next()
          .ok_or_else(|| Error::from_reason("Expected message payload from child."))?
          .trim(),
      );
      match ty {
        "response" => {
          if name == method {
            return Ok(payload.to_string());
          } else {
            return Err(Error::from_reason(format!(
              "name mismatch for response: expected `{method}`, got `{name}`"
            )));
          }
        }
        "error" => {
          return Err(self.create_error(name, payload, &method));
        }
        "call" => {
          self.handle_call(&env, name, payload)?;
        }
        _ => {
          return Err(Error::from_reason(format!(
            "Invalid message type from child: `{ty}`"
          )));
        }
      }
    }

    Err(Error::from_reason("No response from child/unexpected EOF."))
  }

  /// Registers a JavaScript callback that the child can invoke before
  /// completing a request. The callback will receive a JSON-encoded string as
  /// its argument and should return a JSON-encoded string as its result.
  ///
  /// If the callback throws, an it will be handled appropriately by
  /// `requestSync` and the child will be notified.
  #[napi]
  pub fn register_callback(
    &mut self,
    name: String,
    cb: Function<'static, (String, String), String>,
  ) -> Result<()> {
    self.callbacks.insert(name, cb.create_ref()?);
    Ok(())
  }

  /// Terminates the child process
  #[napi]
  pub fn terminate(&mut self) -> Result<()> {
    self.child.kill()?;
    Ok(())
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

  fn read_line(&mut self) -> Result<Option<String>> {
    let mut line = String::new();
    let bytes_read = self.reader.read_line(&mut line)?;
    if bytes_read == 0 {
        return Ok(None);
    }
    Ok(Some(line))
  }

  fn read_binary(&mut self, size: usize) -> Result<Vec<u8>> {
    let mut buf = vec![0u8; size];
    self.reader.read_exact(&mut buf)?;
    
    // Consume the newline that follows the binary data
    let mut newline_buf = [0u8; 1];
    self.reader.read_exact(&mut newline_buf)?;
    
    // Verify that we actually read a newline
    if newline_buf[0] != b'\n' {
      return Err(Error::from_reason("Expected newline after binary data"));
    }
    
    Ok(buf)
  }

  // Helper method to create an error
  fn create_error(&self, name: &str, payload: &str, expected_method: &str) -> Error {
    if name == expected_method {
      Error::from_reason(payload)
    } else {
      Error::from_reason(format!(
        "name mismatch for response: expected `{expected_method}`, got `{name}`"
      ))
    }
  }

  // Helper method to handle callback calls
  fn handle_call(&mut self, env: &Env, name: &str, payload: &str) -> Result<()> {
    if let Some(cb) = self.callbacks.get(name) {
      match cb.borrow_back(env)?.call((name.into(), payload.into())) {
        Ok(res) => {
          self.write_message("call-response", name, res.trim())?;
        }
        Err(e) => {
          self.write_message("call-error", name, format!("{e}").trim())?;
          return Err(Error::from_reason(format!(
            "Error calling callback `{name}`: {}",
            e
          )));
        }
      }
    } else {
      self.write_message("call-error", name, &format!("unknown callback: `{name}`. Please make sure to register it on the JavaScript side before invoking it."))?;
      return Err(Error::from_reason(format!(
        "no callback named `{name}` found"
      )));
    }
    Ok(())
  }
}
