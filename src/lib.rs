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

pub type Callback = Function<'static, (String, String), String>;

/// A synchronous RPC channel that allows JavaScript to synchronously call out
/// to a child process and get a response over a line-based protocol,
/// including handling of JavaScript-side callbacks before the call completes.
///
/// #### Protocol
///
/// Requests follow a MessagePack-based "tuple"/array protocol with 3 items:
/// `(<type>, <name>, <payload>)`. All items are binary arrays of 8-bit
/// integers, including the `<type>` and `<name>`, to avoid unnecessary
/// encoding/decoding at the protocol level.
///
/// For specific message types and their corresponding protocol behavior, please
/// see `MessageType` below.
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
    self
      .conn
      .write(MessageType::Request as u8, method_bytes, payload)?;
    loop {
      let (ty, name, payload) = self.conn.read()?;
      match ty.try_into().map_err(Error::from_reason)? {
        MessageType::Response => {
          if name == method_bytes {
            return Ok(payload.into());
          } else {
            let name = String::from_utf8_lossy(&name);
            return Err(Error::from_reason(format!(
              "name mismatch for response: expected `{method}`, got `{name}`"
            )));
          }
        }
        MessageType::Error => {
          return Err(
            self
              .conn
              .create_error(&String::from_utf8_lossy(&name), payload, &method)
              .into(),
          );
        }
        MessageType::Call => {
          self.handle_call(&env, &String::from_utf8_lossy(&name), payload)?;
        }
        _ => {
          return Err(Error::from_reason(format!(
            "Invalid message type from child: {ty:?}"
          )))
        }
      }
    }
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
          self.conn.write(
            MessageType::CallResponse as u8,
            name.as_bytes(),
            res.as_bytes(),
          )?;
        }
        Err(e) => {
          self.conn.write(
            MessageType::CallError as u8,
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
      self.conn.write(MessageType::CallError as u8, name.as_bytes(), format!("unknown callback: `{name}`. Please make sure to register it on the JavaScript side before invoking it.").as_bytes())?;
      return Err(Error::from_reason(format!(
        "no callback named `{name}` found"
      )));
    }
    Ok(())
  }
}

/// Messages types exchanged between the channel and its child. All messages
/// have an associated `<name>` and `<payload>`, which will both be arrays of
/// 8-bit integers (`Uint8Array`s).
#[napi]
#[repr(u8)]
pub enum MessageType {
  // --- Sent by channel---
  /// A request to the child with the given raw byte `<payload>`, with
  /// `<name>` as the method name. The child may send back any number of
  /// `MessageType.Call` messages and must then close the request with either a
  /// `MessageType.Response`, or a `MessageType.Error`.  message.
  Request = 1,
  /// A response to a `MessageType.Call` message that the child previously sent.
  /// The `<payload>` is the return value from invoking the JavaScript callback
  /// associated with it. If the callback errors, `MessageType.CallError` will
  /// be sent to the child.
  CallResponse,
  /// Informs the child that an error occurred. The `<payload>` will be the
  /// binary representation of the stringified error, as UTF-8 bytes, not
  /// necessarily in JSON format. The method linked to this message will also
  /// throw an error after sending this message to its child and terminate the
  /// request call.
  CallError,

  // --- Sent by child ---
  /// A response to a request that the call was for. `<name>` MUST match the
  /// `MessageType.Request` message's `<name>` argument.
  Response,
  /// A response that denotes some error occurred while processing the request
  /// on the child side. The `<payload>` will simply be the binary
  /// representation of the stringified error, as UTF-8 bytes, not necessarily
  /// in JSON format. The method associated with this call will also throw an
  /// error after receiving this message from the child.
  Error,
  /// A request to invoke a pre-registered JavaScript callback (see
  /// `SyncRpcChannel#registerCallback`). `<name>` is the name of the callback,
  /// and `<payload>` is an encoded UTF-8 string that the callback will be
  /// called with. The child should then listen for `MessageType.CallResponse`
  /// and `MessageType.CallError` messages.
  Call,
  // NOTE: Do NOT put any variants below this one, always add them _before_ it.
  // See comment in TryFrom impl, and remove this when `variant_count` stabilizes.
  _UnusedPlaceholderVariant,
  // NOTHING SHOULD GO BELOW HERE
}

impl TryFrom<u8> for MessageType {
  type Error = String;

  fn try_from(value: u8) -> std::result::Result<Self, <MessageType as TryFrom<u8>>::Error> {
    // TODO: change to the following line when `variant_count` stabilizes
    // (https://github.com/rust-lang/rust/issues/73662) and remove `_UnusedPlaceholderVariant`
    //
    // if (1..=std::mem::variant_count::<MessageType>()) {
    if (1..(MessageType::_UnusedPlaceholderVariant as u8)).contains(&value) {
      Ok(unsafe { std::mem::transmute::<u8, MessageType>(value) })
    } else {
      Err(format!("Invalid message type: {value}"))
    }
  }
}
