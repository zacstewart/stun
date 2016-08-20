use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs, UdpSocket};
use std::mem;

const MAGIC_COOKIE: [u8; 4] = [0x21, 0x12, 0xA4, 0x42];

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MessageClass {
    Request =         0b00000000000000,
    Indication =      0b00000000010000,
    SuccessResponse = 0b00000100000000,
    FailureResponse = 0b00000100010000
}

impl MessageClass {
    fn from_u16(num: u16) -> Option<MessageClass> {
        match num {
            0b00000000000000 => Some(MessageClass::Request),
            0b00000000010000 => Some(MessageClass::Indication),
            0b00000100000000 => Some(MessageClass::SuccessResponse),
            0b00000100010000 => Some(MessageClass::FailureResponse),
            _                => None
        }
    }
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MessageMethod {
    Binding = 0b00000000000001
}

#[derive(Debug)]
struct Header {
    class: MessageClass,
    method: MessageMethod
}

impl Header {
    pub fn decode(encoded: &[u8]) -> Header {
        let message_type = ((encoded[0] as u16) << 8) | (encoded[1] as u16);
        let class = MessageClass::from_u16(message_type & 0b00000100010000).unwrap();
        Header {
            class: class,
            method: MessageMethod::Binding
        }
    }

    fn encode(&self) -> Vec<u8> {
        let message_type: [u8; 2] = unsafe { mem::transmute(self.message_type().swap_bytes()) };
        let message_length: [u8; 2] = unsafe { mem::transmute(self.message_length().swap_bytes()) };
        let transaction_id: [u8; 12] = [7; 12];

        let mut bytes = vec![];
        bytes.extend(&message_type);
        bytes.extend(&message_length);
        bytes.extend(&MAGIC_COOKIE);
        bytes.extend(&transaction_id);
        bytes
    }

    fn message_length(&self) -> u16 {
        0
    }

    fn message_type(&self) -> u16 {
        (self.class as u16) | (self.method as u16)
    }
}

#[derive(Debug)]
pub struct XorMappedAddress(pub SocketAddr);

impl XorMappedAddress {
    fn decode(encoded: Vec<u8>) -> Result<XorMappedAddress, String> {
        let port = (((encoded[2] as u16) << 8) | (encoded[3] as u16)) ^ 0x2112;
        let encoded_ip = &encoded[4..];
        let ip = match encoded[1] {
            1 => {
                let octets: Vec<u8> = encoded_ip.iter()
                    .zip(&MAGIC_COOKIE)
                    .map(|(b,m)| b ^ m)
                    .collect();
                IpAddr::V4(Ipv4Addr::new(octets[0], octets[1], octets[2], octets[3]))
            }
            2 => {
                IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0))
            },
            e @ _ => { return Err(format!("Invalid address family: {:?}", e)) }
        };

        let address = SocketAddr::new(ip, port);

        Ok(XorMappedAddress(address))
    }
}
#[derive(Debug)]
pub enum Attribute {
    MappedAddress,
    Username,
    MessageIntegrity,
    ErrorCode,
    UnknownAttributes,
    Realm,
    Nonce,
    XorMappedAddress(XorMappedAddress)
}

impl Attribute {
    fn decode_all(encoded: &[u8]) -> Result<Vec<Attribute>, String> {
        let mut encoded = encoded.to_vec();
        let mut attributes = vec![];

        while !encoded.is_empty() {
            let attribute_type = ((encoded.remove(0) as u16) << 8) | (encoded.remove(0) as u16);
            let length = ((encoded.remove(0) as usize) << 8) | (encoded.remove(0) as usize);
            let value = encoded.drain(..length).collect();
            let attribute = match attribute_type {
                0x0001 => Attribute::decode_mapped_address(value),
                0x0006 => Attribute::decode_username(value),
                0x0008 => Attribute::decode_message_integrity(value),
                0x0009 => Attribute::decode_error_code(value),
                0x000A => Attribute::decode_unknown_attributes(value),
                0x0014 => Attribute::decode_realm(value),
                0x0015 => Attribute::decode_nonce(value),
                0x0020 => Attribute::decode_xor_mapped_address(value),
                _ => { Err(format!("Unknown attribute type: 0x{:x}", attribute_type)) }
            };

            match attribute {
                Ok(attr) => attributes.push(attr),
                Err(error) => println!("{}", error)
            }
        }

        Ok(attributes)
    }

