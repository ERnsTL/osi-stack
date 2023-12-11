use std::{io::{Read, Write}, collections::HashMap};
use advmac::MacAddr6;
use afpacket::sync::RawPacketStream;
use etherparse::Ethernet2Header;

use super::{Nsap, Qos};

pub fn parse_macaddr(instr: &str) -> Result<MacAddr6, advmac::ParseError> {
    MacAddr6::parse_str(instr)
}

#[derive(Debug)]
enum NClnpPdu<'a> {
    Inactive { fixed_mini: NFixedPartMiniForInactive<'a>, data: NDataPart<'a> },
    NDataPDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, seg: Option<NSegmentationPart<'a>>, opt: Option<NOptionsPart<'a>>, discard: Option<NReasonForDiscardPart<'a>>, data: Option<NDataPart<'a>>},
    // no segmentation, but reason for discard is mandatory
    ErrorReportPDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, op: Option<NOptionsPart<'a>>, discard: NReasonForDiscardPart<'a>, data: Option<NDataPart<'a>> },
    // these are the same as DataPDU / DT PDU
    EchoRequestPDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, seg: Option<NSegmentationPart<'a>>, opt: Option<NOptionsPart<'a>>, discard: Option<NReasonForDiscardPart<'a>>, data: Option<NDataPart<'a>> },
    EchoResponsePDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, seg: Option<NSegmentationPart<'a>>, opt: Option<NOptionsPart<'a>>, discard: Option<NReasonForDiscardPart<'a>>, data: Option<NDataPart<'a>> },
    MulticastDataPDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, seg: Option<NSegmentationPart<'a>>, opt: Option<NOptionsPart<'a>>, discard: Option<NReasonForDiscardPart<'a>>, data: Option<NDataPart<'a>> }
}

#[derive(Debug)]
struct NFixedPartMiniForInactive<'a> {
    network_layer_protocol_identifier: &'a u8
}

// TODO was not possible to have it as an enum and match on it, comparing to u8
const NETWORK_LAYER_PROTOCOL_IDENTIFIER_CLNP_FULL: u8 = 0b1000_0001;
const NETWORK_LAYER_PROTOCOL_IDENTIFIER_CLNP_INACTIVE: u8 = 0b0000_0000;

#[derive(Debug)]
struct NFixedPart<'a> {
    network_layer_protocol_identifier: &'a u8,
    length_indicator: &'a u8,
    version_protocol_id_extension: &'a u8,
    lifetime: &'a u8,
    /// 0 = not permitted, no segmentation part present in PDU, non-segmenting protocol subset in use
    /// 1 = permitted, segmentation part shall be present in PDU, full protocol is in use
    sp_segmentation_permitted: bool,   // 1 bit
    ms_more_segments: bool,   // 1 bit
    er_error_report: bool,  // 1 bit
    type_: bool, // 5 bits
    /// contains ^ sub-bit values
    octet5: &'a u8,
    segment_length: &'a u16,
    checksum: &'a u16
}

enum SpSegmentationPermittedBit {
    ONE = 0b1000_0000,
    ZERO = 0b0000_0000
}

enum MsMoreSegmentsBit {
    ONE = 0b0100_0000,
    ZERO = 0b0000_0000
}

enum ErErrorReportBit {
    ONE = 0b0010_0000,
    ZERO = 0b0000_0000
}

impl NFixedPart<'_> {
    /// Return the bits for octet 5 of the fixed part of the NPDU
    fn compose_octet5(sp_segmentation_permitted: SpSegmentationPermittedBit,
        ms_more_segments: MsMoreSegmentsBit,
        er_error_report: ErErrorReportBit,
        type_: u8
    ) -> Option<u8>  {
        if type_ >= 32 {
            // only have 5 bits of space (0 to 31)
            return None;
        }
        return Some(sp_segmentation_permitted as u8 | ms_more_segments as u8 | er_error_report as u8| type_);
    }

    /// Return the bits for octet 5 of the fixed part of the NPDU
    fn compose_octet5_unchecked(sp_segmentation_permitted: SpSegmentationPermittedBit,
        ms_more_segments: MsMoreSegmentsBit,
        er_error_report: ErErrorReportBit,
        type_: u8
    ) -> u8  {
        // simply overwrites any data in bits 1,2,3 if number in type uses more than 5 bits
        return sp_segmentation_permitted as u8 | ms_more_segments as u8 | er_error_report as u8| type_;
    }
}

