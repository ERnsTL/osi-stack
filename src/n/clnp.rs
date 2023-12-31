extern crate simplelog; //TODO check the paris feature flag for tags, useful?

use std::{collections::HashMap, io::Error, thread::{self, Thread, JoinHandle}, sync::{Arc, Mutex}, time::Duration};

use advmac::MacAddr6;
use rand::Rng;
use chrono::prelude::*;

use crate::dl::SNUnitDataRequest;
use super::{Nsap, Qos, NUnitDataIndication};

pub fn parse_macaddr(instr: &str) -> Result<MacAddr6, advmac::ParseError> {
    MacAddr6::parse_str(instr)
}

#[derive(Debug)]
pub enum Pdu<'a> {
    Inactive { fixed_mini: NFixedPartMiniForInactive<'a>, data: NDataPart<'a> },
    DataPDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, seg: Option<NSegmentationPart>, opts: Option<NOptionsPart<'a>>, discard: Option<NReasonForDiscardPart<'a>>, data: Option<NDataPart<'a>>},
    // no segmentation, but reason for discard is mandatory
    ErrorReportPDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, opts: Option<NOptionsPart<'a>>, discard: NReasonForDiscardPart<'a>, data: Option<NDataPart<'a>> },
    // these are the same as DataPDU / DT PDU
    EchoRequestPDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, seg: Option<NSegmentationPart>, opts: Option<NOptionsPart<'a>>, discard: Option<NReasonForDiscardPart<'a>>, data: Option<NDataPart<'a>> },
    EchoResponsePDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, seg: Option<NSegmentationPart>, opts: Option<NOptionsPart<'a>>, discard: Option<NReasonForDiscardPart<'a>>, data: Option<NDataPart<'a>> },
    MulticastDataPDU { fixed: NFixedPart<'a>, addr: NAddressPart<'a>, seg: Option<NSegmentationPart>, opts: Option<NOptionsPart<'a>>, discard: Option<NReasonForDiscardPart<'a>>, data: Option<NDataPart<'a>> }
}

const VERSION_PROTOCOL_ID_EXTENSION_1: u8 = 0b0000_0001;

/// actually 5 bits, so bits 8,7,6 are 0
const TYPE_DT_PDU: u8 = 0b00011100;     // data
const TYPE_MD_PDU: u8 = 0b00011101;     // multicast data
const TYPE_ER_PDU: u8 = 0b00000001;     // error report
const TYPE_ERQ_PDU: u8 = 0b00011110;    // echo request
const TYPE_ERP_PDU: u8 = 0b00011111;    // echo response

const CHECKSUM_INVALID_IGNORE: (&u8, &u8) = (&0, &0);  // X.233 7.2.9 PDU checksum and X.233 6.19 e) for Echo Request function
const SEGMENT_LENGTH_INVALID: u16 = 0;  // X.233 6.19 e) for Echo Request function

