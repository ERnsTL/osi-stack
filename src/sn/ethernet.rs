use std::io::Write;

use advmac::MacAddr6;
use afpacket::sync::RawPacketStream;
use etherparse::Ethernet2Header;

use crate::n;

use super::{SubnetworkService, Qos};

pub struct Service {
    socket: RawPacketStream,
    buffer: [u8; 1500],
}

impl SubnetworkService for Service {
    fn new(socket: RawPacketStream) -> Self {
        Service {
            socket: socket,
            buffer: [0u8; 1500],
        }
    }

    fn sn_unitdata_request(
        &mut self,
        sn_source_address: MacAddr6,
        sn_destination_address: MacAddr6,
        sn_quality_of_service: Qos,
        sn_userdata: &n::clnp::Pdu,  //TODO not perfectly abstracted, should be &[u8], but why not write directly into lower layer's buffer?
    ) {
        // send SNSDU (Ethernet frame)
        //TODO optimize - here an Ethernet2 header is allocated, which copies the values from sn_* - better something which borrows the values
        let pkt_out = Ethernet2Header{
            destination: sn_destination_address.to_array(),
            source: sn_source_address.to_array(),
            ether_type: crate::sn::ETHER_TYPE_CLNP,
        };
        println!("writing SNSDU...");
        let mut remainder = pkt_out.write_to_slice(&mut self.buffer).expect("failed writing SNSDU into buffer");
        //pkt_out.write(&mut self.socket).expect("failed writing frame into socket");
        //TODO optimize is ^ cheaper or below's sn_userdata pdu.into_buf() ?

        // send NPDU (CLNP PDU)
        println!("writing NPDU...");
        let bytes = sn_userdata.into_buf(&mut remainder);
        self.socket.write(&self.buffer[0..bytes + 14]).expect("could not write buffer into socket");    //TODO +14 is not cleanly abtracted

        println!("flushing DL...");
        self.socket.flush().expect("failed to flush socket");
    }

    fn flush(&mut self) {
        self.socket.flush().expect("failed to flush my own socket!");
    }
}