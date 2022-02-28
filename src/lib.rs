
pub mod parse;

use std::net::TcpStream;
use tungstenite::stream::MaybeTlsStream;
use ipnet::IpNet;
use std::net::IpAddr;
use url::Url;
use itertools::Itertools;

use crate::parse::{ RISMessage, RISMessageType };

/// RIS Live websocket endpoint.
pub const RIS_URL: &'static str = {
    "ws://ris-live.ripe.net/v1/ws/?client=ris-rs"
};

/// Representing an active RIS Live session.
pub struct RISLiveSession {
    sock: tungstenite::protocol::WebSocket<MaybeTlsStream<TcpStream>>,
}
impl RISLiveSession {
    /// Create a new session
    pub fn new() -> Self {
        let (sock, _) = tungstenite::connect(Url::parse(RIS_URL).unwrap())
            .expect("Can't connect to RIS Live");
        Self { sock }
    }

    /// Close this session
    pub fn close(&mut self) {
        self.sock.close(None).unwrap();
    }

    /// Subscribe to all withdrawal messages.
    pub fn subscribe_to_withdrawals(&mut self) {
        self.sock.write_message(tungstenite::Message::Text(
            serde_json::json!({
                "type": "ris_subscribe", 
                "data": { "require": "withdrawals" },
            }).to_string()
        )).unwrap()
    }

    /// Given a list of AS numbers, subscribe to updates where each ASN
    /// is present in the path.
    pub fn subscribe_asn_list(&mut self, path_list: &[u32]) {
        for asn in path_list {
            self.sock.write_message(tungstenite::Message::Text(
                serde_json::json!({
                    "type": "ris_subscribe", 
                    "data": { "path": format!("{}", asn) }
                }).to_string()
            )).unwrap()
        }
    }

    pub fn read_msg(&mut self) -> Option<String> {
        let msg = self.sock.read_message()
            .expect("Couldn't read from socket");
        if let tungstenite::Message::Text(s) = msg { 
            Some(s) 
        } else { None }
    }
}
impl std::ops::Drop for RISLiveSession {
    fn drop(&mut self) {
        self.close();
    }
}

#[derive(Debug)]
pub struct AnnouncementVector {
    pub next_hop: IpAddr,
    pub prefixes: Vec<IpNet>,
}

#[derive(Debug)]
pub enum BGPUpdateType {
    Announce { path: Vec<u32>, vectors: Vec<AnnouncementVector> },
    Withdraw { prefixes: Vec<IpNet> },
}

/// Representing a BGP update message.
#[derive(Debug)]
pub struct BGPUpdate {
    timestamp: f64,
    asn: u32,
    kind: BGPUpdateType,
}
impl BGPUpdate {
    pub fn from_message(msg: &RISMessage) -> Option<Vec<Self>> {
        if let RISMessageType::UPDATE { announce, withdrawals } = &msg.ty {
            let mut res = Vec::new();
            if let Some(p) = &announce.path {
                let a = &announce.announcements.as_ref().unwrap();
                let data = BGPUpdateType::Announce {
                    path: p.to_owned(),
                    vectors: a.iter().map(|e| {
                        AnnouncementVector {
                            next_hop: e.next_hop.parse().unwrap(),
                            prefixes: e.prefixes.iter().map(|s| { 
                                s.parse().unwrap() }).collect(),
                        }
                    }).collect(),
                };
                res.push(BGPUpdate {
                    timestamp: msg.timestamp,
                    asn: u32::from_str_radix(&msg.peer_asn, 10).unwrap(),
                    kind: BGPUpdateType::Announce {
                        path: p.to_owned(),
                        vectors: a.iter().map(|e| {
                            AnnouncementVector {
                                next_hop: e.next_hop.parse().unwrap(),
                                prefixes: e.prefixes.iter().map(|s| {
                                    s.parse().unwrap() }).collect(),
                            }
                        }).collect(),
                    },
                });
            }
            if let Some(w) = &withdrawals {
                res.push(BGPUpdate {
                    timestamp: msg.timestamp,
                    asn: u32::from_str_radix(&msg.peer_asn, 10).unwrap(),
                    kind: BGPUpdateType::Withdraw {
                        prefixes: w.iter().map(|s| { s.parse().unwrap() } )
                            .collect(),
                    }
                });
            }
            return Some(res);
        }
        None
    }
}

impl std::fmt::Display for BGPUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            BGPUpdateType::Announce { path, vectors } => {
                for v in vectors {
                    for p in &v.prefixes {
                        let x = format!("{:>6}|A {:<20}|{}", 
                            self.asn.to_string(), p.to_string(), 
                            path.iter().map(|x|
                                format!("{:<6}", x.to_string())).join(" ")
                        );
                        writeln!(f, "{}", x);
                    }
                }
            },
            BGPUpdateType::Withdraw { prefixes } => {
                for prefix in prefixes {
                    let x = format!("{:>6}|W {:<20}", 
                        self.asn.to_string(), prefix.to_string()
                    );
                    writeln!(f, "{}", x);
                }
            },
        }
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        use std::io::BufRead;
        use crate::parse::*;
        use crate::*;

        let f = std::fs::File::open("test-json.txt").unwrap();
        let f = std::io::BufReader::new(f);
        let mut res: Vec<BGPUpdate> = Vec::new();
        for line in f.lines() {
            let l = line.unwrap();
            let x: RISPacket = serde_json::from_str(&l).unwrap();
            if let RISPacket::Message(m) = &x {
                if let Some(mut p) = BGPUpdate::from_message(&m) {
                    res.append(&mut p);
                }
            }
        }
        for update in &res {
            print!("{}", update);
        }
    }
}