impl<'a> Pdu<'_> {
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
            Self::DataPDU { fixed, addr, seg, opts, discard, data } |
            Self::EchoRequestPDU { fixed, addr, seg, opts, discard, data } |
            Self::EchoResponsePDU { fixed, addr, seg, opts, discard, data } => {
                // prepare octet 5
                let octet5 = NFixedPart::compose_octet5_unchecked(
                    //TODO dont know of these conversions are really needed
                    if fixed.sp_segmentation_permitted { SpSegmentationPermittedBit::ONE } else { SpSegmentationPermittedBit::ZERO },
                    if fixed.ms_more_segments { MsMoreSegmentsBit::ONE } else { MsMoreSegmentsBit::ZERO },
                    if fixed.er_error_report { ErErrorReportBit::ONE } else { ErErrorReportBit::ZERO },
                    fixed.type_
                );
                debug!("composing octet 5 has value: {}", octet5);

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
                buffer[5] = fixed_segment_length.to_be_bytes()[0];   // packet length incl. header  //TODO should not be calculated in the case of Echo Request PDU which should contain an Echo Response PDU with invalid checksum and segment length
                buffer[6] = fixed_segment_length.to_be_bytes()[1];
                buffer[7] = *fixed.checksum.0;  // should be set to the invalid value - the checksum algorithm requires 0 for the checksum bytes at first
                buffer[8] = *fixed.checksum.1;
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
                    difference modulos and remainder:  https://stackoverflow.com/questions/31210357/is-there-a-modulus-not-remainder-function-operation
                    */
                    //TODO optimize, this is the 1:1 naive "mod 255 arithmetic calculation variant" given in X.233
                    let mut c0: isize = 0;
                    let mut c1: isize = 0;
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
        }
        //matches!(self, Self::Inactive { .. })
    }

    pub fn from_buf(buffer: &[u8]) -> Pdu { //TODO add error handling (Result)
        match buffer[0] {
            NETWORK_LAYER_PROTOCOL_IDENTIFIER_CLNP_FULL => {
                //TODO implement correct algorithm for PDU decomposition according to standard
                // check for length and PDU type
                let type_;
                if buffer.len() < 5 {
                    // too short
                    panic!();
                }
                // check octet 5
                (_, _, _, type_) = NFixedPart::decompose_octet5(&buffer[4]);
                debug!("got PDU type_: {}", type_);
                /*
                // X.233 7.2.6.1 Segmentation permitted
                if !sp_segmentation_permitted -> segmentation header is not there and value of segment_length field gives the total length of the PDU see 7.2.8 PDU segment length (fixed part) and 7.4.3 Segment offset (segmentation part)
                // X.233 7.6.6.2 More segments
                More segments flag == 1 -> segmentation has occured.
                More segments flag shall not be set to 1 if the segmentation permitted flag is not set to 1.
                when the more segments flag is set to zero, te last octet of the data part is the last octet of the NSDU.
                // X.233 7.2.8 PDU segment length
                if full protocol is employed && PDU is not segmented, value of this field is identical to the value of the total length field located in the segmentation part of the header.
                if non-segmenting protocol subset is employed -> no segmentation part. and segment length field (fixed part) specifies entire length of PDU (header and data, if present)
                // X.233 7.4.1 General (Segmentation part)
                if the SP flag is set in fixed part (7.2.6.1) == 1 the segmentation part of the header shall be present.
                // X.233 7.5.1 General (Options part)
                Options part length = PDU header length - (length of fixed part + length of address part + length of segmentation part)
                // X.233 7.9.5 Reason for discard
                This parameter is valid only for the Error Report PDU.
                // X.233 7.7.1 Structure (Data PDU) and 7.9.1 Structure (Error Report PDU) show nice overall figure of the variable byte indices in the PDU
                */
                match type_ {   //TODO optimize does the ordering of match conditions matter? should most common case be first?
                    TYPE_ER_PDU => {
                        debug!("got an error report PDU");
                        todo!();
                    },
                    TYPE_DT_PDU | TYPE_MD_PDU | TYPE_ERQ_PDU | TYPE_ERP_PDU => {
                        match type_ {
                            TYPE_DT_PDU => { debug!("got a data PDU"); },
                            TYPE_MD_PDU => { debug!("got a multicast data PDU"); },
                            TYPE_ERQ_PDU => { debug!("got an echo request PDU"); },
                            TYPE_ERP_PDU => { debug!("got an echo response PDU"); },
                            _ => { todo!(); }
                        }

                        // decompose PDU

                        // fixed part
                        //TODO check if buffer is actually that long
                        let fixed_part_length: usize = 9; //TODO optimize const
                        let (fixed_part, segmentation_part_present) = NFixedPart::from_buf(&buffer[0..fixed_part_length]).expect("failed to decompose fixed part");
                        if fixed_part.ms_more_segments && !fixed_part.sp_segmentation_permitted {
                            // combination not allowed
                            panic!("sp_segmentation_permitted=false but ms_more_segments=true not allowed");
                        }
                        if fixed_part.ms_more_segments {
                            // segmentation has occured
                            todo!();
                        }

                        // address part
                        //TODO check if buffer length is at least 1+1+1+1 bytes more
                        let (address_part, address_part_length) = NAddressPart::from_buf(&buffer[fixed_part_length..buffer.len()]).expect("failed to decompose address part");

                        // segmentation part
                        let segmentation_part;
                        let segmentation_part_length;
                        if segmentation_part_present {
                            //TODO optimize - is always 6 bytes
                            (segmentation_part, segmentation_part_length) = NSegmentationPart::from_buf(&buffer[(fixed_part_length+address_part_length)..buffer.len()]).expect("failed to decompose segmentation part");
                        } else {
                            segmentation_part = None; segmentation_part_length = 0;
                        }

                        // options part
                        let options_part;
                        let options_part_length = (*fixed_part.length_indicator.unwrap() as usize) - (fixed_part_length + address_part_length + segmentation_part_length);
                        let options_part_present = options_part_length != 0;
                        if options_part_present {
                            options_part = NOptionsPart::from_buf(&buffer[(fixed_part_length+address_part_length+segmentation_part_length)..buffer.len()]).expect("failed to decompose options part");
                        } else {
                            options_part = None;
                        }

                        // reason for discard part
                        let reason_for_discard_part = None; //NOTE: only present in ER PDU
                        let reason_for_discard_part_length = 0;

                        // check header checksum
                        //TODO ... and handle disabled checksum (value 00)

                        // data part
                        let header_length = fixed_part_length+address_part_length+segmentation_part_length+options_part_length+reason_for_discard_part_length;
                        let data_part_length = (fixed_part.segment_length.unwrap() as usize) - header_length;   // TODO if segmented, then this is I think not correct
                        let data_part = NDataPart::from_buf(&buffer[header_length..buffer.len()], data_part_length).expect("failed to decompose data part");  //TODO optimize conversion/casting
                        //TODO check for overhead bytes

                        //TODO decompose Echo Response contained in the Echo Request PDU data part

                        // assemble and return decomposed PDU
                        match type_ {
                            TYPE_DT_PDU => { return Pdu::DataPDU { fixed: fixed_part, addr: address_part, seg: segmentation_part, opts: options_part, discard: reason_for_discard_part, data: data_part }; },
                            TYPE_MD_PDU => { return Pdu::MulticastDataPDU { fixed: fixed_part, addr: address_part, seg: segmentation_part, opts: options_part, discard: reason_for_discard_part, data: data_part }; },
                            TYPE_ERQ_PDU => { return Pdu::EchoRequestPDU { fixed: fixed_part, addr: address_part, seg: segmentation_part, opts: options_part, discard: reason_for_discard_part, data: data_part }; },
                            TYPE_ERP_PDU => { return Pdu::EchoResponsePDU { fixed: fixed_part, addr: address_part, seg: segmentation_part, opts: options_part, discard: reason_for_discard_part, data: data_part }; },
                            _ => { todo!(); }
                        }
                    },
                    _ => {
                        // unknown PDU type
                        panic!("unknown CLNP NPDU type: {}", type_);
                    }
                }
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

    fn new_echo_request(
        sp_segmentation_permitted: bool,    //TODO use that :-)
        source_address: &Nsap,
        destination_address: &Nsap,
        options: &Option<NOptionsPart>, //TODO use that :-)
        correlation_data: &[u8],    // user data of inner Echo Response PDU
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
                type_: TYPE_ERP_PDU,
                octet5: &0,  // to be filled
                segment_length: None,  // an invalid value per 6.19 e) which should also be transmitted this way
                checksum: CHECKSUM_INVALID_IGNORE,    // an invalid value per 6.19 e) which should also be transmitted this way TODO -> use Option
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
                data: correlation_data
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
                type_: TYPE_ERQ_PDU,
                octet5: &0,  // will be filled
                segment_length: None,  // should be filled like any other DT PDU - TODO
                checksum: CHECKSUM_INVALID_IGNORE,    // should be filled like any other DT PDU - TODO
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
    fn get_length_indicators(fixed: &NFixedPart<'_>, addr: &NAddressPart<'_>, seg: &Option<NSegmentationPart>, opts: &Option<NOptionsPart<'_>>, data: &Option<NDataPart<'_>>) -> (u8, u16, u8, u8) {
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
const NETWORK_LAYER_PROTOCOL_IDENTIFIER_CLNP_FULL: u8 = 0b1000_0001;    // used for both full and non-segmenting protocol subset
const NETWORK_LAYER_PROTOCOL_IDENTIFIER_CLNP_INACTIVE: u8 = 0b0000_0000;

#[derive(Debug)]
pub struct NFixedPart<'a> {
    network_layer_protocol_identifier: &'a u8,
    length_indicator: Option<&'a u8>,
    version_protocol_id_extension: &'a u8,
    lifetime: &'a u8,
    /// 0 = not permitted, no segmentation part present in PDU, non-segmenting protocol subset in use
    /// 1 = permitted, segmentation part shall be present in PDU, full protocol is in use
    sp_segmentation_permitted: bool,   // 1 bit
    ms_more_segments: bool,   // 1 bit
    er_error_report: bool,  // 1 bit
    type_: u8, // 5 bits only!
    /// contains ^ sub-bit values
    octet5: &'a u8,
    segment_length: Option<u16>,
    checksum: (&'a u8, &'a u8)  // these are two checksum bytes handled separately according to the algorithm in X.233 Annex C
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

    fn decompose_octet5(octet5: &u8) -> (bool, bool, bool, u8) {
        return (
            (octet5 & 0b10000000) != 0, // no right shift needed - only need to know if bit i is 0, then the number is also zero or if it is != 0 then the bit there is set even if it means 128, 64 or 32 value
            (octet5 & 0b01000000) != 0,
            (octet5 & 0b00100000) != 0,
            octet5 & 0b00011111
        );
    }

    /// returns the fixed part,
    /// segmentation_part_present: bool
    fn from_buf<'a>(buffer: &'a [u8]) -> Result<(NFixedPart<'a>, bool), Error> {
        // checks
        if buffer.len() < 9 {
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "given header too short"));
        }

        // parse fixed part
        debug!("decomposing octet 5 from value: {}", &buffer[4]);
        let (fixed_part_sp_segmentation_permitted,
            fixed_part_ms_more_segments,
            fixed_part_er_error_report,
            fixed_part_type_) = NFixedPart::decompose_octet5(&buffer[4]);
        debug!("octet 5:  sp_segmentation_permitted={}  ms_more_segments={}  er_error_report={}  type_={}", fixed_part_sp_segmentation_permitted, fixed_part_ms_more_segments, fixed_part_er_error_report, fixed_part_type_);
        let fixed_part = NFixedPart {
            network_layer_protocol_identifier: &buffer[0],
            length_indicator: Some(&buffer[1]),
            version_protocol_id_extension: &buffer[2],
            lifetime: &buffer[3],
            sp_segmentation_permitted: fixed_part_sp_segmentation_permitted,
            ms_more_segments: fixed_part_ms_more_segments,
            er_error_report: fixed_part_er_error_report,
            type_: fixed_part_type_,
            octet5: &buffer[4],
            segment_length: Some(u16::from_be_bytes([buffer[5], buffer[6]].try_into().expect("failed to convert segment length from be bytes"))),
            checksum: (&buffer[7], &buffer[8]),
        };

        return Ok((
            fixed_part,
            fixed_part_sp_segmentation_permitted
        ));
    }
}

#[derive(Debug)]
pub struct NAddressPart<'a> {
    destination_address_length_indicator: Option<&'a u8>,
    destination_address: Vec<u8>,  //TODO optimize - owned only because of Pdu::to_buf() converts Nsap to [u8] and "data is owned by current function"
    source_address_length_indicator: Option<&'a u8>,
    source_address: Vec<u8>    //TODO optimize - owned only because of Pdu::to_buf() converts Nsap to [u8] and "data is owned by current function"
}

