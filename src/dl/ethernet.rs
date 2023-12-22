use std::{io::{Write, Read}, thread};

use advmac::MacAddr6;
use afpacket::sync::RawPacketStream;
use etherparse::{Ethernet2Header, ether_type, SingleVlanHeaderSlice};

use crate::n;
use crate::n::NetworkService;

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
        sn_userdata: &mut n::clnp::Pdu,  //TODO not perfectly abstracted, should be &[u8], but why not write directly into lower layer's buffer?
    ) {
        // send SNSDU (Ethernet frame)
        //TODO optimize - here an Ethernet2 header is allocated, which copies the values from sn_* - better something which borrows the values
        let pkt_out = Ethernet2Header{
            destination: sn_destination_address.to_array(),
            source: sn_source_address.to_array(),
            ether_type: crate::dl::ETHER_TYPE_CLNP,
        };
        //println!("writing SNSDU...");
        let mut remainder = pkt_out.write_to_slice(&mut self.buffer).expect("failed writing SNSDU into buffer");
        //pkt_out.write(&mut self.socket).expect("failed writing frame into socket");
        //TODO optimize is ^ cheaper or below's sn_userdata pdu.into_buf() ?

        // send NPDU (CLNP PDU)
        //println!("writing NPDU...");
        let bytes = sn_userdata.into_buf(true, &mut remainder);
        self.socket.write(&self.buffer[0..bytes + 14]).expect("could not write buffer into socket");    //TODO +14 is not cleanly abtracted //TODO handle network down - dont crash, but try again

        //println!("flushing DL...");
        self.socket.flush().expect("failed to flush socket");
    }

    fn flush(&mut self) {
        self.socket.flush().expect("failed to flush my own socket!");
    }

    fn sn_unitdata_indication(
        sn_source_address: MacAddr6,
        sn_destination_address: MacAddr6,
        sn_quality_of_service: &Qos,
        sn_userdata: &[u8]
    ) {
        let n_quality_of_service = n::Qos{}; //TODO from sn_quality_of_service
        //TODO the source and destination addresses should probably also be converted to NSAPs for the N layer protocol

        // forward up from DL to N layer
        //TODO this method will need &mut self at some point, but this will create 2 borrows - one for read and one for write
        //TODO must enable 2 threads working inside NClnpService.
        //TODO modify to have NClnpService .read and .write inner parts - only these get borrowed. And these 2 only lock the shared host lists etc. when really needed.
        n::clnp::Service::n_unitdata_indication(
            sn_source_address,
            sn_destination_address,
            &n_quality_of_service,
            sn_userdata
        );
    }

    // read SN-UNITDATA Indications from the socket
    fn run(&mut self) {
        let mut socket = self.socket.clone();
        let _ = thread::spawn(move || {
            let qos = Qos{};
            loop {
                let mut buffer = [0u8; 1500];
                println!("reading frame...");
                let num_bytes = socket.read(&mut buffer).expect("could not read DL frame from socket into buffer"); //TODO handle network down - dont crash, but try again

                // hand-cooked version, because we dont care about getting IP and TCP/UDP parsed
                let eth_header = etherparse::Ethernet2HeaderSlice::from_slice(&buffer).expect("could not parse Ethernet2 header");
                println!("destination: {:x?}  source: {:x?}  ethertype: 0x{:04x}", eth_header.destination(), eth_header.source(), eth_header.ether_type());
                let mut vlan_len: usize = 0;
                match eth_header.ether_type() {
                    ether_type::VLAN_TAGGED_FRAME | ether_type::PROVIDER_BRIDGING | ether_type::VLAN_DOUBLE_TAGGED_FRAME => {
                        let vlan_header = SingleVlanHeaderSlice::from_slice(&buffer[eth_header.slice().len()-1..buffer.len()-1]).expect("could not parse single VLAN header");
                        println!("vlan: {:?}", vlan_header);
                        vlan_len = vlan_header.slice().len();
                        //TODO handle what comes after vlan
                    },
                    ether_type::IPV6 => { println!("{}", "got ipv6, ignoring"); }
                    ether_type::IPV4 => { println!("{}", "got ipv4, ignoring"); }
                    ETHER_TYPE_CLNP => { println!("ah, got CLNP - feel warmly welcome!"); } //TODO optimize - does the order of match legs affect performance?
                    _ => { println!("{}", "got unknown, discarding"); }
                }

                // send up the stack to Subnetwork Service as SN-UNITDATA Indication
                Self::sn_unitdata_indication(
                    MacAddr6::from(eth_header.source()),
                    MacAddr6::from(eth_header.destination()),
                    &qos,
                    &buffer[0+eth_header.slice().len() .. num_bytes]    //TODO plus VLAN 802.11q (?) header, if present
                );
            }
        });
    }
}