use std::io::{self, BufReader, BufWriter, Stdin, Stdout};

use libsyncrpc_connection::RpcConnection;

static BIG_ARR: [u8; 1024 * 1024] = [0; 1024 * 1024];

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut conn = RpcConnection::new(BufReader::new(io::stdin()), BufWriter::new(io::stdout()))?;
  // eprintln!("Child initialized?");
  while let Some((ty, name, payload)) = conn.read()? {
    match [&ty[..], &name[..], &payload[..]] {
      [b"request", b"echo", payload] => {
        // Just echo it
        conn.write(b"response", b"echo", payload)?;
      }
      [b"request", b"callback-echo", payload] => {
        let res_payload = call(&mut conn, b"echo", payload)?;
        conn.write(b"response", b"callback-echo", &res_payload)?;
      }
      [b"request", b"binary", _] => {
        conn.write(b"response", b"binary", &BIG_ARR)?;
      }
      [b"request", b"empty", _] => {
        conn.write(b"response", b"empty", b"")?;
      }
      [b"request", b"concat", _] => {
        let one = call(&mut conn, b"one", b"1")?;
        let two = call(&mut conn, b"two", b"2")?;
        let three = call(&mut conn, b"three", b"3")?;
        conn.write(b"response", b"concat", &[one, two, three].concat())?;
      }
      [b"request", b"error", _] => {
        conn.write(b"error", b"error", b"\"something went wrong\"")?;
      }
      [b"request", b"throw", _] => {
        conn.write(b"call", b"throw", b"")?;
        let (ty, name, _) = conn.read()?.expect("No response?");
        if &ty != b"call-error" || &name != b"throw" {
          panic!("Unexpected response : {:?}\\t{:?}\\t...", ty, name);
        }
        // Do nothing
      }
      [b"mmap", b"resize", _payload] => {
        // eprintln!("wooooo. Stuff!");
        #[cfg(feature = "mmap")]
        {
          let (size, _) = _payload.split_at(size_of::<usize>());
          let size = usize::from_le_bytes(size.try_into().expect("Bad mmap size bytes."));
          // eprintln!("Child got resize request. New size: {size}.");
          conn.resize_mmap_ack(size)?;
        }
      }
      [ty, name, _] => {
        panic!(
          "Unexpected message : {}\\t{}",
          String::from_utf8_lossy(ty),
          String::from_utf8_lossy(name)
        );
      }
    }
  }
  Ok(())
}

fn call(
  conn: &mut RpcConnection<BufReader<Stdin>, BufWriter<Stdout>>,
  name: &[u8],
  payload: &[u8],
) -> io::Result<Vec<u8>> {
  conn.write(b"call", name, payload)?;
  let (res_ty, res_name, res_payload) = conn.read()?.expect("Child expected a response but socket to parent closed.");
  if res_ty != b"call-response" {
    panic!(
      "Expected a call-response but got {}",
      String::from_utf8_lossy(&res_ty)
    );
  }
  if res_name != name {
    panic!(
      "Unexpected response name : {}",
      String::from_utf8_lossy(&res_name)
    );
  }
  Ok(res_payload)
}