/// returns the address part,
/// address_part_length: usize
impl NAddressPart<'_> {
    fn from_buf<'a>(buffer: &'a [u8]) -> Result<(NAddressPart<'a>, usize), Error> {
        //TODO having destination_address_length_indicator and source_address_length_indicator be &u8 causes all kinds of conversions, casts and temporary values
        if buffer.len() < 1 {   // 1 byte for each NSAP at the very minimum
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "given address part buffer too short at all"));
        }
        let destination_address_length_indicator: &u8 = &buffer[0];
        if buffer.len() < 1 + (*destination_address_length_indicator as usize) {
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "given address part buffer too short to accomodate destination address"));
        }
        let destination_address = &buffer[1..1+(*destination_address_length_indicator as usize)];
        let source_address_length_indicator: &u8 = &buffer[1+(*destination_address_length_indicator as usize)];
        if buffer.len() < 1 + (*destination_address_length_indicator as usize) + 1 + (*source_address_length_indicator as usize) {
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "given address part buffer too short to accomodate source address"));
        }
        let source_address = &buffer[1+(*destination_address_length_indicator as usize)+1..1+(*destination_address_length_indicator as usize)+1+(*source_address_length_indicator as usize)];

        let address_part = NAddressPart {
            destination_address_length_indicator: Some(destination_address_length_indicator),
            destination_address: destination_address.to_vec(),  //TODO optimize - does this copy?
            source_address_length_indicator: Some(source_address_length_indicator),
            source_address: source_address.to_vec(),    //TODO optimize - does this copy?
        };
        return Ok((
            address_part,
            1+(*destination_address_length_indicator as usize)+1+(*source_address_length_indicator as usize)
        ));
    }
}

