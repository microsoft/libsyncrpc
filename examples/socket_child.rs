use std::io::{self, BufRead, BufReader, Lines, Stdin};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut lines = io::BufReader::new(io::stdin()).lines();
    while let Some(line) = lines.next() {
        let line = line?;
        let parts: Vec<&str> = line.split('\t').collect();
        match &parts[..] {
            ["request", "echo", payload] => {
                // Just echo it
                println!("response\techo\t{}", payload);
            }
            ["request", "callback-echo", payload] => {
                let res_payload = call(&mut lines, "echo", payload)?;
                println!("response\tcallback-echo\t{res_payload}");
            }
            ["request", "concat", _] => {
                let one = call(&mut lines, "one", "1")?;
                let two = call(&mut lines, "two", "2")?;
                let three = call(&mut lines, "three", "3")?;
                println!("response\tconcat\t\"{one}{two}{three}\"");
            }
            msg => {
                panic!("Unexpected message : {:?}", msg);
            }
        }
    }
    Ok(())
}

fn call(lines: &mut Lines<BufReader<Stdin>>, name: &str, payload: &str) -> io::Result<String> {
    println!("call\t{name}\t{payload}");
    let response = lines.next().expect("no response?")?;
    let parts: Vec<&str> = response.split('\t').collect();
    let ["call-response", res_name, res_payload] = parts.as_slice() else {
        panic!("Unexpected response : {:?}", parts);
    };
    if *res_name != name {
        panic!("Unexpected response name : {res_name}");
    }
    Ok(res_payload.to_string())
}