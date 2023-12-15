use std::collections::HashMap;
use advmac::MacAddr6;

use super::{Nsap, Qos};
use crate::dl::{SubnetworkService, self};

pub fn parse_macaddr(instr: &str) -> Result<MacAddr6, advmac::ParseError> {
    MacAddr6::parse_str(instr)
}

#[derive(Debug)]
pub enum Pdu<'a> {
    Inactive { fixed_mini: NFixedPartMiniForInactive<'a>, data: NDataPart<'a> },
    NDataPDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, seg: Option<NSegmentationPart<'a>>, opt: Option<NOptionsPart<'a>>, discard: Option<NReasonForDiscardPart<'a>>, data: Option<NDataPart<'a>>},
    // no segmentation, but reason for discard is mandatory
    ErrorReportPDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, op: Option<NOptionsPart<'a>>, discard: NReasonForDiscardPart<'a>, data: Option<NDataPart<'a>> },
    // these are the same as DataPDU / DT PDU
    EchoRequestPDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, seg: Option<NSegmentationPart<'a>>, opt: Option<NOptionsPart<'a>>, discard: Option<NReasonForDiscardPart<'a>>, data: Option<NDataPart<'a>> },
    EchoResponsePDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, seg: Option<NSegmentationPart<'a>>, opt: Option<NOptionsPart<'a>>, discard: Option<NReasonForDiscardPart<'a>>, data: Option<NDataPart<'a>> },
    MulticastDataPDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, seg: Option<NSegmentationPart<'a>>, opt: Option<NOptionsPart<'a>>, discard: Option<NReasonForDiscardPart<'a>>, data: Option<NDataPart<'a>> }
}

const VERSION_PROTOCOL_ID_EXTENSION_1: u8 = 0b0000_0001;

/// actually 5 bits, so bits 8,7,6 are 0
const TYPE_DT_PDU: u8 = 0b00011100;     // data
const TYPE_MD_PDU: u8 = 0b00011101;     // multicast data
const TYPE_ER_PDU: u8 = 0b00000001;     // error report
const TYPE_ERQ_PDU: u8 = 0b00011110;    // echo request
const TYPE_ERP_PDU: u8 = 0b00011111;    // echo response

impl<'a> Pdu<'_> {
    fn new_echo_request(
        sp_segmentation_permitted: bool,
        source_address: &Nsap,
        destination_address: &Nsap,
        options: &Option<NOptionsPart>
    ) -> Pdu<'a> {
        // compose echo response PDU to be put into the echo request PDU's data part
        let erp_pdu = Pdu::EchoResponsePDU {
            fixed: NFixedPart {
                network_layer_protocol_identifier: &NETWORK_LAYER_PROTOCOL_IDENTIFIER_CLNP_FULL,
                length_indicator: &mut 0,    // will be filled
                version_protocol_id_extension: &VERSION_PROTOCOL_ID_EXTENSION_1,
                lifetime: &(((1000*10)/500) as u8),   // TODO 10 seconds  //TODO optimize converts from i32 to u8 
                sp_segmentation_permitted: false,   // setting to false for now TODO
                ms_more_segments: false,    // X.233 6.19 e) value of zero
                er_error_report: false,
                type_: &TYPE_ERP_PDU,
                octet5: &0,  // to be filled
                segment_length: &0,  // an invalid value per 6.19 e)
                checksum: &0,    // an invalid value per 6.19 e)
            },
            addr: NAddressPart {
                destination_address_length_indicator: todo!(),
                destination_address: source_address.as_u8().as_slice(),    // TODO X.233 6.19 e) demands a "valid value" meaning the return address?
                source_address_length_indicator: todo!(),
                source_address: destination_address.as_u8().as_slice(),    // TODO X.233 6.19 e) demands a "valid value" meaning the return address?
            },
            seg: None,  // only if the sp_segmentation_permitted bit is set, shall this part be present X.233 6.19 e)
            opt: None,  // may be present and contain any options from X.233 7.5
            discard: None,
            data: Some(NDataPart {
                data: &r"correlation number for ping".as_bytes()   //TODO
            })
        };

        if let Pdu::EchoResponsePDU { fixed, addr, opt, seg, discard, ..} = erp_pdu {
            // set octet 5
            *fixed.octet5 = NFixedPart::compose_octet5_unchecked(
                //TODO dont know of these conversions are really needed
                if fixed.sp_segmentation_permitted { SpSegmentationPermittedBit::ONE } else { SpSegmentationPermittedBit::ZERO },
                if fixed.ms_more_segments { MsMoreSegmentsBit::ONE } else { MsMoreSegmentsBit::ZERO },
                if fixed.er_error_report { ErErrorReportBit::ONE } else { ErErrorReportBit::ZERO },
                *fixed.type_
            );

            // set length indicators
            Pdu::compose_length_indicators(&mut fixed, &mut addr, &seg, &opt);
        }