#[derive(Debug)]
pub struct NSegmentationPart {
    data_unit_identifier: u16,  // did not find a way in from_buf() to reference into the buffer directly and get out an &'a u16 with BE to NE (native endian) conversion, therefore no &u16 but u16 //TODO optimize
    segment_offset: u16,
    total_length: u16
}

/// returns the segmentation part,
/// segmentation_part_length: usize
impl NSegmentationPart {
    fn from_buf<'a>(buffer: &[u8]) -> Result<(Option<NSegmentationPart>, usize), Error> {
        if buffer.len() < 6 {
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "given segmentation part buffer too short"));
        }
        let segmentation_part = NSegmentationPart {
            data_unit_identifier: u16::from_be_bytes(buffer[0..1].try_into().unwrap()),    //TODO optimize these calls
            segment_offset: u16::from_be_bytes(buffer[2..3].try_into().unwrap()),
            total_length: u16::from_be_bytes(buffer[4..5].try_into().unwrap()),
        };
        return Ok((
            Some(segmentation_part),    // decision about None or Some is made outside in Pdu::from_buf()
            6
        ));
    }
}

#[derive(Debug)]
pub struct NOptionsPart<'a> {
    params: &'a [NParameter<'a>]
}

/// only contained in NOptionsPart
//TODO decomposition of these parameters
#[derive(Debug)]
pub struct NParameter<'a> {
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

    fn from_buf(buffer: &[u8]) -> Result<Option<Self>, Error> {
        todo!()
    }
}

