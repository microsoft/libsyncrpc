use std::{
  collections::HashMap,
  io::{BufReader, BufWriter},
  process::{Child, ChildStdin, ChildStdout},
};

use napi::{
  bindgen_prelude::{Function, FunctionRef, Result, Uint8Array},
  Env, Error,
};

use libsyncrpc_connection::RpcConnection;

#[macro_use]
extern crate napi_derive;

pub type MessageComponents = (Vec<u8>, Vec<u8>, Vec<u8>);
pub type Callback = Function<'static, (String, String), String>;

/// A synchronous RPC channel that allows JavaScript to synchronously call out
/// to a child process and get a response over a line-based protocol,
/// including handling of JavaScript-side callbacks before the call completes.
///
/// #### Protocol
///
/// Requests follow a simple delimiter-and-size-based protocol that communicates
/// with the child process through the child's stdin and stdout streams.
///
/// All payloads are assumed to be pre-encoded JSON strings on either end--this API
/// does not do any of its own JSON or even string encoding/decoding itself.  it.
///
/// The child should handle the following messages through its `stdin`. In all
/// below examples, `<payload-size>` is a 4-byte sequence representing an unsigned
/// 32-bit integer. The following `<payload>` will be that many bytes long. Each
/// message ends once the payload ends. The payload may be interpreted in
/// different ways depending on the message, for example as raw binary data or a
/// UTF-8 string. All other values (`<name>`, `<method>`, etc) are expected to be
/// UTF-8-encoded bytes.
///
/// * `request\t<method>\t<payload-size><payload>`: a request to the child with the
///   given raw byte `<payload>`, with `<method>` as the method name. The child should
///   send back any number of `call` messages and close the request with either a
///   `response` or `error` message.
/// * `call-response\t<name>\t<payload-size><payload>`: a response to a `call`
///   message that the child previously sent. The `<payload>` is the return value
///   from invoking the JavaScript callback associated with it. If the callback
///   errors, `call-error` will be sent to the child.
/// * `call-error\t<name>\t<payload-size><payload>`: informs the child that an error
///   occurred. The `<payload>` will be the binary representation of the stringified
///   error, as UTF-8 bytes, not necessarily in JSON format. The method linked to this
///   message will also throw an error after sending this message to its child and
///   terminate the request call.
///
/// The channel handles the following messages from the child's `stdout`:
///
/// * `response\t<method>\t<payload-size><payload>`: a response to a request that the
///   call was for. `<method>` MUST match the `request`
///   message's `<method>` argument.
/// * `error\t<method>\t<payload-size><payload>`: a response that denotes some error
///   occurred while processing the request on the child side. The `<payload>` will
///   simply be the binary representation of the stringified error, as UTF-8 bytes,
///   not necessarily in JSON format. The method associated with this call will also
///   throw an error after receiving this message from the child.
/// * `call\t<name>\t<payload-size><payload>`: a request to invoke a pre-registered
///   JavaScript callback (see `registerCallback`). `<name>` is the name of the
///   callback, and `<payload>` is an encoded UTF-8 string that the callback will be
///   called with. The child should then listen for `call-response` and `call-error`
///   messages.
///
#[napi]
pub struct SyncRpcChannel {
  child: Child,
  conn: RpcConnection<BufReader<ChildStdout>, BufWriter<ChildStdin>>,
  callbacks: HashMap<String, FunctionRef<(String, String), String>>,
}

#[napi]
impl SyncRpcChannel {
  /// Constructs a new `SyncRpcChannel` by spawning a child process with the
  /// given `exe` executable, and a given set of `args`.
  #[napi(constructor)]
  pub fn new(exe: String, args: Vec<String>) -> Result<Self> {
    let mut child = std::process::Command::new(exe)
      .stdin(std::process::Stdio::piped())
      .stdout(std::process::Stdio::piped())
      .stderr(std::process::Stdio::inherit())
      .args(args)
      .spawn()?;
    Ok(Self {
      conn: RpcConnection::new(
        BufReader::new(child.stdout.take().expect("Where did ChildStdout go?")),
        BufWriter::new(child.stdin.take().expect("Where did ChildStdin go?")),
      )?,
      callbacks: HashMap::new(),
      child,
    })
  }

  /// Send a request to the child process and wait for a response. The method
  /// will not return, synchronously, until a response is received or an error
  /// occurs.
  ///
  /// This method will take care of encoding and decoding the binary payload to
  /// and from a JS string automatically and suitable for smaller payloads.
  #[napi]
  pub fn request_sync(&mut self, env: Env, method: String, payload: String) -> Result<String> {
    self
      .request_bytes_sync(env, method, payload.as_bytes())
      .and_then(|arr| {
        String::from_utf8((&arr[..]).into()).map_err(|e| {
          Error::from_reason(format!("Error while encoding response as a string: {e}"))
        })
      })
  }

