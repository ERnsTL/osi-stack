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
    NDataPDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, seg: Option<NSegmentationPart<'a>>, opts: Option<NOptionsPart<'a>>, discard: Option<NReasonForDiscardPart<'a>>, data: Option<NDataPart<'a>>},
    // no segmentation, but reason for discard is mandatory
    ErrorReportPDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, opts: Option<NOptionsPart<'a>>, discard: NReasonForDiscardPart<'a>, data: Option<NDataPart<'a>> },
    // these are the same as DataPDU / DT PDU
    EchoRequestPDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, seg: Option<NSegmentationPart<'a>>, opts: Option<NOptionsPart<'a>>, discard: Option<NReasonForDiscardPart<'a>>, data: Option<NDataPart<'a>> },
    EchoResponsePDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, seg: Option<NSegmentationPart<'a>>, opts: Option<NOptionsPart<'a>>, discard: Option<NReasonForDiscardPart<'a>>, data: Option<NDataPart<'a>> },
    MulticastDataPDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, seg: Option<NSegmentationPart<'a>>, opts: Option<NOptionsPart<'a>>, discard: Option<NReasonForDiscardPart<'a>>, data: Option<NDataPart<'a>> }
}

const VERSION_PROTOCOL_ID_EXTENSION_1: u8 = 0b0000_0001;

/// actually 5 bits, so bits 8,7,6 are 0
const TYPE_DT_PDU: u8 = 0b00011100;     // data
const TYPE_MD_PDU: u8 = 0b00011101;     // multicast data
const TYPE_ER_PDU: u8 = 0b00000001;     // error report
const TYPE_ERQ_PDU: u8 = 0b00011110;    // echo request
const TYPE_ERP_PDU: u8 = 0b00011111;    // echo response

const CHECKSUM_INVALID_IGNORE: u16 = 0;  // X.233 7.2.9 PDU checksum and X.233 6.19 e) for Echo Request function
const SEGMENT_LENGTH_INVALID: u16 = 0;  // X.233 6.19 e) for Echo Request function

impl<'a> Pdu<'_> {
    fn new_echo_request(
        sp_segmentation_permitted: bool,    //TODO use that :-)
        source_address: &Nsap,
        destination_address: &Nsap,
        options: &Option<NOptionsPart>, //TODO use that :-)
        buffer_scratch: &'a mut [u8]    /* TODO optimize - this is horrible; 
        currently so and Pdu fields mix of & and owned values and Option<> values because 
        Pdu is used for composition (want as many & as possible) and for compositing (have 
        unknown values like length indicators, unset values and cannot put echo response PDU 
        into buffer of outer echo request PDU because of not owned buffer in this function) */
    ) -> Pdu<'a> {
        // compose echo response PDU to be put into the echo request PDU's data part
        let erp_pdu_destination_address = destination_address.to_u8();   //TODO optimize
        let erp_pdu_source_address = source_address.to_u8();
        let mut erp_pdu = Pdu::EchoResponsePDU {
            fixed: NFixedPart {
                network_layer_protocol_identifier: &NETWORK_LAYER_PROTOCOL_IDENTIFIER_CLNP_FULL,
                length_indicator: None,    // will be filled
                version_protocol_id_extension: &VERSION_PROTOCOL_ID_EXTENSION_1,
                lifetime: &(((1000*10)/500) as u8),   // TODO 10 seconds  //TODO optimize converts from i32 to u8 
                sp_segmentation_permitted: false,   // setting to false for now TODO
                ms_more_segments: false,    // X.233 6.19 e) value of zero
                er_error_report: false,
                type_: &TYPE_ERP_PDU,
                octet5: &0,  // to be filled
                segment_length: &SEGMENT_LENGTH_INVALID,  // an invalid value per 6.19 e) which should also be transmitted this way TODO -> use Option
                checksum: &CHECKSUM_INVALID_IGNORE,    // an invalid value per 6.19 e) which should also be transmitted this way TODO -> use Option
            },
            addr: NAddressPart {
                destination_address_length_indicator: None,   // will be filled later
                destination_address: erp_pdu_destination_address.clone(),    //TODO optimize clone  // TODO X.233 6.19 e) demands a "valid value" meaning the return address?
                source_address_length_indicator: None,    // will be filled later
                source_address: erp_pdu_source_address.clone(),    //TODO optimize clone  // TODO X.233 6.19 e) demands a "valid value" meaning the return address?
            },
            seg: None,  // only if the sp_segmentation_permitted bit is set, shall this part be present X.233 6.19 e)
            opts: None,  // may be present and contain any options from X.233 7.5
            discard: None,
            data: Some(NDataPart {
                data: &r"xxxxxxx".as_bytes()   //TODO should be correlation number / sequence number
            })
        };