#[derive(Debug)]
pub struct NReasonForDiscardPart<'a> {
    /// has format of a parameter from the options part
    param: &'a NParameter<'a>   //TODO enforce that here only parameter code "1100 0001" is allowed
}

#[derive(Debug)]
pub struct NDataPart<'a> {
    data: &'a [u8]
}

impl NDataPart<'_> {
    fn from_buf<'a>(buffer: &'a [u8], data_part_length: usize) -> Result<Option<NDataPart<'a>>, Error> {
        if buffer.len() < data_part_length {
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "given data part buffer too short"));
        }
        return Ok(Some(NDataPart {
            data: &buffer[0..data_part_length],
        }));
    }
}

pub struct Service<'a> {
    // internal state
    pub serviced_nsaps: Vec<Nsap>,  //TODO should be via get_serviced_nsap() but this would mean a 2nd borrow (borrow-checker understands direct variable access but if it is done via a method like get_serviced_nsap() then locks the whole service variable and we have a 2nd borrow)
    known_hosts: HashMap<String, Nsap>,
    network_entity_title: &'a str,   // own title
    echo_request_correlation_table: Arc<Mutex<HashMap<u16, DateTime<Utc>>>>,    //TODO harden for collisions //TODO currently this is global correlation - have this per-target-NSAP?

    // underlying service assumed by the protocol = subnet service on data link layer
    sn_service_to: Arc<Mutex<rtrb::Producer<SNUnitDataRequest>>>,
    sn_service_to_wakeup: Arc<Mutex<Option<JoinHandle<Thread>>>>,
    sn_service_from: Arc<Mutex<rtrb::Consumer<NUnitDataIndication>>>,
}

