use std::io;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for line in io::stdin().lines() {
        let line = line?;
        let parts: Vec<&str> = line.split('\t').collect();
        match &parts[..] {
            ["request", "echo", payload] => {
                // Just echo it
                println!("response\techo\t{}", payload);
            }
            msg => {
                panic!("Unexpected message : {:?}", msg);
            }
        }
    }
    Ok(())
}