  /// Send a request to the child process and wait for a response. The method
  /// will not return, synchronously, until a response is received or an error
  /// occurs.
  ///
  /// Unlike `requestSync`, this method will not do any of its own encoding or
  /// decoding of payload data. Everything will be as sent/received through the
  /// underlying protocol.
  #[napi]
  pub fn request_binary_sync(
    &mut self,
    env: Env,
    method: String,
    payload: Uint8Array,
  ) -> Result<Uint8Array> {
    self.request_bytes_sync(env, method, &payload)
  }

  fn request_bytes_sync(&mut self, env: Env, method: String, payload: &[u8]) -> Result<Uint8Array> {
    let method_bytes = method.as_bytes();
    self.conn.write(b"request", method_bytes, payload)?;
    while let Ok(Some((ty, name, payload))) = self.conn.read() {
      match &ty[..] {
        b"response" => {
          if name == method_bytes {
            return Ok(payload.into());
          } else {
            let name = String::from_utf8_lossy(&name);
            return Err(Error::from_reason(format!(
              "name mismatch for response: expected `{method}`, got `{name}`"
            )));
          }
        }
        #[cfg(feature = "mmap")]
        b"mmap" => {
          if &name == b"resize" {
            println!("Got a resize request from child???");
            let (size, _) = payload.split_at(size_of::<usize>());
            self.conn.resize_mmap_ack(usize::from_le_bytes(
              size
                .try_into()
                .map_err(|e| Error::from_reason(format!("Bad mmap size bytes. {e}")))?,
            ))?;
          } else {
            return Err(Error::from_reason(format!(
              "Invalid mmap message from child: {}",
              String::from_utf8_lossy(&name)
            )));
          }
        }
        b"error" => {
          return Err(
            self
              .conn
              .create_error(&String::from_utf8_lossy(&name), payload, &method)
              .into(),
          );
        }
        b"call" => {
          self.handle_call(&env, &String::from_utf8_lossy(&name), payload)?;
        }
        _ => match String::from_utf8(ty) {
          Ok(ty) => {
            return Err(Error::from_reason(format!(
              "Invalid message type from child: `{ty}`"
            )));
          }
          Err(e) => {
            return Err(Error::from_reason(format!("{e}")));
          }
        },
      }
    }
    Err(Error::from_reason("No response from child/unexpected EOF."))
  }

  /// Registers a JavaScript callback that the child can invoke before
  /// completing a request. The callback will receive a string name and a string
  /// payload as its arguments and should return a string as its result.
  ///
  /// There is currently no `Uint8Array`-only equivalent to this functionality.
  ///
  /// If the callback throws, an it will be handled appropriately by
  /// `requestSync` and the child will be notified.
  #[napi(ts_args_type = "name: string, callback: (name: string, payload: string) => string")]
  pub fn register_callback(&mut self, name: String, cb: Callback) -> Result<()> {
    self.callbacks.insert(name, cb.create_ref()?);
    Ok(())
  }

  // Closes the channel, terminating its underlying process.
  #[napi]
  pub fn close(&mut self) -> Result<()> {
    self.child.kill()?;
    Ok(())
  }

  // Helper method to handle callback calls
  fn handle_call(&mut self, env: &Env, name: &str, payload: Vec<u8>) -> Result<()> {
    if let Some(cb) = self.callbacks.get(name) {
      match cb.borrow_back(env)?.call((
        name.into(),
        String::from_utf8(payload).map_err(|e| {
          Error::from_reason(format!(
            "Failed to deserialize callback payload into a string: {e}"
          ))
        })?,
      )) {
        Ok(res) => {
          self
            .conn
            .write(b"call-response", name.as_bytes(), res.as_bytes())?;
        }
        Err(e) => {
          self.conn.write(
            b"call-error",
            name.as_bytes(),
            format!("{e}").trim().as_bytes(),
          )?;
          return Err(Error::from_reason(format!(
            "Error calling callback `{name}`: {}",
            e
          )));
        }
      }
    } else {
      self.conn.write(b"call-error", name.as_bytes(), format!("unknown callback: `{name}`. Please make sure to register it on the JavaScript side before invoking it.").as_bytes())?;
      return Err(Error::from_reason(format!(
        "no callback named `{name}` found"
      )));
    }
    Ok(())
  }
}
