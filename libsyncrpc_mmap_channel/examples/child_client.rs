use libsyncrpc_mmap_channel::MmapIPCChannel;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let read_from = std::env::args_os().nth(1).expect("Expected read path as command line argument");
    let write_to = std::env::args_os().nth(2).expect("Expected write path as command line argument");
    let mut channel = MmapIPCChannel::new(&read_from, &write_to);
    channel.write("request\techo\t\"Hello, world!\"".into()).unwrap();
    loop {
        match &channel.read().unwrap().splitn(3, '\t').collect::<Vec<&str>>()[..] {
            ["init", ..] => {
                // Channel initialization message.
            }
            ["request", "echo", payload] => {
                // Just echo it
                channel.write(format!("response\techo\t{payload}"))?;
            }
            msg => {
                eprintln!("Unexpected message : {:?}", msg);
            }
        }
    }
}