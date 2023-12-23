use advmac::MacAddr6;
use afpacket::sync::RawPacketStream;

use crate::n::NUnitDataIndication;

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

pub trait SubnetworkService<'a> {
    fn new(
        socket: RawPacketStream,
        n_service_from: rtrb::Consumer<SNUnitDataRequest>,
        n_service_to: rtrb::Producer<NUnitDataIndication>,
    ) -> Self where Self: Sized;    //TODO make network service exchangeable without requiring "dyn" (optimize)
    /// called by NS
    fn sn_unitdata_request(&mut self,
        sn_source_address: MacAddr6,
        sn_destination_address: MacAddr6,
        sn_quality_of_service: Qos,
        sn_userdata: crate::n::clnp::Pdu
    );
    /// called by NS
    fn flush(&mut self);
    /// called by run()
    fn sn_unitdata_indication(//&self,
        n_service_to: &mut rtrb::Producer<NUnitDataIndication>,
        sn_source_address: MacAddr6,
        sn_destination_address: MacAddr6,
        sn_quality_of_service: Qos,
        sn_userdata: &'a [u8]
    );
    fn run(&mut self);
}

pub struct SNUnitDataRequest {
    pub sn_source_address: MacAddr6,
    pub sn_destination_address: MacAddr6,
    pub sn_quality_of_service: Qos,
    pub sn_userdata: Vec<u8>,
}