#[derive(Debug)]
struct NAddressPart<'a> {
    destination_address_length_indicator: &'a u8,
    destination_address: &'a [u8],
    source_address_length_indicator: &'a u8,
    source_address: &'a [u8]
}

#[derive(Debug)]
struct NSegmentationPart<'a> {
    data_unit_identifier: &'a u16,
    segment_offset: &'a u16,
    total_length: &'a u16
}

#[derive(Debug)]
struct NOptionsPart<'a> {
    params: &'a [NParameter<'a>]
}

/// only contained in NOptionsPart
//TODO decomposition of these parameters
#[derive(Debug)]
struct NParameter<'a> {
    parameter_code: &'a u8,
    parameter_length: &'a u8,
    parameter_value: &'a [u8],
}

#[derive(Debug)]
struct NReasonForDiscardPart<'a> {
    /// has format of a parameter from the options part
    param: &'a NParameter<'a>   //TODO enforce that here only parameter code "1100 0001" is allowed
}

#[derive(Debug)]
struct NDataPart<'a> {
    data: &'a [u8]
}

impl NClnpPdu<'_> {
    fn into_buf(&self, buffer: &mut [u8]) -> usize {
        //TODO check if given buffer is really < MTU
        match self {
            Self::Inactive{fixed_mini, data} => {
                buffer[0] = *fixed_mini.network_layer_protocol_identifier as u8;
                //TOD optimize
                for i in 0..data.data.len() {
                    buffer[i+1] = data.data[i];
                }
                return 1 + data.data.len();
            },
            _ => { todo!(); }
        }
        //matches!(self, Self::Inactive { .. })
    }

    fn from_buf(buffer: &[u8]) -> NClnpPdu {
        match buffer[0] {
            NETWORK_LAYER_PROTOCOL_IDENTIFIER_CLNP_FULL => {
                todo!();
            },
            NETWORK_LAYER_PROTOCOL_IDENTIFIER_CLNP_INACTIVE => {
                NClnpPdu::Inactive {
                    fixed_mini: NFixedPartMiniForInactive { network_layer_protocol_identifier: &buffer[0] },
                    data: NDataPart { data: &buffer[1..buffer.len()] }  //TODO note, we dont really know how long the data part is at this point
                }
            }
            _ => {
                todo!();
            }
        }
    }
}

pub(crate) struct Service {
    // internal state
    serviced_nsaps: Vec<Nsap>,
    known_hosts: HashMap<String, Nsap>,

    // underlying data link service
    //TODO kind of - package that
    socket: RawPacketStream,
    buffer: [u8; 1500],
}

impl super::NetworkService for Service {
    fn new(socket: RawPacketStream) -> Service {
        Service {
            socket: socket,
            buffer: [0u8; 1500],
            serviced_nsaps: vec![],
            known_hosts: HashMap::new(),
        }
    }

    // TODO serviced NSAP
    // TODO fix parameters
    fn add_serviced_nsap(&mut self, authority: u16, area: u16, sub_area: u16, remainder: MacAddr6) {
        self.serviced_nsaps.push(Nsap {
            authority: authority,
            area: area,
            sub_area: sub_area,
            local_address: remainder,
        })
    }

    // TODO serviced NSAP in subnet according to "expected services of subnet network service" or so (?)
    fn add_serviced_subnet_nsap(&mut self, net: u16, sub_net: u16, macaddr: MacAddr6) {
        self.add_serviced_nsap(49, net, sub_net, macaddr);
    }

    //TODO quick version - implement proper name lookup
    fn resolve_nsap(&self, system_title: &str) -> Option<&Nsap> {
        if let Some(address) = self.known_hosts.get(system_title) {
            return Some(address);
        } else {
            return None;
        }
    }

    //TODO quick version - implement proper name lookup
    //TODO currently we use MAC access for "NSAP"
    fn add_known_host(&mut self, system_title: String, nsap: &str) {
        self.known_hosts.insert(system_title, Nsap {
            authority: 49,
            area: 1,
            sub_area: 1,
            local_address: parse_macaddr(nsap).expect("could not parse mac address"),
        });
    }

    //TODO there are/can be multiple
    fn get_serviced_nsap(&mut self) -> Option<&Nsap> {
        return self.serviced_nsaps.get(0);
    }

