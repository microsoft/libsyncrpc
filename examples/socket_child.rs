use std::io::{self, BufRead, BufReader, Lines, Stdin, Write, stdout};

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
            ["request-bin", "binary", _] => {
                // Create a realistic binary payload with 32-bit integers
                // One of the integers will have bytes that include a newline character (0x0A)
                
                // We'll create a small binary array of 5 32-bit integers
                // [1, 10, 266, 1000000, 2147483647]
                // 10 decimal = 0x0000000A which includes a byte that's the newline character
                // 266 decimal = 0x0000010A which includes a byte that's the newline character
                let mut binary_data = Vec::new();
                
                // Add integer 1 (0x00000001)
                binary_data.extend_from_slice(&1_u32.to_le_bytes());
                
                // Add integer 10 (0x0000000A) - contains a newline byte
                binary_data.extend_from_slice(&10_u32.to_le_bytes());
                
                // Add integer 266 (0x0000010A) - contains a newline byte
                binary_data.extend_from_slice(&266_u32.to_le_bytes());
                
                // Add integer 1000000 (0x000F4240)
                binary_data.extend_from_slice(&1000000_u32.to_le_bytes());
                
                // Add integer 2147483647 (0x7FFFFFFF) - max positive 32-bit signed integer
                binary_data.extend_from_slice(&2147483647_u32.to_le_bytes());
                
                // Calculate the size of the binary data
                let size = binary_data.len();
                
                // Write the binary response header
                let header = format!("response-bin\tbinary\t{}\n", size);
                let mut stdout = stdout();
                stdout.write_all(header.as_bytes())?;
                
                // Write the raw binary data directly to stdout
                stdout.write_all(&binary_data)?;
                
                // Add the final newline
                stdout.write_all(b"\n")?;
                stdout.flush()?;
            }
            ["request-bin", "binary-with-callback", count_str] => {
                // Parse the count for sequence length
                let count = count_str.parse::<u32>().unwrap_or(5);
                
                // Get a value from JavaScript callback to include in our binary response
                let suffix_value_str = call(&mut lines, "getSuffix", "")?;
                let suffix_value = suffix_value_str.parse::<u32>().unwrap_or(0);
                
                // Create binary data with integers 1 to count followed by the suffix value
                let mut binary_data = Vec::new();
                
                // Add the sequence of integers
                for i in 1..=count {
                    binary_data.extend_from_slice(&i.to_le_bytes());
                }
                
                // Add the suffix value from the callback
                binary_data.extend_from_slice(&suffix_value.to_le_bytes());
                
                // Calculate the size of the binary data
                let size = binary_data.len();
                
                // Write the binary response header
                let header = format!("response-bin\tbinary-with-callback\t{}\n", size);
                let mut stdout = stdout();
                stdout.write_all(header.as_bytes())?;
                
                // Write the raw binary data directly to stdout
                stdout.write_all(&binary_data)?;
                
                // Add the final newline
                stdout.write_all(b"\n")?;
                stdout.flush()?;
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
            ["request", "error", _] => {
                println!("error\terror\t\"something went wrong\"");
            }
            ["request", "throw", _] => {
                println!("call\tthrow\t\"\"");
                let response = lines.next().expect("no response?")?;
                let parts: Vec<&str> = response.split('\t').collect();
                let ["call-error", "throw", _] = parts.as_slice() else {
                    panic!("Unexpected response : {:?}", parts);
                };
                // Do nothing
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