        // compose the inner echo response PDU
        // X.233 6.19 e) for the inner Echo Response packed in Echo Request PDU, an invalid value shall be set for segment length and checksum in the fixed part
        //let mut buffer: [u8; 64] = [0; 64]; //TODO optimize allocation
        let data_num_bytes = erp_pdu.into_buf(false, buffer_scratch); //TODO optimize useless putting into buffer

        // now the outer resp. actual echo request PDU
        let erq_pdu = Pdu::EchoRequestPDU {
            fixed: NFixedPart {
                network_layer_protocol_identifier: &NETWORK_LAYER_PROTOCOL_IDENTIFIER_CLNP_FULL,
                length_indicator: None,    // will be filled
                version_protocol_id_extension: &VERSION_PROTOCOL_ID_EXTENSION_1,
                lifetime: &(((1000*10)/500) as u8),   //TODO 10 seconds  //TODO optimize converts from i32 to u8 
                sp_segmentation_permitted: false,   // setting to false for now TODO - depending on network service setting / protocol subset
                ms_more_segments: false,   // will be filled
                er_error_report: false,
                type_: &TYPE_ERQ_PDU,
                octet5: &0,  // will be filled
                segment_length: &SEGMENT_LENGTH_INVALID,  // should be filled like any other DT PDU - TODO
                checksum: &CHECKSUM_INVALID_IGNORE,    // should be filled like any other DT PDU - TODO
            },
            addr: NAddressPart {
                destination_address_length_indicator: None,   // will be filled
                destination_address: erp_pdu_destination_address,   // X.233 6.19 b) TODO implement fully
                source_address_length_indicator: None,    // will be filled
                source_address: erp_pdu_source_address,    // X.233 6.19 b) TODO implement fully
            },
            seg: None,  // only if the sp_segmentation_permitted bit is set, shall this part be present X.233 6.19 e)
            opts: None,  // may be present and contain any options from X.233 7.5
            discard: None,
            data: Some(NDataPart {
                data: &buffer_scratch[0..data_num_bytes],   // the echo request PDU per X.233 6.19 TODO
            })
        };