    fn n_unitdata_request(
        &mut self,
        ns_destination_title: &str,
        ns_quality_of_service: &Qos,
        ns_userdata: &[u8]
    ) {
        let get_serviced_nsap = self.get_serviced_nsap().expect("no serviced NSAPs").clone();
        let dest_nsap = self.resolve_nsap(ns_destination_title).expect("cannot resolve destination host").clone();
        self.n_unitdata_request_internal(
            &get_serviced_nsap,
            &dest_nsap,
            &ns_quality_of_service,
            ns_userdata
        );
    }

    //TODO implement properly (PDU decomposition)
    fn n_unitdata_indication(
        ns_source_address: MacAddr6,
        ns_destination_address: MacAddr6,
        ns_quality_of_service: &Qos,
        ns_userdata: &[u8]
    ) {
        println!("got CLNP packet: {:?}", NClnpPdu::from_buf(ns_userdata));
    }
}

impl Service {
    // TODO only inactive implemented
    fn n_unitdata_request_internal(
        &mut self,
        ns_source_address: &Nsap,
        ns_destination_address: &Nsap,
        ns_quality_of_service: &Qos,
        ns_userdata: &[u8]
    ) {
        // check if we are on same Ethernet broadcast domain as destination
        if can_use_inactive_subset(ns_source_address, ns_destination_address) {
            // compose PDU(s)
            let pdus = pdu_composition(true, ns_source_address, ns_destination_address, ns_quality_of_service, ns_userdata);
            for pdu in pdus {
                // send DLPDU (Ethernet frame header)
                let pkt_out = Ethernet2Header{
                    destination: ns_destination_address.local_address.to_array(),
                    source: ns_source_address.local_address.to_array(),
                    ether_type: crate::n::ETHER_TYPE_CLNP,
                };
                println!("writing DLPDU...");
                let mut remainder = pkt_out.write_to_slice(&mut self.buffer).expect("failed writing DLPDU into buffer");
                //pkt_out.write(&mut self.socket).expect("failed writing frame into socket");

                // send NPDU (CLNP PDU)
                println!("writing NPDU...");
                let bytes = pdu.into_buf(&mut remainder);
                self.socket.write(&self.buffer[0..bytes + 14]).expect("could not write buffer into socket");    //TODO +14 is not cleanly abtracted

                println!("flushing DL...");
                self.socket.flush().expect("failed to flush socket");
            }
            return;
        }
        todo!();
    }
}

//TODO
fn can_use_inactive_subset(ns_source_address: &Nsap, ns_destination_address: &Nsap) -> bool {
    // TODO check if on same subnetwork (AKA in same Ethernet segment)
    true
}

// 6.1
// TODO WIP
// TODO optimize - this function allocates CLNP PDUs for every call
fn pdu_composition<'a>(inactive: bool, ns_source_address: &'a Nsap, ns_destination_address: &'a Nsap, ns_quality_of_service: &'a Qos, ns_userdata: &'a [u8]) -> Vec<NClnpPdu<'a>> {
    if inactive {
        return vec![NClnpPdu::Inactive {
            fixed_mini: NFixedPartMiniForInactive { network_layer_protocol_identifier: &NETWORK_LAYER_PROTOCOL_IDENTIFIER_CLNP_INACTIVE },
            data: NDataPart { data: ns_userdata }
        }]
    } else {
        todo!();
    }
}

enum HeaderFormatAnalysisResult {
    TooShortTooIdentify,
    FullProtocol,
    InactiveProtocol,
    UnknownProtocol,
}

// 6.3
// TODO only what is neccessary for inactive protocol subset
fn header_format_analysis(packet: &[u8]) -> HeaderFormatAnalysisResult {
    if packet.len() < 1 {
        return HeaderFormatAnalysisResult::TooShortTooIdentify;
    }
    match packet[0] {
        NETWORK_LAYER_PROTOCOL_IDENTIFIER_CLNP_FULL => HeaderFormatAnalysisResult::FullProtocol,
        NETWORK_LAYER_PROTOCOL_IDENTIFIER_CLNP_INACTIVE => HeaderFormatAnalysisResult::InactiveProtocol,
        //TODO check after ^ The Network entity in this case determines that either the Subnetwork Point of Attachment
        //(SNPA) address encoded as NPAI in the supporting subnetwork protocol (see 8.1) corresponds directly to an NSAP
        //address serviced by this Network entity, or that an error has occurred.
        _ => HeaderFormatAnalysisResult::UnknownProtocol
    }
}