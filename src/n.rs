pub mod clnp;

use afpacket::sync::RawPacketStream;
use advmac::MacAddr6;

pub const ETHER_TYPE_CLNP: u16 = 0x8872;  // as per https://datatracker.ietf.org/doc/html/draft-kaplan-isis-ext-eth-ip-clns-2-00

pub trait NetworkService {
    fn new(socket: RawPacketStream) -> Self;
    fn add_serviced_nsap(&mut self, authority: u16, area: u16, sub_area: u16, remainder: MacAddr6);
    fn add_serviced_subnet_nsap(&mut self, net: u16, sub_net: u16, macaddr: MacAddr6);
    fn resolve_nsap(&self, system_title: &str) -> Option<&Nsap>;
    fn add_known_host(&mut self, system_title: String, nsap: &str);
    fn get_serviced_nsap(&mut self) -> Option<&Nsap>;
    fn n_unitdata_request(
        &mut self,
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

}

//TODO implement full NSAP
#[derive(Clone)]
pub(crate) struct Nsap {
    authority: u16, // 49 = local network
    area: u16,  //net (?)
    sub_area: u16,  //subnet (?)
    local_address: MacAddr6,    //TODO fix - this is of course not correct
}

pub(crate) struct Qos {
    //TODO
}