impl<'a> super::NetworkService<'a> for Service<'a> {
    fn new(
        network_entity_title: &'a str,
        sn_service_to: rtrb::Producer<SNUnitDataRequest>,
        sn_service_to_wakeup: Arc<Mutex<Option<JoinHandle<Thread>>>>,
        sn_service_from: rtrb::Consumer<NUnitDataIndication>
    ) -> Service<'a> {
        Service {
            serviced_nsaps: vec![],
            known_hosts: HashMap::new(),
            network_entity_title: network_entity_title,
            echo_request_correlation_table: Arc::new(Mutex::new(HashMap::new())),
            sn_service_to: Arc::new(Mutex::new(sn_service_to)),
            sn_service_to_wakeup: sn_service_to_wakeup,
            sn_service_from: Arc::new(Mutex::new(sn_service_from)),
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

    //TODO it seems this should be used only for CLNP Data PDUs (and if they go to a Multicast address, then incidentially Multicast Data PDUs) but not for Echo Request PDUs
    // because the N-UNITDATA-REQUEST does not have a parameter for CLNP PDU type so there is no way to select composition of an Echo Request PDU
    fn n_unitdata_request(
        &mut self,
        ns_destination_title: &str,
        ns_quality_of_service: &Qos,
        ns_userdata: &[u8]
    ) {
        let get_serviced_nsap = self.get_serviced_nsap().expect("no serviced NSAPs").clone();   //TODO optimize clone - again the cannot borrow self 2 times issue
        let dest_nsap = self.resolve_nsap(ns_destination_title).expect("cannot resolve destination host").clone();  //TODO optimize clone - again the cannot borrow self 2 times issue
        /*
        self.n_unitdata_request_internal(
            &get_serviced_nsap,
            &dest_nsap,
            &ns_quality_of_service,
            ns_userdata
        );
        */
        let ns_source_address = get_serviced_nsap;
        let ns_destination_address = dest_nsap;
        // check if we are on same Ethernet broadcast domain as destination
        if can_use_inactive_subset(&ns_source_address, &ns_destination_address) {
            // compose PDU(s)
            let pdus = self.pdu_composition(true, &ns_source_address, &ns_destination_address, ns_quality_of_service, ns_userdata);
            // unitdata request to SN
            for mut pdu in pdus {   //TODO optimize this should iterate over &Pdu not Pdu (copy?)
                let mut buffer = [0u8; 1500];    //TODO optimize this whole to_buf and transfer to SN
                let bytes = pdu.into_buf(true, &mut buffer);
                let mut thevec: Vec<u8> = Vec::with_capacity(bytes);
                thevec.extend_from_slice(&buffer[0..bytes]);
                self.sn_service_to.lock().expect("failed to lock sn_service_to").push(SNUnitDataRequest{
                    sn_source_address: ns_source_address.local_address,
                    sn_destination_address: ns_destination_address.local_address,
                    sn_quality_of_service: crate::dl::Qos{},   //TODO optimize useless allocation; and no real conversion - the point of having two different QoS on DL and N layer is that the codes for QoS cloud be different
                    sn_userdata: thevec,    //TODO not perfect abstraction, but should save us a memcpy
                }).expect("failed to push into sn_service_to, SN-UNITDATA-REQUEST lost because of congestion"); //TODO congestion situation - apply congestion function
                //self.sn_service_to.flush();   //TODO make it flush the socket - we can control this via unpark
                self.sn_service_to_wakeup.lock().expect("failed to lock sn_service_to_wake").as_ref().expect("failed to get sn_service_to_wakeup (taker)").thread().unpark();
            }
            return;
        }
        todo!();
    }

    //TODO implement properly (PDU decomposition)
    fn n_unitdata_indication(//&self,
        sn_service_to: &mut rtrb::Producer<SNUnitDataRequest>,
        sn_service_to_wakeup: &JoinHandle<Thread>,
        echo_request_correlation_table: Arc<Mutex<HashMap<u16, DateTime<Utc>>>>,
        // actual parameters
        ns_source_address: MacAddr6,
        ns_destination_address: MacAddr6,
        ns_quality_of_service: &Qos,
        ns_userdata: &[u8]
    ) {
        let pdu = Pdu::from_buf(ns_userdata);
        debug!("got CLNP packet: {:?}", pdu);
        match pdu { //TODO optimize does match leg ordering affect performance?
            Pdu::Inactive { fixed_mini, data } => {
                debug!("n_unitdata_indication(): got inactive protocol subset packet");
                if let Ok(thestr) = std::str::from_utf8(&data.data) {
                    info!("Inactive subset packet data ({} bytes, padded by SN): {}", data.data.len(), thestr);
                } else {
                    info!("Inactive subset packet data ({} bytes, padded by SN): {:?}", data.data.len(), data.data);
                }
                //TODO implement further functions
            },
            Pdu::EchoRequestPDU { fixed, addr, seg, opts, discard, data  } => {
                if let Some(data_inner) = data {
                    debug!("parsing inner Echo Response");
                    let erp_pdu_inner = Pdu::from_buf(data_inner.data);
                    debug!("got inner Echo Response: {:?}", erp_pdu_inner);
                    // respond with echo response
                    if let Pdu::EchoResponsePDU { fixed, addr, seg, opts, discard, data } = erp_pdu_inner {
                        //TODO implement correct behavior according to Echo Response function
                        //TODO add checks - otherwise this can be used for DoS attack ("please bomb that other host")
                        // send back to sender
                        sn_service_to.push(SNUnitDataRequest {
                            sn_source_address: ns_destination_address,
                            sn_destination_address: ns_source_address,
                            sn_quality_of_service: crate::dl::Qos::from_ns_quality_of_service(ns_quality_of_service),    //TODO optimize?  //TODO convert NS QoS to SN QoS
                            sn_userdata: data_inner.data.to_vec()    //TODO security    //TODO optimize?
                        }).expect("failed to push into sn_service_to, SN-UNITDATA-REQUEST lost because of congestion"); //TODO congestion situation, apply congestion function
                        // wake up SN thread
                        sn_service_to_wakeup.thread().unpark();
                    } else {
                        panic!("expected inner echo response PDU inside received echo request")
                    }
                } else {
                    panic!("expected inner echo response PDU inside received echo request (no data part)");
                }
            },
            Pdu::EchoResponsePDU { fixed, addr, seg, opts, discard, data } => {
                // correlate
                let now = Utc::now();
                let mut table = echo_request_correlation_table.lock().expect("failed to lock echo_request_correlation_table");
                let correlation_data_u8 = data.expect("failed to get data part from Echo Response PDU").data;   //TODO harden
                //TODO check if data has enough bytes for the correlation data
                //TODO optimize correlation data can be just meaningless u8 data instead of nice u16 be/ne data
                let correlation_data = u16::from_ne_bytes([correlation_data_u8[0], correlation_data_u8[1]].try_into().expect("failed to convert correlation data from ne bytes"));
                if let Some(time_sent) = table.remove(&correlation_data) {
                    info!("echo response after {}  {}ms  {}us  {}ns",
                        now - time_sent,
                        (now - time_sent).num_milliseconds(),
                        (now - time_sent).num_microseconds().expect("failed to convert duration for display"),
                        (now - time_sent).num_nanoseconds().expect("failed to convert duration for display")
                    );
                    //info!("echo response after {}us", (now - time_sent).num_microseconds().expect("failed to convert duration for display"));
                } else {
                    info!("stray Echo Response PDU received: failed to correlate");
                }
            }
            Pdu::DataPDU { fixed, addr, seg, opts, discard, data } => {
                debug!("n_unitdata_indication(): got data PDU");
                if let Some(datapart) = data {
                    info!("data PDU data: {:?}", datapart.data);
                } else {
                    info!("data PDU data: None");
                }
                //TODO implement further functions
            }
            Pdu::MulticastDataPDU { fixed, addr, seg, opts, discard, data } => {
                debug!("n_unitdata_indication(): got multicast data PDU");
                todo!();
            }
            Pdu::ErrorReportPDU { fixed, addr, opts, discard, data } => {
                debug!("n_unitdata_indication(): got error report PDU");
                todo!();
                //TODO correlate to previously-sent PDU (how?)
            }
            _ => { info!("n_unitdata_indication(): unknown CLNP PDU type"); }
        }
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
        if let Some(ref destination_title2) = destination_title {
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
        //TODO super-clunky
        info!("echo request from {} to {}: ", source_address.to_string(), destination_address.to_string());

        // check length
        //TODO 6.19 d)

        // correlation data
        let correlation_data: u16 = rand::thread_rng().gen();

        // compose ERQ PDU
        let mut buffer_scratch = [0u8; 64];
        let mut erq_pdu = Pdu::new_echo_request(
            false,   //TODO implement non-segmenting protocol subset properly - refer to NS.operating mode or so
            &source_address,
            &destination_address,
            &options,
            &correlation_data.to_ne_bytes(),  // no need to convert to bigendian/network encoding, is just for us and we dont need to treat it as a number anyway
            &mut buffer_scratch
        );

        // send it via data link or subnetwork
        let sn_quality_of_service = crate::dl::Qos{};  //TODO convert Network Layer QoS to Data Link Layer QoS
        let mut buffer = [0u8; 1500];   //TODO optimize this whole to_buf and transfer to SN
        let bytes = erq_pdu.into_buf(true, &mut buffer);
        let mut thevec: Vec<u8> = Vec::with_capacity(bytes);
        thevec.extend_from_slice(&buffer[0..bytes]);
        self.sn_service_to.lock().expect("failed to lock sn_service_to").push(SNUnitDataRequest{
            sn_source_address: source_address.local_address,
            sn_destination_address: destination_address.local_address,
            sn_quality_of_service: sn_quality_of_service,
            sn_userdata: thevec,
        }).expect("failed to push SNUnitDataRequest into sn_service");
        // wake up SN thread
        self.sn_service_to_wakeup.lock().expect("failed to lock sn_service_to_wake").as_ref().expect("failed to get sn_service_to_wakeup (taker)").thread().unpark();

        //TODO send properly via n_unitdata_request()
        //###
        //TODO remove self.buffer_out
        /*
        let mut buffer = [0u8; 1500];   //TODO optimize this whole to_buf and transfer to SN
        let bytes = erq_pdu.into_buf(true, &mut buffer);
        self.n_unitdata_request(
            destination_title.expect("expected destination_title - other not implemented yet").as_str(),
            quality_of_service,
            &buffer[0..bytes]
        );
        */

        // add entry to correlation table
        self.echo_request_correlation_table.lock().expect("failed to lock echo_request_correlation_table").insert(correlation_data, Utc::now());
    }

    fn run(&mut self,
        sn2ns_consumer_wakeup_give: Arc<Mutex<Option<JoinHandle<Thread>>>>
    ) {
        // read N-UNITDATA-INDICATION from SN
        let sn_service_from_arc = self.sn_service_from.clone();
        let sn_service_to_arc = self.sn_service_to.clone();
        let sn_service_to_wakeup_arc = self.sn_service_to_wakeup.clone();
        let echo_request_correlation_table_arc = self.echo_request_correlation_table.clone();
        let sn2ns_consumer_wakeup = thread::Builder::new().name("N CLNP <- SN".to_string()).spawn(move || {
            // keep permanent lock on this
            let mut sn_service_from = sn_service_from_arc.lock().expect("failed to lock sn_service_from");
            // NOTE: cannot keep permanent lock on sn_service_to because other places need it, too
            //let mut sn_service_to = sn_service_to_arc.lock().expect("failed to lock sn_service_to");
            loop {
                // pop all
                loop {
                    if let Ok(n_unitdata_indication) = sn_service_from.pop() {
                        debug!("got N UnitData indication: {:?}", n_unitdata_indication);
                        let mut sn_service_to = sn_service_to_arc.lock().expect("failed to lock sn_service_to");
                        let sn_service_to_wakeup_outer = sn_service_to_wakeup_arc.lock().expect("failed to lock sn_service_to_wakeup (taker)");
                        let sn_service_to_wakeup = sn_service_to_wakeup_outer.as_ref().expect("sn_service_to_wakeup is none (taker)");
                        //TODO optimize ^ we can surely take this join handle out and clone it - dont need to lock mutex on every call
                        Self::n_unitdata_indication(
                            &mut *sn_service_to,
                            sn_service_to_wakeup,
                            echo_request_correlation_table_arc.clone(), //TODO optimize - for now clone() to avoid "use of moved value"
                            n_unitdata_indication.ns_source_address,
                            n_unitdata_indication.ns_destination_address,
                            &n_unitdata_indication.ns_quality_of_service,
                            &n_unitdata_indication.ns_userdata
                        );
                    } else {
                        break;
                    }
                }
                //TODO even when done, check again, if a new batch has arrived in the meantime (we dont notice a further wakeups while this thread is running)
                thread::park(); // wait for unpark wakeup call from SN
            }
        }).expect("failed to start thread");
        // put thread handle into well-known place
        sn2ns_consumer_wakeup_give.lock().expect("failed to lock sn2ns_consumer_wakeup (giver)").replace(sn2ns_consumer_wakeup);

        // maintenance thread
        let echo_request_correlation_table_arc2 = self.echo_request_correlation_table.clone();
        let _ = thread::Builder::new().name("N CLNP".to_string()).spawn(move || {
            let timeout = chrono::Duration::seconds(5);
            loop {
                // clean up old Echo Request correlations
                {
                    let now = Utc::now();
                    let mut table = echo_request_correlation_table_arc2.lock().expect("failed to lock echo_request_correlation_table");
                    // NOTE: not possible to remove entries during iteration for loop, so we collect the keys to be removed and the remove them
                    let timedout: Vec<u16> = table
                        .iter()
                        .filter(|&(_, &v)| now - v > timeout)
                        .map(|(k, _)| k.clone())
                        .collect();
                    for key in timedout {
                        // remove old entry
                        info!("Echo request timed out");    //TODO dont know to which NSAP this timeout belongs
                        table.remove(&key);
                    }
                }   // release lock

                // sleep
                thread::sleep(Duration::from_millis(1000))
            }
        });
    }

    // 6.1
    // TODO WIP
    // TODO optimize - this function allocates CLNP PDUs for every call
    fn pdu_composition(&self, inactive: bool, ns_source_address: &'a Nsap, ns_destination_address: &'a Nsap, ns_quality_of_service: &'a Qos, ns_userdata: &'a [u8]) -> Vec<Pdu<'a>> {
        if inactive {
            return vec![Pdu::Inactive {
                fixed_mini: NFixedPartMiniForInactive { network_layer_protocol_identifier: &NETWORK_LAYER_PROTOCOL_IDENTIFIER_CLNP_INACTIVE },
                data: NDataPart { data: ns_userdata }
            }]
        } else {
            todo!();
        }
    }
}

//TODO
fn can_use_inactive_subset(ns_source_address: &Nsap, ns_destination_address: &Nsap) -> bool {
    // TODO check if on same subnetwork (AKA in same Ethernet segment)
    true
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