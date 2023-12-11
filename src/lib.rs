use etherparse::{SlicedPacket, ether_type, VlanSlice, SingleVlanHeaderSlice, Ethernet2Header};
use netconfig::sys::InterfaceExt;
use pnet::datalink::Config;
use tun_tap::{Iface, Mode};
use netconfig::Interface;
//use ethernet::Ethernet2Header;
//use pdu::EthernetPdu;
use advmac::MacAddr6;
use std::collections::HashMap;
// used by pdu+netconfig
use std::{thread, time::Duration, result::Result};

extern crate pnet;
use pnet::datalink::{self, NetworkInterface, EtherType};
use pnet::datalink::Channel::Ethernet;
use pnet::packet::{Packet, MutablePacket};
use pnet::packet::ethernet::{EthernetPacket, MutableEthernetPacket};

use afpacket::sync::RawPacketStream;
use std::io::{Read, Write};
//use nom::HexDisplay;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

const ETHER_TYPE_CLNP: u16 = 0x8872;  // as per https://datatracker.ietf.org/doc/html/draft-kaplan-isis-ext-eth-ip-clns-2-00

pub fn parse_macaddr(instr: &str) -> Result<MacAddr6, advmac::ParseError> {
    MacAddr6::parse_str(instr)
}

pub fn new_interface(macaddr: Option<MacAddr6>, dest_macaddr: Option<MacAddr6>) -> String {
    // NOTE: the tun/tap driver's prefixed "protocol info" is just useful for TUN devices (to get IP protocol)
    let iface = Iface::without_packet_info("tap0", Mode::Tap).expect("Failed to create a TAP device");
    let name = iface.name();
    //iface.set_non_blocking().expect("could not set interface nonblocking")
    println!("got interface name: {}", name);

    // configure interface
    let iface_config = Interface::try_from_name(name).expect("could not look up interface by name");

    // set MAC address
    if macaddr.is_some() {
        println!("setting given hardware address...");
        iface_config.set_hwaddress(macaddr.unwrap()).expect("could not set hardware address on interface");
    }

    // get MAC address
    let hwaddr = iface_config.hwaddress().expect("could not get hardware address of interface");
    println!("got hardware address: {}", hwaddr);
    //TODO set static hardware address

    // set interface up
    println!("bring interface up...");
    iface_config.set_up(true).expect("failed to bring interface up");
    iface_config.set_running(true).expect("failed to set interface to running");
    thread::sleep(Duration::from_millis(1000));

    // remove any assigned IP addresses, otherwise the interface constantly looks for routers and DHCP servers
    // NOTE: addresses are assigned automatically only AFTER bringing the interface up
    println!("remove any IP addresses on interface...");
    iface_config.addresses().expect("failed to enumerate interface addresses").into_iter().for_each(|x| { iface_config.remove_address(x).expect("could not remove address from interface"); } );
    /*
    let name = "tap0";
    let hwaddr = macaddr.unwrap();
    */

    // pnet starting here
    let interface_names_match =|iface: &NetworkInterface| iface.name == name;

    // Find the network interface with the provided name
    let interfaces = datalink::interfaces();
    let interface = interfaces.into_iter()
                            .filter(interface_names_match)
                            .next()
                            .expect("could not find my interface");
    println!("pnet hat mac address: {:?}", interface.mac.expect("could not get interface mac"));

    // Create a new channel, dealing with layer 2 packets
    let (mut tx, mut rx) = match datalink::channel(&interface, Config {
        write_buffer_size: 4096,
        read_buffer_size: 4096,
        read_timeout: Some(Duration::from_millis(10*1000)),
        write_timeout: Some(Duration::from_millis(10*1000)),
        channel_type: datalink::ChannelType::Layer2,    //datalink::ChannelType::Layer3(ETHER_TYPE_CLNP),
        bpf_fd_attempts: Config::default().bpf_fd_attempts,
        linux_fanout: None,
        promiscuous: false,
    }) {
        // TODO ^ must be modified to allow setting the EtherType in Config:  https://docs.rs/pnet_datalink/0.34.0/src/pnet_datalink/linux.rs.html#100
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("Unhandled channel type"),
        Err(e) => panic!("An error occurred when creating the datalink channel: {}", e)
    };

    loop {
        println!("waiting packet...");
        match rx.next() {
            Ok(packet) => {
                let packet = EthernetPacket::new(packet).expect("could not create new ethernet packet");
                println!("got packet source={}  destination={}  ethertype={:x?}  len={}, discarding", packet.get_source(), packet.get_destination(), packet.get_ethertype(), packet.packet().len());

                // Constructs a single packet, the same length as the the one received,
                // using the provided closure. This allows the packet to be constructed
                // directly in the write buffer, without copying. If copying is not a
                // problem, you could also use send_to.
                //
                // The packet is sent once the closure has finished executing.
                /*
                tx.build_and_send(1, packet.packet().len(),
                    &mut |new_packet| {
                        let mut new_packet = MutableEthernetPacket::new(new_packet).expect("could not create new ethernet packet for sending back");

                        // Create a clone of the original packet
                        //new_packet.clone_from(&packet);
                        new_packet.set_ethertype(pnet::packet::ethernet::EtherType(0x8872));

                        // Switch the source and destination
                        new_packet.set_source(packet.get_destination());
                        new_packet.set_destination(packet.get_source());
                });
                println!("sent reply");
                */

                /*
                for _ in 0..1 {
                    println!("sending packet from {:x?} to {:x?}...", hwaddr, dest_macaddr);
                    tx.build_and_send(1, packet.packet().len(),
                    &mut |new_packet| {
                        let mut new_packet = MutableEthernetPacket::new(new_packet).expect("could not create new ethernet packet for sending back");

                        new_packet.set_ethertype(pnet::packet::ethernet::EtherType(ETHER_TYPE_CLNP));
                        new_packet.set_destination(pnet::datalink::MacAddr(dest_macaddr.unwrap().as_slice()[0], dest_macaddr.unwrap().as_slice()[1], dest_macaddr.unwrap().as_slice()[2], dest_macaddr.unwrap().as_slice()[3], dest_macaddr.unwrap().as_slice()[4], dest_macaddr.unwrap().as_slice()[5]));
                        new_packet.set_source(pnet::datalink::MacAddr(hwaddr.as_slice()[0], hwaddr.as_slice()[1], hwaddr.as_slice()[2], hwaddr.as_slice()[3], hwaddr.as_slice()[4], hwaddr.as_slice()[5]));

                        iface.send(&new_packet.packet()).expect("could not directly send the data into interface");
                    });
                    thread::sleep(Duration::from_millis(2*1000));
                }
                */
            },
            Err(e) => {
                // If an error occurs, we can handle it here
                //panic!("An error occurred while reading: {}", e);

                println!("timeout on read (err is {}), sending a packet...", e);

                for _ in 0..1 {
                    println!("sending packet from {:x?} to {:x?}...", hwaddr, dest_macaddr);
                    //tx.send_to(&[1, 2, 3], None);
                    tx.build_and_send(1, 20,
                    &mut |new_packet| {
                        let mut new_packet = MutableEthernetPacket::new(new_packet).expect("could not create new ethernet packet for sending back");

                        new_packet.set_destination(pnet::datalink::MacAddr(dest_macaddr.unwrap().as_slice()[0], dest_macaddr.unwrap().as_slice()[1], dest_macaddr.unwrap().as_slice()[2], dest_macaddr.unwrap().as_slice()[3], dest_macaddr.unwrap().as_slice()[4], dest_macaddr.unwrap().as_slice()[5]));
                        new_packet.set_source(pnet::datalink::MacAddr(hwaddr.as_slice()[0], hwaddr.as_slice()[1], hwaddr.as_slice()[2], hwaddr.as_slice()[3], hwaddr.as_slice()[4], hwaddr.as_slice()[5]));
                        new_packet.set_ethertype(pnet::packet::ethernet::EtherType(ETHER_TYPE_CLNP));

                        //iface.send(&new_packet.packet()[0..17]).expect("could not directly send the data into interface");
                    });
                    thread::sleep(Duration::from_millis(1000));
                }
            }
        }

        //iface.recv(buf)










    /*
    loop {
        println!("receiving...");
        let mut buffer = vec![0; 1500]; // MTU
        let read_bytes = iface.recv(&mut buffer).expect("could not receive packet");
        println!("got packet with {} bytes, parsing", read_bytes);
    */

        // hand-cooked version, because we dont care about getting IP and TCP/UDP parsed
        /*
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
            ETHER_TYPE_CLNP => { println!("ah, got CLNP - feel warmly welcome!"); }
            _ => { println!("{}", "got unknown, discarding"); }
        }
        let network_slice = &buffer[eth_header.slice().len() + vlan_len .. read_bytes];
        println!("network_data: {:?}  len={}", network_slice, network_slice.len());
        */


    }
}

