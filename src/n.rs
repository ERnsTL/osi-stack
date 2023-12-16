pub mod clnp;

use advmac::MacAddr6;
use crate::n::clnp::NOptionsPart;

pub trait NetworkService<'a> {
    fn new(sn_service: crate::dl::ethernet::Service, network_entity_title: &'a str) -> Self;   // consume the SubnetService
    //TODO this ^ is not nicely abstracted, should allow all implementors of SubnetService, but then compiler suggests dyn, which has runtime cost :-(
    fn add_serviced_nsap(&mut self, authority: u16, area: u16, sub_area: u16, remainder: MacAddr6);
    fn add_serviced_subnet_nsap(&mut self, net: u16, sub_net: u16, macaddr: MacAddr6);
    fn resolve_nsap(&self, system_title: &str) -> Option<&Nsap>;
    fn add_known_host(&mut self, system_title: String, nsap: &str);
    fn get_serviced_nsap(&self) -> Option<&Nsap>;
    fn n_unitdata_request(&mut self,
        ns_destination_title: &str,
        ns_quality_of_service: &Qos,
        ns_userdata: &[u8]
    );
    fn n_unitdata_indication(
        ns_source_address: MacAddr6,
        ns_destination_address: MacAddr6,
        ns_quality_of_service: &Qos,
        ns_userdata: &[u8]
    );
    // X.233 6.19 Echo request function
    fn echo_request(&mut self,
        destination_title: Option<String>,
        destination_nsap: Option<&Nsap>,
        source_address_index: Option<usize>,
        options: Option<NOptionsPart>,
        ns_quality_of_service: &Qos
    );
}

//TODO implement full NSAP
#[derive(Clone)]
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

    //TODO optimize
    fn as_u8(&self) -> Vec<u8> {
        //TODO optimize https://stackoverflow.com/questions/40154150/how-do-i-concatenate-two-slices-in-rust
        let mut one = [self.authority.to_ne_bytes(), self.area.to_ne_bytes(), self.sub_area.to_ne_bytes()].concat();
        one.extend_from_slice(self.local_address.as_slice());
        return one;
    }
}

pub struct Qos {
    //TODO
}