        return erq_pdu;
    }

    //TODO is "reason for discard" part of the header, thus the header length - or only for error report PDU?
    /* TODO According to 7.2.9 PDU checksum
    The checksum is computed on the entire PDU header. For the Data, Echo Request, and Echo Reply PDUs, this includes
    the segmentation and options parts (if present). For the Error Report PDU, this includes the reason for discard field as
    well. */
    fn get_length_indicators(fixed: &NFixedPart<'_>, addr: &NAddressPart<'_>, seg: &Option<NSegmentationPart<'_>>, opts: &Option<NOptionsPart<'_>>, data: &Option<NDataPart<'_>>) -> (u8, u16, u8, u8) {
        return (
            // fixed part

            // length indicator (overall header)
            (
                // fixed part
                (1+1+1+1+1+2+2) +
                // address part
                //(*addr.destination_address_length_indicator.unwrap() + *addr.source_address_length_indicator.unwrap()) +  //TODO currently not using the length indicators - but why
                (1 + (addr.destination_address.len() as u8) + 1 + (addr.source_address.len() as u8)) +
                // segmentation part
                (if seg.is_some() { 2+2+2 } else { 0 }) +
                // options part
                (if let Some(opts_inner) = opts {
                    opts_inner.len_bytes() as u8    //TODO optimize
                } else {
                    0 as u8
                })
            ),
            // segment length
            (
                // fixed part
                (1+1+1+1+1+2+2) +
                // address part
                //(*addr.destination_address_length_indicator.unwrap() + *addr.source_address_length_indicator.unwrap()) +  //TODO currently not using the length indicators - but why
                (1 + (addr.destination_address.len() as u16) + 1 + (addr.source_address.len() as u16)) +
                // segmentation part
                (if seg.is_some() { 2+2+2 } else { 0 }) +
                // options part
                (if let Some(opts_inner) = opts {
                    opts_inner.len_bytes() as u16    //TODO optimize
                } else {
                    0 as u16
                }) +
                //TODO optimize ^ above is duplicated
                // data part
                (if let Some(data_inner) = data {
                    data_inner.data.len() as u16    //TODO optimize
                } else {
                    0 as u16
                })
            ),

            // address part

            // destination address length indicator
            addr.destination_address.len() as u8,
            // source address length indicator
            addr.source_address.len() as u8,
        );
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
    length_indicator: Option<&'a u8>,
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
    destination_address_length_indicator: Option<&'a u8>,
    destination_address: Vec<u8>,  //TODO optimize - owned only because of Pdu::to_buf() converts Nsap to [u8] and "data is owned by current function"
    source_address_length_indicator: Option<&'a u8>,
    source_address: Vec<u8>    //TODO optimize - owned only because of Pdu::to_buf() converts Nsap to [u8] and "data is owned by current function"
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

impl NOptionsPart<'_> {
    fn len_bytes(&self) -> usize {
        let mut bytes = self.params.len()* (1+1);   // type and length
        for i in 0..self.params.len() {
            bytes += self.params[i].parameter_value.len();
        }
        return bytes;
    }
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
    /// serialize struct into buffer, calculating a few fields along the way
    /// Sender decides on the checksum option
    pub fn into_buf(&mut self, checksum_option: bool, buffer: &mut [u8]) -> usize {
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
            Self::NDataPDU { fixed, addr, seg, opts, discard, data } |
            Self::EchoRequestPDU { fixed, addr, seg, opts, discard, data } |
            Self::EchoResponsePDU { fixed, addr, seg, opts, discard, data } => {
                // prepare octet 5
                let octet5 = NFixedPart::compose_octet5_unchecked(
                    //TODO dont know of these conversions are really needed
                    if fixed.sp_segmentation_permitted { SpSegmentationPermittedBit::ONE } else { SpSegmentationPermittedBit::ZERO },
                    if fixed.ms_more_segments { MsMoreSegmentsBit::ONE } else { MsMoreSegmentsBit::ZERO },
                    if fixed.er_error_report { ErErrorReportBit::ONE } else { ErErrorReportBit::ZERO },
                    *fixed.type_
                );

                // prepare length indicators
                //TODO because of setting the values here we have to make it &mut self - make it possible to use &self?
                //TODO regarding length indicators calculation: is "reason for discard" part of the header as per Standard? Or is this actually part of the Data part of ER PDU?
                let (fixed_length_indicator, fixed_segment_length, addr_destination_address_length_indicator, addr_source_address_length_indicator) = Pdu::get_length_indicators(&fixed, &addr, &seg, &opts, &data);

                // write into output buffer
                let mut bytes = 0;

                // fixed part
                buffer[0] = *fixed.network_layer_protocol_identifier;
                buffer[1] = fixed_length_indicator; // header length
                buffer[2] = *fixed.version_protocol_id_extension;
                buffer[3] = *fixed.lifetime;
                buffer[4] = octet5;
                //let segment_length_ne = fixed.segment_length.to_be_bytes();
                //buffer[5] = segment_length_ne[0];   // packet length incl. header   //TODO calculate ;-)
                //buffer[6] = segment_length_ne[1];
                buffer[5] = fixed_segment_length.to_be_bytes()[0];   // packet length incl. header
                buffer[6] = fixed_segment_length.to_be_bytes()[1];
                let checksum_be = fixed.checksum.to_be_bytes(); // should be set to the invalid value - the checksum algorithm requires 0 for the checksum bytes at first
                buffer[7] = checksum_be[0];
                buffer[8] = checksum_be[1];
                bytes += 9;

                // address part
                //destination address
                buffer[9] = addr_destination_address_length_indicator;
                bytes += 1;
                for i in 0..addr.destination_address.len() {
                    buffer[bytes+i] = addr.destination_address[i];
                }
                bytes += addr_destination_address_length_indicator as usize;   //TODO optimize
                // source address
                buffer[bytes] = addr_source_address_length_indicator;
                bytes += 1;
                for i in 0..addr.source_address.len() {
                    buffer[bytes+i] = addr.source_address[i];
                }
                bytes += addr_source_address_length_indicator as usize;    //TODO optimize

                // segmentation part
                if let Some(seg_inner) = seg {
                    let data_unit_identifier_be = seg_inner.data_unit_identifier.to_be_bytes();
                    buffer[bytes] = data_unit_identifier_be[0];
                    buffer[bytes+1] = data_unit_identifier_be[1];
                    bytes += 2;
                    let segment_offset_be = seg_inner.segment_offset.to_be_bytes();
                    buffer[bytes] = segment_offset_be[0];
                    buffer[bytes+1] = segment_offset_be[1];
                    bytes += 2;
                    let total_length_be = seg_inner.total_length.to_be_bytes();
                    buffer[bytes] = total_length_be[0];
                    buffer[bytes+1] = total_length_be[1];
                    bytes += 2;
                }

                // options part
                if let Some(opts_inner) = opts {
                    bytes += opts_inner.len_bytes();
                    //TODO compose the options
                    todo!();
                }

                // reason for discard part
                // N/A only for ER PDU

                // now set the checksum for the header
                if checksum_option {
                    // calculate checksum
                    /*
                    see X.233 6.11 PDU header error detection function 
                    and X.233 Annex C Algorithms for PDU header error detection function
                    ideas in Wireshark OSI protocols dissector:  https://gitlab.com/wireshark/wireshark/-/blob/master/epan/dissectors/packet-osi.c#L113
                    efficient mod-255 computation:  https://stackoverflow.com/questions/68074457/efficient-modulo-255-computation
                    */
                    //TODO optimize, this is the 1:1 naive "mod 255 arithmetic calculation variant" given in X.233
                    let mut c0: isize = 0;
                    let mut c1: isize = 0;
                    println!("checksum:  got {} bytes header", bytes);
                    for i in 0..bytes {
                        c0 = c0 + buffer[i] as isize;
                        c1 = c1 + c0;
                    }
                    let mut x = ((bytes as isize - 8) * c0 - c1).rem_euclid(255);
                    let mut y = ((bytes as isize - 7) * (-1 * c0) + c1).rem_euclid(255);   // % operator would give wrong result for negative y
                    if x == 0 { x = 255; }
                    if y == 0 { y = 255; }

                    // assign into fixed part field
                    buffer[7] = x as u8;
                    buffer[8] = y as u8;
                }

                // data part
                //TODO optimize
                if let Some(data_inner) = data {
                    for i in 0..data_inner.data.len() {
                        buffer[bytes+i] = data_inner.data[i];
                    }
                    return bytes + data_inner.data.len();
                } else {
                    return bytes;   //TODO
                }
            },
            //TODO are data PDU, ERQ, ERP PDU *and* multicast serialized in the same way?
            Self::MulticastDataPDU { fixed, addr, seg, opts, discard, data } => {
                todo!();
            },
            Self::ErrorReportPDU { fixed, addr, opts, discard, data } => {
                todo!();
                //param: &'a NParameter<'a>   //TODO enforce that here only parameter code "1100 0001" is allowed
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
        destination_nsap: Option<&Nsap>,
        source_address_index: Option<usize>,
        options: Option<NOptionsPart>,
        quality_of_service: &crate::n::Qos
    ) {
        // destination
        let destination_address: &Nsap;
        if let Some(destination_title2) = destination_title {
            // convert to NSAP
            destination_address = self.resolve_nsap(&destination_title2).expect("failed to resolve system-title");
            // Nsap::new_from_network_entity_title(destination_title.unwrap());
            // TODO implement ^ kind of NSAP which is allowed by standard
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
        let mut buffer_scratch = [0u8; 64];
        let mut erq_pdu = Pdu::new_echo_request(
            false,   //TODO implement non-segmenting protocol subset properly - refer to NS.operating mode or so
            &source_address,
            &destination_address,
            &options,
            &mut buffer_scratch
        );

        // send it via data link or subnetwork
        let sn_quality_of_service = dl::Qos{};  //TODO convert Network Layer QoS to Data Link Layer QoS
        self.sn_service.sn_unitdata_request(
            source_address.local_address,
            destination_address.local_address,
            sn_quality_of_service,
            &mut erq_pdu
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
            for mut pdu in pdus {   //TODO optimize this should iterate over &Pdu not Pdu (copy?)
                self.sn_service.sn_unitdata_request(
                    ns_source_address.local_address,
                    ns_destination_address.local_address,
                    dl::Qos{},   //TODO optimize useless allocation; and no real conversion - the point of having two different QoS on DL and N layer is that the codes for QoS cloud be different
                    &mut pdu    //TODO not perfect abstraction, but should save us a memcpy
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