#[derive(Debug)]
enum NClnpPdu<'a> {
    Inactive { fixed_mini: NFixedPartMiniForInactive<'a>, data: NDataPart<'a> },
    NDataPDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, seg: Option<NSegmentationPart<'a>>, opt: Option<NOptionsPart<'a>>, discard: Option<NReasonForDiscardPart<'a>>, data: Option<NDataPart<'a>>},
    // no segmentation, but reason for discard is mandatory
    ErrorReportPDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, op: Option<NOptionsPart<'a>>, discard: NReasonForDiscardPart<'a>, data: Option<NDataPart<'a>> },
    // these are the same as DataPDU / DT PDU
    EchoRequestPDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, seg: Option<NSegmentationPart<'a>>, opt: Option<NOptionsPart<'a>>, discard: Option<NReasonForDiscardPart<'a>>, data: Option<NDataPart<'a>> },
    EchoResponsePDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, seg: Option<NSegmentationPart<'a>>, opt: Option<NOptionsPart<'a>>, discard: Option<NReasonForDiscardPart<'a>>, data: Option<NDataPart<'a>> },
    MulticastDataPDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, seg: Option<NSegmentationPart<'a>>, opt: Option<NOptionsPart<'a>>, discard: Option<NReasonForDiscardPart<'a>>, data: Option<NDataPart<'a>> }
}

