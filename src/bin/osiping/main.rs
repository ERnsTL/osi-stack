use std::env;
use std::thread;
use std::time::Duration;

use osistack::n;
use osistack::n::NetworkService;

pub fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() < 4 {
        panic!("usage: {} <interface-name> <own-network-entity-title> <destination-host> [<host-name> <mac-address>]", args[0]);
    }

    let interface_name: &str;
    let dest_host: &str;
    let network_entity_title: &str;
    let mut hosts: Vec<(&str, &str)> = vec![];   // name, MAC address

    interface_name = &args[1];
    network_entity_title = &args[2];
    dest_host = &args[3];

    let mut next_host: &str = r"";
    let mut next_macaddr: &str;
    for i in 4..args.len() {
        if next_host == "" {
            next_host = &args[i].as_str();
        } else {
            // save value
            next_macaddr = &args[i].as_str();
            // submit
            hosts.push((next_host, next_macaddr));
            // clear
            next_host = "";
        }
    }

    // set up network
    let (mut sn, mut ns) = osistack::new(interface_name, network_entity_title, hosts);

    // application logic

    // send request to other host
    let qos = n::Qos{};
    //let source_nsap = ns.get_serviced_nsap().expect("failed to get own serviced NSAP");
    //TODO fix ^ 2nd borrow, Rust's borrow checker cannot look into functions which fields they actually lock
    loop {
        //println!("echo request from {}:", source_nsap.to_string());
        /*
        ns.n_unitdata_request(
            dest_host,  //TODO change to proper, which is NSAP address - there is no echo service on DL layer
            &qos,
            r"test".as_bytes()
        );
        */
        let source_nsap = ns.echo_request(
            Some(dest_host.to_owned()), //TODO optimize clone
            None,
            Some(0),    //TODO just to avoid 2nd borrow on whole of ns
            None,
            &qos
        );

        thread::sleep(Duration::from_secs(2));
    }
}