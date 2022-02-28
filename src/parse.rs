
use serde::Deserialize;

/// Representing some RIS Live JSON packet.
#[derive(Debug, Deserialize)]
#[serde(tag="type", content="data")]
pub enum RISPacket {
    #[serde(rename="ris_message")]
    Message(RISMessage),
}

/// RIS Live JSON message format.
#[derive(Debug, Deserialize)]
#[serde(tag="type")]
pub struct RISMessage {
    pub timestamp: f64,
    pub peer: String,
    pub peer_asn: String,
    pub id: String,
    pub host: String,

    // NOTE: Flattening is kind of annoying, but it seems like it works?
    #[serde(rename="type", flatten)]
    pub ty: RISMessageType,
}

/// JSON format for different kinds of BGP messages.
///
/// NOTE: The prescence of the 'path' key always indicates announcments.
/// Withdrawals (if they are included) always come afterwards.
#[derive(Debug, Deserialize)]
#[serde(tag="type")]
pub enum RISMessageType {
    UPDATE {
        #[serde(rename="path", flatten)]
        announce: RISAnnouncement,
        withdrawals: Option<Vec<String>>,
    },
}

/// JSON format for a set of announcements.
#[derive(Debug, Deserialize)]
pub struct RISAnnouncement {
    pub path: Option<Vec<u32>>,
    pub community: Option<Vec<Vec<u32>>>,
    pub origin: Option<String>,
    pub announcements: Option<Vec<AnnouncementEntry>>,
}

/// JSON format for a particular announcement.
#[derive(Debug, Deserialize)]
pub struct AnnouncementEntry {
    pub next_hop: String,
    pub prefixes: Vec<String>,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        use std::io::BufRead;
        use crate::parse::*;
        let f = std::fs::File::open("test-json.txt").unwrap();
        let f = std::io::BufReader::new(f);
        for line in f.lines() {
            let l = line.unwrap();
            let x: RISPacket = serde_json::from_str(&l).unwrap();
            if let RISPacket::Message(m) = &x {
                if let RISMessageType::UPDATE { announce, withdrawals } = &m.ty {
                    if announce.path.is_none() {
                        assert!(announce.community.is_none());
                        assert!(announce.origin.is_none());
                        assert!(announce.announcements.is_none());
                    }
                }
            }
            //println!("{:#?}", x);
        }
    }
}
