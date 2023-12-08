use etherparse::{SlicedPacket, ether_type, VlanSlice, SingleVlanHeaderSlice, Ethernet2Header};
use netconfig::sys::InterfaceExt;
use tun_tap::{Iface, Mode};
use netconfig::Interface;
//use ethernet::Ethernet2Header;
//use pdu::EthernetPdu;
use advmac::MacAddr6;   // used by pdu+netconfig
use std::{thread, time::Duration};

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

const ETHER_TYPE_CLNP: u16 = 0x8872;  // as per https://datatracker.ietf.org/doc/html/draft-kaplan-isis-ext-eth-ip-clns-2-00

pub fn parse_macaddr(instr: &str) -> Result<MacAddr6, advmac::ParseError> {
    MacAddr6::parse_str(instr)
}

pub fn new_interface(macaddr: Option<MacAddr6>, dest_macaddr: Option<MacAddr6>) -> String {
    // NOTE: the tun/tap driver's prefixed "protocol info" is just useful for TUN devices (to get IP protocol)
    let iface = Iface::without_packet_info("", Mode::Tap).expect("Failed to create a TAP device");
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

    // remove any assigned IP addresses, otherwise the interface constantly looks for routers and DHCP servers
    // NOTE: addresses are assigned automatically only AFTER bringing the interface up
    println!("remove any IP addresses on interface...");
    iface_config.addresses().expect("failed to enumerate interface addresses").into_iter().for_each(|x| { iface_config.remove_address(x).expect("could not remove address from interface"); } );

    loop {
        println!("receiving...");
        let mut buffer = vec![0; 1500]; // MTU
        let read_bytes = iface.recv(&mut buffer).expect("could not receive packet");
        println!("got packet with {} bytes, parsing", read_bytes);

        //let eth2header = Ethernet2Header::from_bytes(&buffer[0..(6+6+2)]);
        //println!("got {:?}", eth2header);

        /*
        match SlicedPacket::from_ethernet(&buffer) {
            Err(value) => println!("Err {:?}", value),
            Ok(value) => {
                println!("link: {:?}", value.link.unwrap().to_header());
                println!("vlan: {:?}", value.vlan);
                //println!("ip: {:?}", value.ip);
                //println!("transport: {:?}", value.transport);
                println!("payload: {:?}", value.payload);
            }
        }
        */

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
        let network_slice = &buffer[eth_header.slice().len() + vlan_len .. read_bytes];
        println!("network_data: {:?}  len={}", network_slice, network_slice.len());

        /*
        match EthernetPdu::new(&buffer) {
            Ok(ethernet_pdu) => {
                print!("[ethernet] destination_address: {:x?} ", ethernet_pdu.destination_address().as_ref());
                print!("source_address: {:x?} ", ethernet_pdu.source_address().as_ref());
                print!("ethertype: 0x{:04x} ", ethernet_pdu.ethertype());
                if let Some(vlan) = ethernet_pdu.vlan() {
                    print!("vlan: 0x{:04x} ", vlan);
                }
                println!(" ");

                // of interest?
                match ethernet_pdu.ethertype() {
                    0x86dd => { println!("{}", "got ipv6, ignoring"); }
                    0x0800 => { println!("{}", "got ipv4, ignoring"); }
                    ETHER_TYPE_CLNP => { println!("ah, got CLNP - feel warmly welcome!"); }
                    _ => { println!("{}", "got unknown, discarding"); }
                }

            }
            Err(e) => {
                panic!("EthernetPdu::new() parser failure: {:?}", e);
            }
        }
        */

	loop {
        // send a packet
        if let Some(dest_macaddr) = dest_macaddr {
            print!("sending packet from {:x?} to {:x?}...", hwaddr, dest_macaddr);
            let eth_header_out = Ethernet2Header {
                destination: dest_macaddr.to_array(),
                source: hwaddr.to_array(),
                ether_type: ETHER_TYPE_CLNP,
            };
            let _ = eth_header_out.write_to_slice(&mut buffer).expect("could not write out packet into buffer");
            let sent_bytes = iface.send(&buffer[0..100]).expect("could not send out packet");
            println!("done, sent {} bytes", sent_bytes);
        };
        thread::sleep(Duration::from_millis(1000));
        }
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
