pub mod clnp;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::prelude::*;

use advmac::MacAddr6;

use crate::dl::SNUnitDataRequest;
use crate::n::clnp::NOptionsPart;

pub trait NetworkService<'a> {
    fn new(
        network_entity_title: &'a str,
        sn_service_to: rtrb::Producer<SNUnitDataRequest>,
        sn_service_from: rtrb::Consumer<NUnitDataIndication>,
    ) -> Self;
    fn add_serviced_nsap(&mut self, authority: u16, area: u16, sub_area: u16, remainder: MacAddr6);
    fn add_serviced_subnet_nsap(&mut self, net: u16, sub_net: u16, macaddr: MacAddr6);
    fn resolve_nsap(&self, system_title: &str) -> Option<&Nsap>;
    fn add_known_host(&mut self, system_title: String, nsap: &str);
    fn get_serviced_nsap(&self) -> Option<&Nsap>;
    /// called by TS
    fn n_unitdata_request(&mut self,
        ns_destination_title: &str,
        ns_quality_of_service: &'a Qos,
        ns_userdata: &'a [u8]
    );
    /// called by SN
    fn n_unitdata_indication(//&self,
        sn_service_to: &mut rtrb::Producer<SNUnitDataRequest>,
        echo_request_correlation_table: Arc<Mutex<HashMap<u16, DateTime<Utc>>>>,
        // actual parameters
        ns_source_address: MacAddr6,
        ns_destination_address: MacAddr6,
        ns_quality_of_service: &Qos,
        ns_userdata: &[u8]
    );
    /// X.233 6.19 Echo request function
    /// called by application
    fn echo_request(&mut self,
        destination_title: Option<String>,
        destination_nsap: Option<&Nsap>,
        source_address_index: Option<usize>,
        options: Option<NOptionsPart>,
        ns_quality_of_service: &Qos
    ); //TODO clunky to return the sending Nsap, and even that is not possible inside echo_request() this should be known beforehand, but alas, Rust's no 2nd borrow on ns variable
    fn run(&mut self);
    //TODO
    fn pdu_composition(&self, inactive: bool, ns_source_address: &'a Nsap, ns_destination_address: &'a Nsap, ns_quality_of_service: &'a Qos, ns_userdata: &'a [u8]) -> Vec<crate::n::clnp::Pdu<'a>>;
}

//TODO implement full NSAP
#[derive(Clone, Debug)]
pub struct Nsap {
    authority: u16, // 49 = local network
    area: u16,  //net (?)
    sub_area: u16,  //subnet (?)
    local_address: MacAddr6,    //TODO fix - this is of course not correct
}

impl Nsap {
    fn new_from_network_entity_title(network_entity_title: String) -> Nsap {
        todo!()
    }

    fn len(&self) -> usize {
        return 2+2+2+6;
    }

    //TODO optimize
    fn to_u8(&self) -> Vec<u8> {
        //TODO optimize https://stackoverflow.com/questions/40154150/how-do-i-concatenate-two-slices-in-rust
        //let mut one = [self.authority.to_ne_bytes(), self.area.to_ne_bytes(), self.sub_area.to_ne_bytes()].concat();
        //one.extend_from_slice(self.local_address.as_slice());
        //return one;

        let mut buffer = Vec::with_capacity(self.len());
        /* NOTE: with capacity we cannot assign buffer[..] directly, because len <= cap. so we have to push()
        as long as len <= cap there will be no allocations.
        have to push each byte individually because if we push [u8;2] as first one, then it only wants further [u8;2] */
        buffer.push(self.authority.to_be_bytes()[0]);
        buffer.push(self.authority.to_be_bytes()[1]);
        buffer.push(self.area.to_be_bytes()[0]);
        buffer.push(self.area.to_be_bytes()[1]);
        buffer.push(self.sub_area.to_be_bytes()[0]);
        buffer.push(self.sub_area.to_be_bytes()[1]);
        buffer.push(self.local_address.to_array()[0]);
        buffer.push(self.local_address.to_array()[1]);
        buffer.push(self.local_address.to_array()[2]);
        buffer.push(self.local_address.to_array()[3]);
        buffer.push(self.local_address.to_array()[4]);
        buffer.push(self.local_address.to_array()[5]);
        return buffer;
    }
}

impl ToString for Nsap {
    fn to_string(&self) -> String {
        format!("{}.{}.{}.{}", self.authority, self.area, self.sub_area, self.local_address.format_string(advmac::MacAddrFormat::Hexadecimal).to_lowercase())
    }
}

#[derive(Debug)]
pub struct Qos {
    //TODO
}

#[derive(Debug)]
pub struct NUnitDataIndication {
    pub ns_source_address: MacAddr6,
    pub ns_destination_address: MacAddr6,
    pub ns_quality_of_service: crate::n::Qos,
    pub ns_userdata: Vec<u8>    //TODO optimize copying?
}