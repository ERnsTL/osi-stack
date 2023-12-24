use std::thread;
use netconfig::Interface;
use afpacket::sync::RawPacketStream;

pub mod n;
mod dl;
use crate::{n::NetworkService, dl::SubnetworkService};

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
pub fn new<'a>(interface_name: &'a str, network_entity_title: &'a str, hosts: Vec<(&str, &str)>) -> n::clnp::Service<'a> {
    let mut ps = RawPacketStream::new_with_ethertype(dl::ETHER_TYPE_CLNP).expect("failed to create new raw socket on given interface");
    ps.bind_with_ethertype(interface_name, dl::ETHER_TYPE_CLNP).expect("failed to bind to interface");

    // configure interface
    let iface_config = Interface::try_from_name(interface_name).expect("could not look up interface by name");

    // get MAC address
    let macaddr = iface_config.hwaddress().expect("could not get hardware address of interface");
    println!("got SNPA address: {}", macaddr);

    // dont need it anymore
    drop(iface_config);

    // start up OSI network service
    let (mut sn2ns_producer, mut sn2ns_consumer) = rtrb::RingBuffer::new(7);
    let (mut ns2sn_producer, mut ns2sn_consumer) = rtrb::RingBuffer::new(7);
    let mut sn = dl::ethernet::Service::new(ps, ns2sn_consumer, sn2ns_producer);
    let mut ns = n::clnp::Service::new(network_entity_title, ns2sn_producer, sn2ns_consumer);
    // set own/serviced NSAPs
    //TODO optimize locking here - maybe it is fine to pack up ns and sn into Arc<Mutex<>> upon calling run()
    ns.add_serviced_subnet_nsap(1, 1, macaddr);
    // add known hosts
    for host in hosts {
        ns.add_known_host(host.0.to_owned(), host.1);   //TODO optimize clone
    }

    // start SN
    //let _ = thread::spawn(move || {
        sn.run();
    //});
    /*
    let _ = thread::spawn(|| {
        sn.run2();
    });
    */
    /*
    let _ = thread::spawn(move || {
        ns.run();
    });
    */

    return ns;  //TODO instead of NS, return likely the ACSE for registering applications
}