use advmac::MacAddr6;
use std::env;

pub fn main() {
    //println!("{}", add(1, 1));

    let args: Vec<_> = env::args().collect();
    let my_macaddr: Option<MacAddr6>;
    let dest_macaddr: Option<MacAddr6>;
    match args.len() {
        3 => {
            my_macaddr = Some(osistack::parse_macaddr(&args[1]).expect("could not parse own MAC address"));
            dest_macaddr = Some(osistack::parse_macaddr(&args[2]).expect("could not parse destination MAC address"));
        },
        2 => {
            my_macaddr = Some(osistack::parse_macaddr(&args[1]).expect("could not parse own MAC address"));
            dest_macaddr = None;
        },
        _ => {
            my_macaddr = None;
            dest_macaddr = None;
        }
    }

    // set MAC address from env
    osistack::new_interface(my_macaddr, dest_macaddr);
}

// TODO add protocol server or should each application create its own interface?