    fn decode_mapped_address(_value: Vec<u8>) -> Result<Attribute, String> {
        Ok(Attribute::MappedAddress)
    }

    fn decode_username(_value: Vec<u8>) -> Result<Attribute, String> {
        Ok(Attribute::Username)
    }

    fn decode_message_integrity(_value: Vec<u8>) -> Result<Attribute, String> {
        Ok(Attribute::MessageIntegrity)
    }

    fn decode_error_code(_value: Vec<u8>) -> Result<Attribute, String> {
        Ok(Attribute::ErrorCode)
    }

    fn decode_unknown_attributes(_value: Vec<u8>) -> Result<Attribute, String> {
        Ok(Attribute::UnknownAttributes)
    }

    fn decode_realm(_value: Vec<u8>) -> Result<Attribute, String> {
        Ok(Attribute::Realm)
    }

    fn decode_nonce(_value: Vec<u8>) -> Result<Attribute, String> {
        Ok(Attribute::Nonce)
    }

    fn decode_xor_mapped_address(value: Vec<u8>) -> Result<Attribute, String> {
        XorMappedAddress::decode(value).map(|a| Attribute::XorMappedAddress(a))
    }
}

#[derive(Debug)]
pub struct Message {
    header: Header,
    pub attributes: Vec<Attribute>
}

impl Message {
    pub fn request() -> Message {
        let header = Header {
            class: MessageClass::Request,
            method: MessageMethod::Binding
        };
        Message {
            header: header,
            attributes: vec![]
        }
    }

    pub fn decode(encoded: Vec<u8>) -> Message {
        let header = Header::decode(&encoded[..20]);
        let attributes = Attribute::decode_all(&encoded[20..]).unwrap();
        Message {
            header: header,
            attributes: attributes
        }
    }
    pub fn encode(&self) -> Vec<u8> {
        self.header.encode()
    }
}

pub struct Client<T: ToSocketAddrs> {
    server: T,
    socket: UdpSocket
}

impl<T: ToSocketAddrs + Copy> Client<T> {
    pub fn new(server_address: T, local_port: u16) -> Client<T> {
        Client {
            server: server_address,
            socket: UdpSocket::bind(("0.0.0.0", local_port)).unwrap()
        }
    }

    pub fn send(&self, message: Vec<u8>) -> Vec<u8> {
        self.socket.send_to(message.as_slice(), self.server).unwrap();
        let mut buf = [0; 512];
        let (amt, _) = self.socket.recv_from(&mut buf).unwrap();
        buf[..amt].to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_message() {
        let encoded = Message::request().encode();

        let expected = vec![0, 1, 0, 0, 33, 18, 164, 66, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7];
        assert_eq!(encoded, expected);
    }

    #[test]
    fn test_decode_message() {
        let encoded: Vec<u8> = vec![
            1, 1, 0, 12, 33, 18, 164, 66, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
            0, 32, 0, 8, 0, 1, 183, 220, 67, 210, 130, 201];
        let message = Message::decode(encoded);
        assert_eq!(message.header.method, MessageMethod::Binding);
        assert_eq!(message.header.class, MessageClass::SuccessResponse);
        assert_eq!(message.attributes.len(), 1);
    }

    #[test]
    fn test_decode_xor_mapped_address() {
        use std::net::{IpAddr, Ipv4Addr};

        let encoded = vec![0, 1, 59, 25, 67, 210, 130, 201];
        let XorMappedAddress(address) = XorMappedAddress::decode(encoded).unwrap();

        assert_eq!(address.port(), 6667);
        assert_eq!(address.ip(), IpAddr::V4(Ipv4Addr::new(98, 192, 38, 139)));
    }
}
