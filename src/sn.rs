use advmac::MacAddr6;
use afpacket::sync::RawPacketStream;

pub mod ethernet;

pub const ETHER_TYPE_CLNP: u16 = 0x8872;  // as per https://datatracker.ietf.org/doc/html/draft-kaplan-isis-ext-eth-ip-clns-2-00

// is a subnetwork-dependent QoS code, different from the OSI NS QoS codes
// TODO is ^ true?
pub struct Qos {}

pub trait SubnetworkService {
    fn new(socket: RawPacketStream) -> Self where Self: Sized;
    fn sn_unitdata_request(&mut self,
        sn_source_address: MacAddr6,
        sn_destination_address: MacAddr6,
        sn_quality_of_service: Qos,
        sn_userdata: &crate::n::clnp::Pdu
    );
    fn flush(&mut self);
}