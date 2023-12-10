use etherparse::{SlicedPacket, ether_type, VlanSlice, SingleVlanHeaderSlice, Ethernet2Header};
use netconfig::sys::InterfaceExt;
use pnet::datalink::Config;
use tun_tap::{Iface, Mode};
use netconfig::Interface;
//use ethernet::Ethernet2Header;
//use pdu::EthernetPdu;
use advmac::MacAddr6;   // used by pdu+netconfig
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

enum NClnpPdu<'a> {
    Inactive(NFixedPartMiniForInactive<'a>, NDataPart<'a>),
    NDataPDU {fixed: NFixedPart<'a>, addr: NAddressPart<'a>, seg: Option<NSegmentationPart<'a>>, opt: Option<NOptionsPart<'a>>, discard: Option<NReasonForDiscardPart<'a>>, data: Option<NDataPart<'a>>},
    // no segmentation, but reason for discard is mandatory
    ErrorReportPDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, op: Option<NOptionsPart<'a>>, discard: NReasonForDiscardPart<'a>, data: Option<NDataPart<'a>> },
    // these are the same as DataPDU / DT PDU
    EchoRequestPDU{ fixed: NFixedPart<'a>, addr: NAddressPart<'a>, seg: Option<NSegmentationPart<'a>>, opt: Option<NOptionsPart<'a>>, discard: Option<NReasonForDiscardPart<'a>>, data: Option<NDataPart<'a>> },
    EchoResponsePDU{ fixed: NFixedPart<'a>, addr: NAddressPart<'a>, seg: Option<NSegmentationPart<'a>>, opt: Option<NOptionsPart<'a>>, discard: Option<NReasonForDiscardPart<'a>>, data: Option<NDataPart<'a>> },
    MulticastDataPDU{ fixed: NFixedPart<'a>, addr: NAddressPart<'a>, seg: Option<NSegmentationPart<'a>>, opt: Option<NOptionsPart<'a>>, discard: Option<NReasonForDiscardPart<'a>>, data: Option<NDataPart<'a>> }
}

struct NFixedPartMiniForInactive<'a> {
    network_layer_protocol_identifier: &'a u8
}

struct NFixedPart<'a> {
    network_layer_protocol_identifier: &'a u8,
    length_indicator: &'a u8,
    version_protocol_id_extension: &'a u8,
    lifetime: &'a u8,
    /// 0 = not permitted, no segmentation part present in PDU, non-segmenting protocol subset in use
    /// 1 = permitted, segmentation part shall be present in PDU, full protocol is in use
    sp_segmentation_permitted: bool,   //TODO sub-byte value
    ms_more_segments: bool,   //TODO sub-byte value
    er_error_report: bool,  //TODO sub-byte value
    type_: bool, //TODO sub-byte value
    segment_length: &'a u16,
    checksum: &'a u16
}

struct NAddressPart<'a> {
    destination_address_length_indicator: &'a u8,
    destination_address: &'a [u8],
    source_address_length_indicator: &'a u8,
    source_address: &'a [u8]
}

struct NSegmentationPart<'a> {
    data_unit_identifier: &'a u16,
    segment_offset: &'a u16,
    total_length: &'a u16
}

struct NOptionsPart<'a> {
    params: &'a [NParameter<'a>]
}

/// only contained in NOptionsPart
//TODO decomposition of these parameters
struct NParameter<'a> {
    parameter_code: &'a u8,
    parameter_length: &'a u8,
    parameter_value: &'a [u8],
}

struct NReasonForDiscardPart<'a> {
    /// has format of a parameter from the options part
    param: &'a NParameter<'a>   //TODO enforce that here only parameter code "1100 0001" is allowed
}

struct NDataPart<'a> {
    data: &'a [u8]
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
pub fn new_interface2(interface_name: &str, dest_macaddr: Option<MacAddr6>) -> String {
    let mut ps = RawPacketStream::new_with_ethertype(ETHER_TYPE_CLNP).expect("failed to create new raw socket on given interface");
    ps.bind_with_ethertype(interface_name, ETHER_TYPE_CLNP).expect("failed to bind to interface");

    // configure interface
    let iface_config = Interface::try_from_name(interface_name).expect("could not look up interface by name");

    // get MAC address
    let macaddr = iface_config.hwaddress().expect("could not get hardware address of interface");
    println!("got DLSAP: {}", macaddr);
    
    // dont need it anymore
    drop(iface_config);
    
    // write frames
    if dest_macaddr.is_some() {
        let mut ps2 = ps.clone();
        let _ = thread::spawn(move || {
            loop {
                print!("sending frame...");
                let pkt_out = etherparse::Ethernet2Header{
                    destination: dest_macaddr.unwrap().to_array(),
                    source: macaddr.to_array(),
                    ether_type: ETHER_TYPE_CLNP,
                };
                //println!("writing...");
                pkt_out.write(&mut ps2).expect("failed writing frame into socket");
                //println!("flushing...");
                ps2.flush().expect("failed to flush socket");
                println!("done");

                thread::sleep(Duration::from_secs(2));
            }
        });
    } else {
        println!("no destination DLSAP given, not sending frames");
    }

    // read frame
    loop {
        let mut buffer = [0u8; 1500];
        println!("reading frame...");
        ps.read(&mut buffer).unwrap();
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

        //let network_slice = &buffer[eth_header.slice().len() + vlan_len .. read_bytes];
        //println!("network_data: {:?}  len={}", network_slice, network_slice.len());
    }
}