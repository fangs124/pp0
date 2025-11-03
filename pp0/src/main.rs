use std::{
    fs::File,
    io::{BufReader, Read},
};

use nnue::Network;
use pp0::uci_loop;

fn main() -> std::io::Result<()> {
    let file = File::open("net.nnue")?;
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents)?;
    let mut net: Network = serde_json::from_str(&contents).unwrap();
    uci_loop(&mut net)?;

    Ok(())
}
