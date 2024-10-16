use std::collections::HashMap;

use ipc_handler::IPCHandler;
use napi::{
  bindgen_prelude::{Function, Result},
  Error,
};

use crate::mmap::MmapIPC;
use crate::socket_line::SocketLineIPC;

#[macro_use]
extern crate napi_derive;

mod ipc_handler;
mod mmap;
mod socket_line;

/// A synchronous RPC channel that allows JavaScript to synchronously call out
/// to a child process and get a response over a simple tab-delimited protocol,
/// including handling of JavaScript-side callbacks before the call completes.
/// For details on the protocol, see the `requestSync` method.
#[napi]
pub struct SyncRpcChannel {
  ipc: Box<dyn IPCHandler>,
  callbacks: HashMap<String, Function<'static, (String, String), String>>,
}

#[napi]
impl SyncRpcChannel {
  #[napi(factory)]
  pub fn with_line_protocol(exe: String, args: Vec<String>) -> Result<Self> {
    Ok(Self {
      ipc: Box::new(SocketLineIPC::new(exe, args)?),
      callbacks: HashMap::new(),
    })
  }

  #[napi(factory)]
  pub fn with_mmap_protocol(exe: String, args: Vec<String>) -> Result<Self> {
    Ok(Self {
      ipc: Box::new(MmapIPC::new(exe, args)?),
      callbacks: HashMap::new(),
    })
  }

  /// Send a request to the child process and wait for a response. The method
  /// will not return, synchronously, until a response is received or an error
  /// occurs.
  ///
  /// Requests follow a simple line-based protocol that communicates with the
  /// child process through the child's stdin and stdout streams.
  ///
  /// All payloads are expected to be pre-encoded `"`-delimited JSON strings
  /// on either end--this API does not do any of its own JSON
  /// encoding/decoding itself.
  ///
  /// #### Protocol
  ///
  /// The child should handle the following messages through its stdin:
  ///
  /// * `request\t<method>\t<payload>\n`: a request to the child with the
  ///   given JSON `<payload>`, with `<method>` as the method name. The child
  ///   should send back any number of `call` messages and close the request
  ///   with either a `response` or `error` message.
  /// * `call-response\t<name>\t<payload>\n`: a response to a `call` message
  ///   that the child previously sent. The `<payload>` is the encoded result
  ///   from invoking the JavaScript callback associated with it. If the
  ///   callback errors
  /// * `call-error\t<name>\t<message>\n`: informs the child that an error
  ///   occurred. The `<message>` will simply be the stringified error, not
  ///   necessarily in JSON format. This method will also throw an error after
  ///   sending this message to its child and terminate the request call.
  ///
  /// The channel handles the following messages from the child's stdout:
  ///
  /// * `response\t<method>\t<payload>\n`: a response to a request that the
  ///   call was for. `<payload>` will be the call's return value, and should
  ///   be a JSON-encoded string. `<method>` MUST match the `request`
  ///   message's `<method>` argument.
  /// * `error\t<method>\t<message>\n`: a response that denotes some error
  ///   occurred while processing the request on the child side. The
  ///   `<message>` will be the stringified error, not necessarily in JSON
  ///   format. It will be used as the error message that this method will
  ///   throw (terminating the request). `<method>` MUST match the `request`
  ///   message's `<method>` argument.
  /// * `call\t<name>\t<payload>\n`: a request to invoke a pre-registered
  ///   JavaScript callback (see `registerCallback`). `<name>` is the name of
  ///   the callback, and `<payload>` is the JSON-encoded string that the
  ///   callback will be called with. The child should then listen for
  ///   `call-response` and `call-error` messages.
  #[napi]
  pub fn request_sync(&mut self, method: String, payload: String) -> Result<String> {
    self.ipc.write_message("request", &method, &payload)?;
    while let Some(line) = self.ipc.read_message() {
      let line = line?;
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
          if name == method {
            return Err(Error::from_reason(payload));
          } else {
            return Err(Error::from_reason(format!(
              "name mismatch for response: expected `{method}`, got `{name}`"
            )));
          }
        }
        "call" => {
          if let Some(cb) = self.callbacks.get(name) {
            match cb.call((name.into(), payload.into())) {
              Ok(res) => {
                self.ipc.write_message("call-response", name, res.trim())?;
              }
              Err(e) => {
                self
                  .ipc
                  .write_message("call-error", name, format!("{e}").trim())?;
                return Err(Error::from_reason(format!(
                  "Error calling callback `{name}`: {}",
                  e
                )));
              }
            }
          } else {
            self.ipc.write_message("call-error", name, &format!("unknown callback: `{name}`. Please make sure to register it on the JavaScript side before invoking it."))?;
            return Err(Error::from_reason(format!(
              "no callback named `{name}` found"
            )));
          }
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
  ) {
    self.callbacks.insert(name, cb);
  }

  /// Does what it says on the tin. But you wouldn't do this to a _child_,
  /// would you? Just what kind of person are you?
  #[napi]
  pub fn murder_in_cold_blood(&mut self) -> Result<()> {
    self.ipc.close()?;
    Ok(())
  }
}
