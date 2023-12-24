use std::{io::{Write, Read}, thread, cell::RefCell, sync::{Arc, Mutex}};

use advmac::MacAddr6;
use afpacket::sync::RawPacketStream;
use etherparse::{Ethernet2Header, ether_type, SingleVlanHeaderSlice};

use crate::n::{self, NUnitDataIndication};

use super::{SubnetworkService, Qos, SNUnitDataRequest};

pub struct Service {
    socket: RawPacketStream,
    buffer_in: Arc<Mutex<[u8; 1500]>>,  // from socket
    buffer_out: Arc<Mutex<[u8; 1500]>>, // out into socket

    n_service_from: Arc<Mutex<rtrb::Consumer<SNUnitDataRequest>>>,
    n_service_to: Arc<Mutex<rtrb::Producer<NUnitDataIndication>>>,
}

impl<'a> SubnetworkService<'a> for Service {
    fn new(
        socket: RawPacketStream,
        n_service_from: rtrb::Consumer<SNUnitDataRequest>,
        n_service_to: rtrb::Producer<NUnitDataIndication>
    ) -> Self {
        Service {
            socket: socket,
            buffer_in: Arc::new(Mutex::new([0u8; 1500])),
            buffer_out: Arc::new(Mutex::new([0u8; 1500])),
            n_service_from: Arc::new(Mutex::new(n_service_from)),
            n_service_to: Arc::new(Mutex::new(n_service_to)),
        }
    }

    fn sn_unitdata_request(
        &mut self,
        sn_source_address: MacAddr6,
        sn_destination_address: MacAddr6,
        sn_quality_of_service: Qos,
        mut sn_userdata: n::clnp::Pdu,  //TODO not perfectly abstracted, should be &[u8], but why not write directly into lower layer's buffer?
    ) {
        // send SNSDU (Ethernet frame)
        //TODO optimize - here an Ethernet2 header is allocated, which copies the values from sn_* - better something which borrows the values
        let pkt_out = Ethernet2Header{
            destination: sn_destination_address.to_array(),
            source: sn_source_address.to_array(),
            ether_type: crate::dl::ETHER_TYPE_CLNP,
        };
        //println!("writing SNSDU...");
        let mut buffer_out = self.buffer_out.lock().expect("failed to lock buffer");
        let mut remainder = pkt_out.write_to_slice(&mut *buffer_out).expect("failed writing SNSDU into buffer");
        //pkt_out.write(&mut self.socket).expect("failed writing frame into socket");
        //TODO optimize is ^ cheaper or below's sn_userdata pdu.into_buf() ?

        // send NPDU (CLNP PDU)
        //println!("writing NPDU...");
        let bytes = sn_userdata.into_buf(true, &mut remainder);
        self.socket.write(&buffer_out[0..bytes + 14]).expect("could not write buffer into socket");    //TODO +14 is not cleanly abtracted //TODO handle network down - dont crash, but try again

        //println!("flushing DL...");
        self.socket.flush().expect("failed to flush socket");
    }

    fn flush(&mut self) {
        self.socket.flush().expect("failed to flush my own socket!");
    }

    fn sn_unitdata_indication(
        n_service_to: &mut rtrb::Producer<NUnitDataIndication>, //TODO optimize clunky - &mut self would be nice but complains about 2 mutable borrows to self
        sn_source_address: MacAddr6,
        sn_destination_address: MacAddr6,
        sn_quality_of_service: Qos,
        sn_userdata: &'a [u8]
    ) {
        let n_quality_of_service = n::Qos{}; //TODO from sn_quality_of_service  //TODO optimize - new allocation on every call
        //TODO the source and destination addresses should probably also be converted to NSAPs for the N layer protocol

        // forward up from DL to N layer
        //TODO this method will need &mut self at some point, but this will create 2 borrows - one for read and one for write
        //TODO must enable 2 threads working inside NClnpService.
        //TODO modify to have NClnpService .read and .write inner parts - only these get borrowed. And these 2 only lock the shared host lists etc. when really needed.
        n_service_to.push(NUnitDataIndication{
            ns_source_address: sn_source_address,
            ns_destination_address: sn_destination_address,
            ns_quality_of_service: n_quality_of_service,
            ns_userdata: sn_userdata.to_vec()    //TODO optimize
        });
    }

    /// read and write to/from socket
    fn run(&self) {
        // read SN-UNITDATA Indications from the socket
        let buffer_arc = self.buffer_in.clone();
        let mut socket = self.socket.clone();   //TODO optimize
        let n_service_to_arc = self.n_service_to.clone();
        let _ = thread::spawn(move || {
            let mut buffer = *buffer_arc.lock().expect("failed to lock buffer_in");
            let mut n_service_to = n_service_to_arc.lock().expect("failed to lock n_service_to");  //TODO optimize - gets locked on every iteration
            loop {
                //let mut buffer = [0u8; 1500];
                println!("reading frame...");
                let num_bytes = socket.read(&mut buffer).expect("could not read DL frame from socket into buffer"); //TODO handle network down - dont crash, but try again

                // hand-cooked version, because we dont care about getting IP and TCP/UDP parsed
                let eth_header = etherparse::Ethernet2HeaderSlice::from_slice(&buffer).expect("could not parse Ethernet2 header");
                println!("destination: {:x?}  source: {:x?}  ethertype: 0x{:04x}", eth_header.destination(), eth_header.source(), eth_header.ether_type());
                let mut vlan_len: usize = 0;
                match eth_header.ether_type() {
                    ether_type::VLAN_TAGGED_FRAME | ether_type::PROVIDER_BRIDGING | ether_type::VLAN_DOUBLE_TAGGED_FRAME => {
                        let buffer_length = buffer.len();
                        let vlan_header = SingleVlanHeaderSlice::from_slice(&buffer[eth_header.slice().len()-1..buffer_length-1]).expect("could not parse single VLAN header");
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
                let qos = Qos{};    //TODO optimize allocation
                Self::sn_unitdata_indication(
                    &mut n_service_to, //TODO optimize clunky - &mut self would be nice but complains about 2 mutable borrows to self
                    MacAddr6::from(eth_header.source()),
                    MacAddr6::from(eth_header.destination()),
                    qos,
                    &buffer[0+eth_header.slice().len() .. num_bytes]    //TODO plus VLAN 802.11q (?) header, if present
                );
            }
        });

        // read NS-UNITDATA-REQUEST from NS
        let mut n_service_from = self.n_service_from.lock().expect("failed to lock n_service_from");
        loop {
            if let Ok(sn_unitdata_request) = n_service_from.pop() {
                println!("got sn_unitdata_request from NS: {:?}", sn_unitdata_request);
            }
            thread::sleep(std::time::Duration::from_millis(500));
        }
    }

    fn run2(&self) {
    }
}