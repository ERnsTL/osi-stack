use std::env;

pub fn main() {
    //println!("{}", add(1, 1));

    let args: Vec<_> = env::args().collect();
    if args.len() < 3 {
        panic!("usage: {} <interface-name> <destination-host> [<host-name> <mac-address>]", args[0]);
    }

    let interface_name: &str;
    let dest_host: &str;
    let mut hosts: Vec<(&str, &str)> = vec![];   // name, MAC address

    interface_name = &args[1];
    dest_host = &args[2];

    let mut next_host: &str = r"";
    let mut next_macaddr: &str;
    for i in 3..args.len() {
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

    // engage
    osistack::new_interface2(interface_name, dest_host, hosts);
}

// TODO add protocol server or should each application create its own interface?