        // compose echo request PDU
        let mut buffer: [u8; 64];
        let data_num_bytes = erp_pdu.into_buf(&mut buffer); //TODO optimize useless putting into buffer

        // the actual echo request PDU
        let erq_pdu = Pdu::EchoRequestPDU {
            fixed: NFixedPart {
                network_layer_protocol_identifier: &NETWORK_LAYER_PROTOCOL_IDENTIFIER_CLNP_FULL,
                length_indicator: &mut 0,    // will be filled
                version_protocol_id_extension: &VERSION_PROTOCOL_ID_EXTENSION_1,
                lifetime: &(((1000*10)/500) as u8),   //TODO 10 seconds  //TODO optimize converts from i32 to u8 
                sp_segmentation_permitted: false,   // setting to false for now TODO - depending on network service setting / protocol subset
                ms_more_segments: false,   // will be filled
                er_error_report: false,
                type_: &TYPE_ERQ_PDU,
                octet5: &0,  // will be filled
                segment_length: &0,  // an invalid value per 6.19 e)
                checksum: &0,    // an invalid value per 6.19 e)
            },
            addr: NAddressPart {
                destination_address_length_indicator: todo!(),
                destination_address: destination_address.as_u8().as_slice(),   // X.233 6.19 b) TODO implement fully
                source_address_length_indicator: todo!(),
                source_address: source_address.as_u8().as_slice(),    // X.233 6.19 b) TODO implement fully
            },
            seg: None,  // only if the sp_segmentation_permitted bit is set, shall this part be present X.233 6.19 e)
            opt: None,  // may be present and contain any options from X.233 7.5
            discard: None,
            data: Some(NDataPart {
                data: &buffer[0..data_num_bytes],   // the echo request PDU
            })
        };

        if let Pdu::EchoRequestPDU { fixed, addr, opt, seg, discard, ..} = erq_pdu {
            // set octet 5
            *fixed.octet5 = NFixedPart::compose_octet5_unchecked(
                //TODO dont know of these conversions are really needed
                if fixed.sp_segmentation_permitted { SpSegmentationPermittedBit::ONE } else { SpSegmentationPermittedBit::ZERO },
                if fixed.ms_more_segments { MsMoreSegmentsBit::ONE } else { MsMoreSegmentsBit::ZERO },
                if fixed.er_error_report { ErErrorReportBit::ONE } else { ErErrorReportBit::ZERO },
                *fixed.type_
            );

            // set length indicators
            Pdu::compose_length_indicators(&mut fixed, &mut addr, &seg, &opt);
        }

        return erq_pdu;
    }

    //TODO is "reason for discard" part of the header, thus the header length - or only for error report PDU?
    fn compose_length_indicators(fixed: &mut NFixedPart<'_>, addr: &mut NAddressPart<'_>, seg: &Option<NSegmentationPart<'_>>, opt: &Option<NOptionsPart<'_>>) {
        // address part length indicators
        *addr.destination_address_length_indicator = addr.destination_address.len() as u8;
        *addr.source_address_length_indicator = addr.source_address.len() as u8;
        // fixed part length indicator (overall header)
        *fixed.length_indicator = 
            // fixed part
            (1+1+1+1+1+2+2) +
            // address part
            (*addr.destination_address_length_indicator + *addr.source_address_length_indicator) +
            // segmentation part
            (if seg.is_some() { 2+2+2 } else { 0 }) +
            // options part
            (if let Some(opt_inner) = opt {
                (opt_inner.params.len() * (1+1+1)) as u8
            } else {
                0 as u8
            });
        return;
    }
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
    length_indicator: &'a mut u8,
    version_protocol_id_extension: &'a u8,
    lifetime: &'a u8,
    /// 0 = not permitted, no segmentation part present in PDU, non-segmenting protocol subset in use
    /// 1 = permitted, segmentation part shall be present in PDU, full protocol is in use
    sp_segmentation_permitted: bool,   // 1 bit
    ms_more_segments: bool,   // 1 bit
    er_error_report: bool,  // 1 bit
    type_: &'a u8, // 5 bits only!
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
    destination_address_length_indicator: &'a mut u8,
    destination_address: &'a [u8],
    source_address_length_indicator: &'a mut u8,
    source_address: &'a [u8]
}

#[derive(Debug)]
struct NSegmentationPart<'a> {
    data_unit_identifier: &'a u16,
    segment_offset: &'a u16,
    total_length: &'a u16
}

