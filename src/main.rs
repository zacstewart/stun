use stun::{Client, IpVersion, Message};

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
/// A small command line app to do stun-request
struct Opt {
    /// the port on this machine that you want to be reachable from the internet
    local_port: u16,
    /// the stun server to use, must include a port
    server: String,
    /// which ip-version to use, either 4 or 6
    #[structopt(parse(try_from_str = parse_ipver))]
    ip_version: IpVersion,
}

fn parse_ipver(src: &str) -> Result<IpVersion, String> {
    match src {
        "4" => Ok(IpVersion::V4),
        "6" => Ok(IpVersion::V6),
        e => Err(format!("Unknown IP version: {}", e)),
    }
}

fn main() {
    let opt = Opt::from_args();

    let client = Client::new(opt.server, opt.local_port, opt.ip_version);
    let mesage = Message::request();
    let encoded = mesage.encode();
    let response = client.send(encoded);
    let response = Message::decode(response);
    println!("response: {:?}", response);
}
