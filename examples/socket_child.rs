use std::io::{self, BufReader, BufWriter, Stdin, Stdout};

use libsyncrpc_connection::RpcConnection;

#[allow(non_snake_case)]
mod MessageType {
  #[allow(non_upper_case_globals)]
  pub const Request: u8 = 1;
  #[allow(non_upper_case_globals)]
  pub const CallResponse: u8 = 2;
  #[allow(non_upper_case_globals)]
  pub const CallError: u8 = 3;
  #[allow(non_upper_case_globals)]
  pub const Response: u8 = 4;
  #[allow(non_upper_case_globals)]
  pub const Error: u8 = 5;
  #[allow(non_upper_case_globals)]
  pub const Call: u8 = 6;
}

static BIG_ARR: [u8; 1024 * 1024] = [0; 1024 * 1024];

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut conn = RpcConnection::new(BufReader::new(io::stdin()), BufWriter::new(io::stdout()))?;
  // eprintln!("Child initialized?");
  loop {
    let (ty, name, payload) = conn.read()?;
    match (ty, &name[..], &payload[..]) {
      (MessageType::Request, b"echo", payload) => {
        // Just echo it
        conn.write(MessageType::Response, b"echo", payload)?;
      }
      (MessageType::Request, b"callback-echo", payload) => {
        let res_payload = call(&mut conn, b"echo", payload)?;
        conn.write(MessageType::Response, b"callback-echo", &res_payload)?;
      }
      (MessageType::Request, b"binary", _) => {
        conn.write(MessageType::Response, b"binary", &BIG_ARR)?;
      }
      (MessageType::Request, b"empty", _) => {
        conn.write(MessageType::Response, b"empty", b"")?;
      }
      (MessageType::Request, b"concat", _) => {
        let one = call(&mut conn, b"one", b"1")?;
        let two = call(&mut conn, b"two", b"2")?;
        let three = call(&mut conn, b"three", b"3")?;
        conn.write(
          MessageType::Response,
          b"concat",
          &[one, two, three].concat(),
        )?;
      }
      (MessageType::Request, b"error", _) => {
        conn.write(MessageType::Error, b"error", b"\"something went wrong\"")?;
      }
      (MessageType::Request, b"throw", _) => {
        conn.write(MessageType::Call, b"throw", b"")?;
        let (ty, name, _) = conn.read()?;
        if ty != MessageType::CallError || &name != b"throw" {
          panic!("Unexpected response : {:?}\\t{:?}\\t...", ty, name);
        }
        // Do nothing
      }
      (ty, name, _) => {
        panic!(
          "Unexpected message : ({ty}) {}",
          String::from_utf8_lossy(name)
        );
      }
    }
  }
}

fn call(
  conn: &mut RpcConnection<BufReader<Stdin>, BufWriter<Stdout>>,
  name: &[u8],
  payload: &[u8],
) -> io::Result<Vec<u8>> {
  conn.write(MessageType::Call, name, payload)?;
  let (res_ty, res_name, res_payload) = conn.read()?;
  if res_ty != MessageType::CallResponse {
    panic!("Expected a CallResponse but got {res_ty}");
  }
  if res_name != name {
    panic!(
      "Unexpected response name : {}",
      String::from_utf8_lossy(&res_name)
    );
  }
  Ok(res_payload)
}