#[derive(Debug)]
pub struct NOptionsPart<'a> {
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

impl Pdu<'_> {
    pub fn into_buf(&self, buffer: &mut [u8]) -> usize {
        //TODO check if given buffer is really < MTU
        match self {
            Self::Inactive{fixed_mini, data} => {
                buffer[0] = *fixed_mini.network_layer_protocol_identifier as u8;
                //TODO optimize
                for i in 0..data.data.len() {
                    buffer[i+1] = data.data[i];
                }
                return 1 + data.data.len();
            },
            _ => { todo!(); }
        }
        //matches!(self, Self::Inactive { .. })
    }

    pub fn from_buf(buffer: &[u8]) -> Pdu {
        match buffer[0] {
            NETWORK_LAYER_PROTOCOL_IDENTIFIER_CLNP_FULL => {
                todo!();
            },
            NETWORK_LAYER_PROTOCOL_IDENTIFIER_CLNP_INACTIVE => {
                Pdu::Inactive {
                    fixed_mini: NFixedPartMiniForInactive { network_layer_protocol_identifier: &buffer[0] },
                    data: NDataPart { data: &buffer[1..buffer.len()] }  //TODO note, we dont really know how long the data part is at this point
                }
            }
            _ => {
                todo!();
            }
        }
    }

    //TODO implement and use in Pdu::new_echo_request()
    pub fn as_slice(&self) -> &[u8] {
        todo!();
    }
}

pub struct Service<'a> {
    // internal state
    serviced_nsaps: Vec<Nsap>,
    known_hosts: HashMap<String, Nsap>,
    network_entity_title: &'a str,   // own title

    // underlying service assumed by the protocol = subnet service on data link layer
    sn_service: dl::ethernet::Service,
}

impl<'a> super::NetworkService<'a> for Service<'a> {
    fn new(sn_service: dl::ethernet::Service, network_entity_title: &'a str) -> Service<'a> {
        Service {
            sn_service: sn_service,
            serviced_nsaps: vec![],
            known_hosts: HashMap::new(),
            network_entity_title: network_entity_title,
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
    fn get_serviced_nsap(&self) -> Option<&Nsap> {
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
        println!("got CLNP packet: {:?}", Pdu::from_buf(ns_userdata));
    }

    // X.233 6.19 Echo request function
    //TODO implement correctly to the point
    fn echo_request(&mut self,
        destination_title: Option<String>,
        destination_nsap: Option<Nsap>,
        source_address_index: Option<usize>,
        options: Option<NOptionsPart>,
        quality_of_service: &crate::n::Qos
    ) {
        // destination
        let destination_address;
        if destination_title.is_some() {
            // convert to NSAP
            destination_address = Nsap::new_from_network_entity_title(destination_title.unwrap());
        } else if destination_nsap.is_some() {
            destination_address = destination_nsap.unwrap();
        } else {
            // error
            todo!();
        }

        // prepare source
        let source_address: &Nsap;
        if source_address_index.is_some() {
            // TODO actually use index and get from self.serviced_nsaps as there might exist multiple
            source_address = &self.get_serviced_nsap().expect("failed to get servied NSAP");
        } else {
            source_address = &self.resolve_nsap(self.network_entity_title).expect("failed to get own NSAP");
        }

        // check length
        //TODO 6.19 d)

        // compose ERQ PDU
        let erq_pdu = Pdu::new_echo_request(
            false,   //TODO implement non-segmenting protocol subset properly - refer to NS.operating mode or so
            &source_address,
            &destination_address,
            &options
        );

        // send it via data link or subnetwork
        let sn_quality_of_service = dl::Qos{};  //TODO convert Network Layer QoS to Data Link Layer QoS
        self.sn_service.sn_unitdata_request(
            source_address.local_address,
            destination_address.local_address,
            sn_quality_of_service,
            &erq_pdu
        );
    }
}

impl Service<'_> {
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
                let bla = [1u8];
                self.sn_service.sn_unitdata_request(
                    ns_source_address.local_address,
                    ns_destination_address.local_address,
                    dl::Qos{},   //TODO optimize useless allocation; and no real conversion - the point of having two different QoS on DL and N layer is that the codes for QoS cloud be different
                    &pdu    //TODO not perfect abstraction, but should save us a memcpy
                );
                self.sn_service.flush();
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
fn pdu_composition<'a>(inactive: bool, ns_source_address: &'a Nsap, ns_destination_address: &'a Nsap, ns_quality_of_service: &'a Qos, ns_userdata: &'a [u8]) -> Vec<Pdu<'a>> {
    if inactive {
        return vec![Pdu::Inactive {
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