#[derive(Debug)]
struct NFixedPartMiniForInactive<'a> {
    network_layer_protocol_identifier: &'a u8
}

// TODO was not possible to have it as an enum and match on it, comparing to u8
const NETWORK_LAYER_PROTOCOL_IDENTIFIER_CLNP_FULL: u8 = 0b1000_0001;
const NETWORK_LAYER_PROTOCOL_IDENTIFIER_CLNP_INACTIVE: u8 = 0b0000_0000;

#[derive(Debug)]
struct NFixedPart<'a> {
    network_layer_protocol_identifier: &'a u8,
    length_indicator: &'a u8,
    version_protocol_id_extension: &'a u8,
    lifetime: &'a u8,
    /// 0 = not permitted, no segmentation part present in PDU, non-segmenting protocol subset in use
    /// 1 = permitted, segmentation part shall be present in PDU, full protocol is in use
    sp_segmentation_permitted: bool,   // 1 bit
    ms_more_segments: bool,   // 1 bit
    er_error_report: bool,  // 1 bit
    type_: bool, // 5 bits
    /// contains ^ sub-bit values
    octet5: &'a u8,
    segment_length: &'a u16,
    checksum: &'a u16
}

enum SpSegmentationPermittedBit {
    ONE = 0b1000_0000,
    ZERO = 0b0000_0000
}

enum MsMoreSegmentsBit {
    ONE = 0b0100_0000,
    ZERO = 0b0000_0000
}

enum ErErrorReportBit {
    ONE = 0b0010_0000,
    ZERO = 0b0000_0000
}

impl NFixedPart<'_> {
    /// Return the bits for octet 5 of the fixed part of the NPDU
    fn compose_octet5(sp_segmentation_permitted: SpSegmentationPermittedBit,
        ms_more_segments: MsMoreSegmentsBit,
        er_error_report: ErErrorReportBit,
        type_: u8
    ) -> Option<u8>  {
        if type_ >= 32 {
            // only have 5 bits of space (0 to 31)
            return None;
        }
        return Some(sp_segmentation_permitted as u8 | ms_more_segments as u8 | er_error_report as u8| type_);
    }

    /// Return the bits for octet 5 of the fixed part of the NPDU
    fn compose_octet5_unchecked(sp_segmentation_permitted: SpSegmentationPermittedBit,
        ms_more_segments: MsMoreSegmentsBit,
        er_error_report: ErErrorReportBit,
        type_: u8
    ) -> u8  {
        // simply overwrites any data in bits 1,2,3 if number in type uses more than 5 bits
        return sp_segmentation_permitted as u8 | ms_more_segments as u8 | er_error_report as u8| type_;
    }
}

