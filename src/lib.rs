use etherparse::{ether_type, SingleVlanHeaderSlice, Ethernet2Header};
use netconfig::Interface;
use advmac::MacAddr6;
use std::{thread, time::Duration};
use afpacket::sync::RawPacketStream;
use std::io::Read;

mod n;
mod sn;
use crate::n::clnp::Qos;

pub fn add(left: usize, right: usize) -> usize {
    left + right
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
    let mut ps = RawPacketStream::new_with_ethertype(n::numbers::ETHER_TYPE_CLNP).expect("failed to create new raw socket on given interface");
    ps.bind_with_ethertype(interface_name, n::numbers::ETHER_TYPE_CLNP).expect("failed to bind to interface");

    // configure interface
    let iface_config = Interface::try_from_name(interface_name).expect("could not look up interface by name");

    // get MAC address
    let macaddr = iface_config.hwaddress().expect("could not get hardware address of interface");
    println!("got DLSAP: {}", macaddr);

    // dont need it anymore
    drop(iface_config);

    // start up OSI network service
    let mut ns = n::clnp::NClnpService::new(ps.clone());
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
        n::clnp::NClnpService::n_unitdata_indication(
            MacAddr6::from(eth_header.source()),
            MacAddr6::from(eth_header.destination()),
            &qos,
            &buffer[0+eth_header.slice().len() .. num_bytes]    //TODO plus VLAN 802.11q (?) header, if present
        );

        //let network_slice = &buffer[eth_header.slice().len() + vlan_len .. read_bytes];
        //println!("network_data: {:?}  len={}", network_slice, network_slice.len());
    }
}