use std::{
    env,
    fs::File,
    io::{BufReader, Read},
};

use nnue::Network;
use pp0::{uci_bench, uci_loop};
const WEIGHT: &str = include_str!("../../net.nnue");
fn main() -> std::io::Result<()> {
    //let file = File::open("net.nnue")?;
    //let mut buf_reader = BufReader::new(file);
    //let mut contents = String::new();
    //buf_reader.read_to_string(&mut contents)?;
    let mut net: Network = serde_json::from_str(&WEIGHT).unwrap();
    let args: Vec<String> = env::args().collect();
    if args.len() > 0 {
        for arg in &args {
            if arg == "bench" {
                uci_bench(&mut net);
                return Ok(());
            }
        }
    }
    uci_loop(&mut net)?;

    Ok(())
}