#[derive(Debug)]
struct NAddressPart<'a> {
    destination_address_length_indicator: &'a u8,
    destination_address: &'a [u8],
    source_address_length_indicator: &'a u8,
    source_address: &'a [u8]
}

#[derive(Debug)]
struct NSegmentationPart<'a> {
    data_unit_identifier: &'a u16,
    segment_offset: &'a u16,
    total_length: &'a u16
}

#[derive(Debug)]
struct NOptionsPart<'a> {
    params: &'a [NParameter<'a>]
}

/// only contained in NOptionsPart
//TODO decomposition of these parameters
#[derive(Debug)]
struct NParameter<'a> {
    parameter_code: &'a u8,
    parameter_length: &'a u8,
    parameter_value: &'a [u8],
}

#[derive(Debug)]
struct NReasonForDiscardPart<'a> {
    /// has format of a parameter from the options part
    param: &'a NParameter<'a>   //TODO enforce that here only parameter code "1100 0001" is allowed
}

#[derive(Debug)]
struct NDataPart<'a> {
    data: &'a [u8]
}

impl crate::NClnpPdu<'_> {
    fn into_buf(&self, buffer: &mut [u8]) -> usize {
        //TODO check if given buffer is really < MTU
        match self {
            Self::Inactive{fixed_mini, data} => {
                buffer[0] = *fixed_mini.network_layer_protocol_identifier as u8;
                //TOD optimize
                for i in 0..data.data.len() {
                    buffer[i+1] = data.data[i];
                }
                return 1 + data.data.len();
            },
            _ => { todo!(); }
        }
        //matches!(self, Self::Inactive { .. })
    }

    fn from_buf(buffer: &[u8]) -> NClnpPdu {
        match buffer[0] {
            NETWORK_LAYER_PROTOCOL_IDENTIFIER_CLNP_FULL => {
                todo!();
            },
            NETWORK_LAYER_PROTOCOL_IDENTIFIER_CLNP_INACTIVE => {
                NClnpPdu::Inactive {
                    fixed_mini: NFixedPartMiniForInactive { network_layer_protocol_identifier: &buffer[0] },
                    data: NDataPart { data: &buffer[1..buffer.len()] }  //TODO note, we dont really know how long the data part is at this point
                }
            }
            _ => {
                todo!();
            }
        }
    }
}

//TODO implement full NSAP
#[derive(Clone)]
struct Nsap {
    authority: u16, // 49 = local network
    area: u16,  //net (?)
    sub_area: u16,  //subnet (?)
    local_address: MacAddr6,    //TODO fix - this is of course not correct
}

struct Qos {
    //TODO
}

struct NClnpService {
    // internal state
    serviced_nsaps: Vec<Nsap>,
    known_hosts: HashMap<String, Nsap>,

    // underlying data link service
    //TODO kind of - package that
    socket: RawPacketStream,
    buffer: [u8; 1500],
}

impl NClnpService {
    pub fn new(socket: RawPacketStream) -> NClnpService {
        NClnpService {
            socket: socket,
            buffer: [0u8; 1500],
            serviced_nsaps: vec![],
            known_hosts: HashMap::new(),
        }
    }

    // TODO serviced NSAP
    // TODO fix parameters
    pub fn add_serviced_nsap(&mut self, authority: u16, area: u16, sub_area: u16, remainder: MacAddr6) {
        self.serviced_nsaps.push(Nsap {
            authority: authority,
            area: area,
            sub_area: sub_area,
            local_address: remainder,
        })
    }

    // TODO serviced NSAP in subnet according to "expected services of subnet network service" or so (?)
    pub fn add_serviced_subnet_nsap(&mut self, net: u16, sub_net: u16, macaddr: MacAddr6) {
        self.add_serviced_nsap(49, net, sub_net, macaddr);
    }

