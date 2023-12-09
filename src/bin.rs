use advmac::MacAddr6;
use std::env;

pub fn main() {
    //println!("{}", add(1, 1));

    let args: Vec<_> = env::args().collect();
    let interface_name: Option<&str>;
    let dest_macaddr: Option<MacAddr6>;
    let my_macaddr: Option<MacAddr6>;
    match args.len() {
        4 => {
            interface_name = Some(&args[1]);
            dest_macaddr = Some(osistack::parse_macaddr(&args[2]).expect("could not parse destination MAC address"));
            my_macaddr = Some(osistack::parse_macaddr(&args[3]).expect("could not parse own MAC address"));
        },
        3 => {
            interface_name = Some(&args[1]);
            dest_macaddr = Some(osistack::parse_macaddr(&args[2]).expect("could not parse destination MAC address"));
            my_macaddr = None;
        },
        2 => {
            interface_name = Some(&args[1]);
            dest_macaddr = None;
            my_macaddr = None;
        },
        _ => {
            interface_name = None;
            dest_macaddr = None;
            my_macaddr = None;
        }
    }

    // set MAC address from env
    //osistack::new_interface(my_macaddr, dest_macaddr);
    osistack::new_interface2(interface_name.expect("no interface name given"), dest_macaddr);
}

// TODO add protocol server or should each application create its own interface?