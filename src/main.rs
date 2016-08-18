extern crate stun;

use stun::{Client, Message};

fn main() {
    let client = Client::new("stun.l.google.com:19302", 6667);
    let mesage = Message::request();
    let encoded = mesage.encode();
    let response = client.send(encoded.clone());
    let response = Message::decode(response);
    println!("response: {:?}", response);
}
