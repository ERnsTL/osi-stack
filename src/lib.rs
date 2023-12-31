use std::{thread::{self, Thread, JoinHandle}, sync::{Arc, Mutex}, time::Duration};
use netconfig::Interface;
use afpacket::sync::RawPacketStream;
#[macro_use] extern crate log;
extern crate simplelog; //TODO check the paris feature flag for tags, useful?

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
pub fn new<'a>(interface_name: &'a str, network_entity_title: &'a str, hosts: Vec<(&str, &str)>) -> (dl::ethernet::Service, n::clnp::Service<'a>) {
    // set up logging
    simplelog::TermLogger::init(
        simplelog::LevelFilter::Info,   // can locally increase this for dev, TODO make configurable via args - but better configure this in Cargo.toml
        simplelog::ConfigBuilder::default()
            .set_time_level(simplelog::LevelFilter::Off)
            .set_thread_level(simplelog::LevelFilter::Info)
            .set_thread_mode(simplelog::ThreadLogMode::Names)
            .set_thread_padding(simplelog::ThreadPadding::Right(15))    // maximum thread name length on Linux
            .set_level_padding(simplelog::LevelPadding::Right)
            .build(),
        simplelog::TerminalMode::Mixed, // level error and above to stderr, rest to stdout
        simplelog::ColorChoice::Auto    // depending on whether interactive or not
    ).expect("logging init failed");
    info!("osi-stack starting up"); //TODO add version information

    // connect raw socket to iterface, filtered by EtherType of interest
    let mut ps = RawPacketStream::new_with_ethertype(dl::ETHER_TYPE_CLNP).expect("failed to create new raw socket on given interface");
    ps.bind_with_ethertype(interface_name, dl::ETHER_TYPE_CLNP).expect("failed to bind to interface");

    // configure interface
    let iface_config = Interface::try_from_name(interface_name).expect("could not look up interface by name");

    // get MAC address
    let macaddr = iface_config.hwaddress().expect("could not get hardware address of interface");
    info!("got SNPA address: {}", macaddr);

    // dont need it anymore
    drop(iface_config);

    // compose OSI network stack
    //TODO ability to configure which protocols should be built into the stack
    // NOTE: producer is where the producer (originator) of a message writes into
    // and for each consumer we need a thread handle of the consumer (receiver) thread so that it can be woken up
    // the arc<mutex to receive that handle is given to the Service new() functions
    // and the arc<mutex to give the handle into is given to the Service run() functions
    // which may also take a clone of the arc<mutex of a consumer (receiver) thread as needed
    // In every thread where there are pushes into inter-layer connections done, it needs the consumer (receiver) thread handle to wake the receiver up
    // In every thread where there are pops from inter-layer connections done, it needs to give its thread handle into the arc<mutex (the well-known place) where the sender will get it from
    let (sn2ns_producer, sn2ns_consumer) = rtrb::RingBuffer::new(7);
    let (ns2sn_producer, ns2sn_consumer) = rtrb::RingBuffer::new(7);
    //TODO optimize - WakeupHandle does not require Arc<Mutex<WakeupHandle>>, but Arc<WakeupHandle> is enough - make use of this shortcut
    let sn2ns_consumer_wakeup: Arc<Mutex<Option<JoinHandle<Thread>>>> = Arc::new(Mutex::new(None));
    let ns2sn_consumer_wakeup: Arc<Mutex<Option<JoinHandle<Thread>>>> = Arc::new(Mutex::new(None));
    let sn = dl::ethernet::Service::new(ps, ns2sn_consumer, sn2ns_producer, sn2ns_consumer_wakeup.clone());
    let mut ns = n::clnp::Service::new(network_entity_title, ns2sn_producer, ns2sn_consumer_wakeup.clone(), sn2ns_consumer);
    // set own/serviced NSAPs
    //TODO optimize locking here - maybe it is fine to pack up ns and sn into Arc<Mutex<>> upon calling run()
    ns.add_serviced_subnet_nsap(1, 1, macaddr);
    // add known hosts
    for host in hosts {
        ns.add_known_host(host.0.to_owned(), host.1);   //TODO optimize clone
    }

    // start SN
    // NOTE: will go out of scope at end of this function, at the same time sn cannot be borrowed 2x for read and write threads
    // therefore, interior mutability and because we are multi-threaded, Arc<Mutex<>> is needed. Yay.
    //TODO optimize?
    sn.run(ns2sn_consumer_wakeup);
    // start NS
    ns.run(sn2ns_consumer_wakeup);
    // wait for above run() methods to give their thread wakeup handles - otherwise yet another signal channel needs to be implemented
    thread::sleep(Duration::from_millis(500));

    // NOTE: must return sn with the contained RawPacketStream, otherwise it goes out of scope, even though owned by the threads in sn.run(),
    // but they have only clones. The original must not trigger its free(). So we return it...
    return (sn, ns);  //TODO instead of NS, return likely the ACSE for registering applications
}