    //TODO quick version - implement proper name lookup
    pub fn resolve_nsap(&self, system_title: &str) -> Option<&Nsap> {
        if let Some(address) = self.known_hosts.get(system_title) {
            return Some(address);
        } else {
            return None;
        }
    }

    //TODO quick version - implement proper name lookup
    //TODO currently we use MAC access for "NSAP"
    pub fn add_known_host(&mut self, system_title: String, nsap: &str) {
        self.known_hosts.insert(system_title, Nsap {
            authority: 49,
            area: 1,
            sub_area: 1,
            local_address: parse_macaddr(nsap).expect("could not parse mac address"),
        });
    }

    //TODO there are/can be multiple
    pub fn get_serviced_nsap(&mut self) -> Option<&Nsap> {
        return self.serviced_nsaps.get(0);
    }

    pub fn n_unitdata_request(
        &mut self,
        ns_destination_title: &str,
        ns_quality_of_service: &Qos,
        ns_userdata: &[u8]
    ) {
        let get_serviced_nsap = self.get_serviced_nsap().expect("no serviced NSAPs").clone();
        let dest_nsap = self.resolve_nsap(ns_destination_title).expect("cannot resolve destination host").clone();
        self.n_unitdata_request_internal(
            &get_serviced_nsap,
            &dest_nsap,
            &ns_quality_of_service,
            ns_userdata
        );
    }

    // TODO only inactive implemented
    fn n_unitdata_request_internal(
        &mut self,
        ns_source_address: &Nsap,
        ns_destination_address: &Nsap,
        ns_quality_of_service: &Qos,
        ns_userdata: &[u8]
    ) {
        // check if we are on same Ethernet broadcast domain as destination
        if can_use_inactive_subset(ns_source_address, ns_destination_address) {
            // compose PDU(s)
            let pdus = pdu_composition(true, ns_source_address, ns_destination_address, ns_quality_of_service, ns_userdata);
            for pdu in pdus {
                // send DLPDU (Ethernet frame header)
                let pkt_out = etherparse::Ethernet2Header{
                    destination: ns_destination_address.local_address.to_array(),
                    source: ns_source_address.local_address.to_array(),
                    ether_type: ETHER_TYPE_CLNP,
                };
                println!("writing DLPDU...");
                let mut remainder = pkt_out.write_to_slice(&mut self.buffer).expect("failed writing DLPDU into buffer");
                //pkt_out.write(&mut self.socket).expect("failed writing frame into socket");

                // send NPDU (CLNP PDU)
                println!("writing NPDU...");
                let bytes = pdu.into_buf(&mut remainder);
                self.socket.write(&self.buffer[0..bytes + 14]).expect("could not write buffer into socket");    //TODO +14 is not cleanly abtracted

                println!("flushing DL...");
                self.socket.flush().expect("failed to flush socket");
            }
            return;
        }
        todo!();
    }

    //TODO implement properly (PDU decomposition)
    fn n_unitdata_indication(
        ns_source_address: MacAddr6,
        ns_destination_address: MacAddr6,
        ns_quality_of_service: &Qos,
        ns_userdata: &[u8]
    ) {
        println!("got CLNP packet: {:?}", NClnpPdu::from_buf(ns_userdata));
    }
}

//TODO
fn can_use_inactive_subset(ns_source_address: &Nsap, ns_destination_address: &Nsap) -> bool {
    // TODO check if on same subnetwork (AKA in same Ethernet segment)
    true
}

// 6.1
// TODO WIP
// TODO optimize - this function allocates CLNP PDUs for every call
fn pdu_composition<'a>(inactive: bool, ns_source_address: &'a Nsap, ns_destination_address: &'a Nsap, ns_quality_of_service: &'a Qos, ns_userdata: &'a [u8]) -> Vec<NClnpPdu<'a>> {
    if inactive {
        return vec![NClnpPdu::Inactive {
            fixed_mini: NFixedPartMiniForInactive { network_layer_protocol_identifier: &NETWORK_LAYER_PROTOCOL_IDENTIFIER_CLNP_INACTIVE },
            data: NDataPart { data: ns_userdata }
        }]
    } else {
        todo!();
    }
}

