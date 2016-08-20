extern crate stun;

use stun::{Client, IpVersion, Message};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let local_port = args[1].clone().parse::<u16>().unwrap();
    let server = args[2].clone();
    let ipv = args[3].clone();
    let ipv = match ipv.as_str() {
        "4" => Ok(IpVersion::V4),
        "6" => Ok(IpVersion::V6),
        e @ _ => Err(format!("Unknown IP version: {}", e))
    }.unwrap();

    let client = Client::new(server.as_str(), local_port, ipv);
    let mesage = Message::request();
    let encoded = mesage.encode();
    let response = client.send(encoded.clone());
    let response = Message::decode(response);
    println!("response: {:?}", response);
}
