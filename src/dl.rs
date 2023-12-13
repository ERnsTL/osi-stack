use advmac::MacAddr6;
use afpacket::sync::RawPacketStream;

pub mod ethernet;

pub const ETHER_TYPE_CLNP: u16 = 0x8872;  // as per https://datatracker.ietf.org/doc/html/draft-kaplan-isis-ext-eth-ip-clns-2-00

// NOTE: According to X.233 5.5 "Underlying service assumed by the protocol", the CLNP can run on a data link or a real subnet, 
// which both operate on the Data Link Layer. This is basically where telecom technology and computer networking technology meet
// and are abstracted over.
// Currently, the data structures and naming are in place for Subnetwork Service, SNPA address, Subnetwork address etc. but when
// a data link is to be implemented, this has to be adjusted for somehow so that the Network Layer protocols can handle both types.

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
    fn sn_unitdata_indication_reader(&mut self);
}