enum HeaderFormatAnalysisResult {
    TooShortTooIdentify,
    FullProtocol,
    InactiveProtocol,
    UnknownProtocol,
}

// 6.3
// TODO only what is neccessary for inactive protocol subset
fn header_format_analysis(packet: &[u8]) -> HeaderFormatAnalysisResult {
    if packet.len() < 1 {
        return HeaderFormatAnalysisResult::TooShortTooIdentify;
    }
    match packet[0] {
        NETWORK_LAYER_PROTOCOL_IDENTIFIER_CLNP_FULL => HeaderFormatAnalysisResult::FullProtocol,
        NETWORK_LAYER_PROTOCOL_IDENTIFIER_CLNP_INACTIVE => HeaderFormatAnalysisResult::InactiveProtocol,
        //TODO check after ^ The Network entity in this case determines that either the Subnetwork Point of Attachment
        //(SNPA) address encoded as NPAI in the supporting subnetwork protocol (see 8.1) corresponds directly to an NSAP
        //address serviced by this Network entity, or that an error has occurred.
        _ => HeaderFormatAnalysisResult::UnknownProtocol
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}

// TODO maybe switch to pnet-datalink. but also needs to be fixed for ethertype parameter to socket() and bind()
pub fn new_interface2(interface_name: &str, dest_host: &str, hosts: Vec<(&str, &str)>) -> String {
    let mut ps = RawPacketStream::new_with_ethertype(ETHER_TYPE_CLNP).expect("failed to create new raw socket on given interface");
    ps.bind_with_ethertype(interface_name, ETHER_TYPE_CLNP).expect("failed to bind to interface");

    // configure interface
    let iface_config = Interface::try_from_name(interface_name).expect("could not look up interface by name");

    // get MAC address
    let macaddr = iface_config.hwaddress().expect("could not get hardware address of interface");
    println!("got DLSAP: {}", macaddr);

    // dont need it anymore
    drop(iface_config);

    // start up OSI network service
    let mut ns = NClnpService::new(ps.clone());
    // set own/serviced NSAPs
    ns.add_serviced_subnet_nsap(1, 1, macaddr);
    // add known hosts
    for host in hosts {
        ns.add_known_host(host.0.to_owned(), host.1);   //TODO optimize clone
    }

    // send request to other host
    let qos = Qos{};
    let dest_host2 = dest_host.to_owned();  // clone in order to give it the thread
    let _ = thread::spawn(move || {
        let dest_host3 = dest_host2.as_str();
        loop {
            print!("sending packet...");
            ns.n_unitdata_request(
                dest_host3,
                &qos,
                r"test".as_bytes()
            );
            println!("done");

            thread::sleep(Duration::from_secs(2));
        }
    });

    // read frame
    //TODO change to use network service
    //TODO currently it does not have that feature
    let qos = Qos{};
    loop {
        let mut buffer = [0u8; 1500];
        println!("reading frame...");
        let num_bytes = ps.read(&mut buffer).expect("could not read DL frame from socket into buffer");
        //println!("got frame: {}", buffer.to_hex(24));
        
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
            ETHER_TYPE_CLNP => { println!("ah, got CLNP - feel warmly welcome!"); }
            _ => { println!("{}", "got unknown, discarding"); }
        }

        // forward up from DL to N layer
        //TODO this method will need &mut self at some point, but this will create 2 borrows - one for read and one for write
        //TODO must enable 2 threads working inside NClnpService.
        //TODO modify to have NClnpService .read and .write inner parts - only these get borrowed. And these 2 only lock the shared host lists etc. when really needed.
        NClnpService::n_unitdata_indication(
            MacAddr6::from(eth_header.source()),
            MacAddr6::from(eth_header.destination()),
            &qos,
            &buffer[0+eth_header.slice().len() .. num_bytes]    //TODO plus VLAN 802.11q (?) header, if present
        );

        //let network_slice = &buffer[eth_header.slice().len() + vlan_len .. read_bytes];
        //println!("network_data: {:?}  len={}", network_slice, network_slice.len());
    }
}