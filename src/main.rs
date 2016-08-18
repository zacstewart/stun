extern crate stun;

use stun::{Client, Message};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let local_port = args[1].clone().parse::<u16>().unwrap();
    let client = Client::new("stun.l.google.com:19302", local_port);
    let mesage = Message::request();
    let encoded = mesage.encode();
    let response = client.send(encoded.clone());
    let response = Message::decode(response);
    println!("response: {:?}", response);
}
