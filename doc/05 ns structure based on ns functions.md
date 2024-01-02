# Iteration 5


## Plan

* Finish research for requirements in whole of X.233 (CLNP) in order to get overview of requirements and base to-be architecture on that.
* Change clnp echo_request() to use n_unitdata_request().
* Find out it echo request function should use n-unitdata-request primitive, because latter one does not have parameter to select PDU type.
* Implement 6.1 PDU Composition function.
* 6.1 DUID tracking on n-unitdata-request initial PDUs and derived PDUs (segmentation)

Further:

* Solving 2 borrows needs to be done:
  * [Thread 1](https://www.reddit.com/r/rust/comments/ah6fhi/mutably_borrowing_two_things_simultaneously_from/)
  * [Thread 2](https://stackoverflow.com/questions/70050258/multiple-mutable-borrows-in-rust)
  * For example ns.get_serviced_nsap() and ns.echo_request() results in 2 borrows on ns :-(
* Implement correct PDU composition function for Inactive Protocol subset .
* Implement correct PDU decomposition function for Inactive Protocol subset.
* Implement correct Header Analysis function for Inactive Protocol subset.
* Check if Inactive Protocol subset should report errors - if ER bit is set, send an error report? But in 6.22 functions table it is not listed that error reports are mandatory for the Inactive subset.

06:

* Change implementation to start of Non-Segmenting Protocol subset:
* Add full Echo Request function.
* Add full Echo Response function.
* Add Error Reporting for everything else.
* Send echo request, echo response.

07:

* Add full struct and decomposition of options part resp. parameter meanings.
* Implement full protocol support.

08:

* Add dummy SN or dummy raw socket in order to allow performance measuring and tuning as well as testing.

09:

* Add "osi hosts.txt" support.
* Add correct NSAP support <-> quick parsing of 49.1.1.macaddress

10:

* Add discovery of link-local hosts reachable via Ethernet. This is ES-IS protocol and will be added later.
* Allow parallel receiving and sending via inner_read and inner_write structs.
* What about ARP and DNS to find hosts on the network?
* Keeping track of connections DID (? TODO find it again)
* Implementation of ES-IS protocol (subnetwork coordination? finding other hosts and routes?).
* Do End systems (ES) also send Hello's to each other? What do they do when they receive such and no Intermediate System (IS AKA Router) is present?
  * Yes, they do -> ISO 9542 availabe for free via ISO "Publicly Available Standards".

11:

* Routing table dump as in [RFC 1574](https://datatracker.ietf.org/doc/rfc1574/)

xx:

* add protocol server to be contacted via library and unix socket or should each application create its own stack? well, the network layer handles the network protocol for the whole system and we cannot have 1,2,3,5,10 sockets all bound to the Ethernet interface, each receiving the same packets and handling them x times.

Document research in X.233:
Goal:  list of requirements (must, shall, may) for implementation work packages
Goal:  list of tests (PICS)

* 1 scope (TODO research, TODO impl):
* 2 normative references (TODO research, TODO impl):
* 3 definitions (TODO research, TODO impl):
* 4 abbreviations (TODO research, TODO impl):
* 5 overview of the protocol (TODO research, TODO impl):
* 5.1 internal organization of the network layer (TODO research, TODO impl):
* 5.2 subsets of the protocol (TODO research, TODO impl):
  * Inactive Network Layer Protocol Subset
  * Non-segmenting Protocol Subset
* 5.3.1 addresses (TODO research, TODO impl):
  * NSAP addresses in preferred format.
  * multicast optional, if multiast then destination address = a group NSAP and source != group NSAP.
* 5.3.2 network entity titles (TODO research, TODO impl):
* 5.4 service provided (TODO research, TODO impl):
  * service primitive (API)
  * NSDU max length = 64512 octects
  * N-UNITDATA request and indication, parameters:
    * NS-Source-Address
    * NS-Destination-Address
    * NS-Quality-of-Service
    * NS-Userdata
* 5.5 underlying service assumed (TODO research, TODO impl):
  * assumed underlying "subnetwork protocol" = Ethernet in most cases, but this does not have QoS and is the same as the DL service. described in clause 6.
  * TODO convergence function? research.
* 6 protocol functions (TODO research, TODO impl):
  * reference to table what must be implemented in 6.21 (TODO actually 6.22)
* 6.1 pdu composition function (research DONE, impl part see TODOs)
  * Construction of PDU according to encoding rules in clause 7.
  * TODO paragraph 2 - NPAI (Network Protocol Address Information) for source and destination fields is derived from NS-Source-Address and NS-Destination-Address. But how?
  * TODO DUID tracking:
    * "During the composition of the protocol data unit, a Data Unit Identifier (DUID) is assigned to distinguish this request to transmit NS-Userdata to a particular destination Network service user or users from other such requests. The originator of the PDU shall choose the DUID so that it remains unique (for this source and destination address pair) for the maximum lifetime of the Initial PDU in the network; this rule applies for any PDUs derived from the Initial PDU as a result of the application of the segmentation function (see 6.7). Derived PDUs are considered to correspond to the same Initial PDU, and hence to the same N-UNITDATA request, if they have the same source address, destination address, and data unit identifier."
    * "The DUID is also available for ancillary functions such as error reporting (see 6.10)"
    * Which means error reporting always pertains to a full PDU transmission - it cannot reference a part of a PDU.
  * Total length:
    * "The total length of the PDU in octets is determined by the originator and placed in the total length field of the PDU header. This field is not changed for the lifetime of the protocol data unit, and has the same value in the Initial PDU and in each of any Derived PDUs that may be created from the Initial PDU."
    * "When the non-segmenting protocol subset is employed, neither the total length field nor the data unit identifier field is present. The rules governing the PDU composition function are modified in this case as follows. During the composition of the protocol data unit, the total length of the PDU in octets is determined by the originator and placed in the segment length field of the PDU header. This field is not changed for the lifetime of the PDU. No data unit identification is provided."
    * So, DUID is only needed for the full protocol.
* 6.2 pdu decomposition function (DONE research, TODO impl)
  * TODO NPAI - "The NS-Source-Address and NS-Destination-Address parameters of the N-UNITDATA indication are recovered from the NPAI in the source address and destination address fields of the PDU header."
    * Crossreference: Connection between NPAI, NSAP address, Network Address, Network addresses, Subnetwork address, SNPA, SNPA address are cleared in X.213 Network Service Definition in A.3 Concepts and terminology, but nothing about NPAI in X.200 Basic Model.
    * X.213 A.3.1.1 Subnetwork address in Note: SNPA address is NOT an NSAP, clearly.
  * TODO "The data part of the received PDU is retained until all segments of the original service data unit have been received; collectively, these form the NS-Userdata parameter of the N-UNITDATA indication."
  * TODO QoS "Information relating to the Quality of Service (QOS) provided during the transmission of the PDU is determined from the quality of service and other information contained in the options part of the PDU header. This information constitutes the NS-Quality-of-Service parameter of the N-UNITDATA indication." - TODO what is taken from the options part?
* 6.3 header format analysis function (DONE research, TODO impl)
  * Summary: Determine protocol subset, then check if destination has been reached.
  * Determine protocol subset:
    * If full protocol or inactive subset is in use.
    * based on NLPID field.
    * if CLNP ID is given -> either full protocol or non-segmenting subset is in use (but we dont know which one the sender is using just from the PDUs).
  * If full or non-segmenting, then determine if final destination has been reached:
    * "using the destination address in the PDU header."
    * "If multicast transfer is not supported and if the destination address provided in the PDU identifies either a Network entity title of this Network entity or an NSAP served by this Network entity, then the PDU has reached its destination; if not, it shall be forwarded." TODO derive exact logic from that
  * If inacive protocol subset:
    * if NLIP protocol ID = inactive protocol ID, then no further header analysis.
    * Determines that either a) the Subnetwork Point of Attachment (SNPA) address encoded as NPAI in the supporting subnetwork protocol (see 8.1) corresponds directly to an NSAP address serviced by this Network entity, or b) that an error has occurred."
  * For all protocol variants: Check for multicast source address:
    * If a Network entity supports multicast transmission, check that PDU does not contain a group Network address in the source address field. Any PDU header analysed to have a group address in the source address field shall be discarded.
* 6.3.1 Multicast transfer (TODO research, TODO impl):
  * ...TODO not sure what to make of this paragraph, seems to be an optional feature
* 6.4 PDU lifetime control function (DONe research, TODO impl):
  * Decide if a) it should be forwarded or b) if TTL has expired, and it should be discarded.
  * lifetime field in the PDU header, put there by the originating NE
  * if segmentation function is applied to a PDU, then put the same TTL value into initial and derived PDUs
  * lifetime is decremented by every NE, which processes the PDU
  * decrement by >= 1
  * "decrement by more than 1 if 
    * the transit delay in the underlying service from which the PDU was received; and
    * the delay within the system processing the PDU
  * exceeds or is estimated to exceed 500 ms. In this case, the lifetime field shall be decremented by one for each additional 500 ms of actual or estimated delay."
  * if lifetime would be negative, place value 0
  * if lifetime If the lifetime field reaches a value of zero before the PDU is delivered to its destination, the PDU shall be discarded.
    * and The error reporting function shall be invoked as described in 6.10. This may result in the generation of an Error Report PDU.
  * It is a local matter whether or not the destination Network entity performs the lifetime control function. So, need a configuration option for that.
* 6.5 Route PDU function (TODO research, TODO impl):
  * ...
* 6.6 Forward PDU function (TODO research, TODO impl):
  * ...
* 6.7 Segmentation function (TODO research, TODO impl):
  * ...
* 6.8 Reassembly function (TODO research, TODO impl):
  * 
* 6.9 Discard PDU function (TODO research, TODO impl):
  * ...
* 6.10 Error reporting function (TODO research, TODO impl):
  * ...
* 6.11 PDU header error detection function (TODO research, TODO impl):
  * ...
* 6.12 Padding function (TODO research, TODO impl):
  * ...
* 6.13 Security function (TODO research, TODO impl):
  * ...
* 6.14 Source routeing function (TODO research, TODO impl):
  * ...
* 6.15 Record route function (TODO research, TODO impl):
  * ...
* 6.16 Quality of service maintenance function (TODO research, TODO impl):
  * ...
* 6.17 Priority function (TODO research, TODO impl):
  * ...
* 6.18 Congestion notification function (TODO research, TODO impl):
  * ...
* 6.19 Echo request function (TODO research, TODO impl):
  * ...
* 6.20 Echo response function (TODO research, TODO impl):
  * ...
* 6.21 scope control function (TODO research, TODO impl):
  * ...
* 6.22 classification of functions (TODO research, TODO impl):
  * important table
  * type 1,2,3 functions TODO
  * inactive subset:
    * 6.1 pdu composition
    * 6.2 pdu decomposition
    * 6.3 header format analysis
  * non-segmenting subset:
    * TODO
  * full protocol:
    * TODO
* 7 structure and encoding of PDUs (TODO research, TODO impl):
* 7.1 structure (TODO research, TODO impl):
  * big endian
  * with exception of inactive subset, PDUs shall contain:
    1. fixed part = 7.2
    2. address part = 7.3
    3. segmentation part, if present = 7.4
    4. options part, if present = 7.5
    5. reason-for-discard parameter (ER PDU only), if present (? TODO clause)
    6. data part, if present = 7.6
  * points 1 to 5 = PDU header
  * inactive subset:
    * only 7.8 present
    * 7.2 to 7.5 do not apply = only 7.8 (mini-fixed part) and 7.6 (data)
* 7.2 fixed part (TODO research, TODO impl):
  * TODO
* 7.3 address part (TODO research, TODO impl):
  * TODO
* 7.4 segmentation part (TODO research, TODO impl):
  * TODO
* 7.5 options part (TODO research, TODO impl):
  * TODO
* 7.6 data part (TODO research, TODO impl):
  * TODO
* 7.7 data PDU (!) = DT PDU, shows total packet structure (TODO research, TODO impl):
  * TODO
* 7.8 inactive protocol subset header (TODO research, TODO impl):
  * TODO
* 7.9 error report PDU = ER PDU (TODO research, TODO impl):
  * TODO
* 7.10 echo request PDU = ERQ PDU (TODO research, TODO impl):
  * TODO
* 7.11 echo response PDU = ERP PDU (TODO research, TODO impl):
  * TODO
* 7.12 multicast data PDU = MD PDU (TODO research, TODO impl):
  * TODO
* 8 provision of the underlying service (TODO research, TODO impl):
  * TODO
* 9 conformance (TODO research, TODO impl):
  * TODO
* annex A - PICS proforma
  * TODO

Abbreviations:

* Connection-oriented, connection-mode, connectionless-mode:
  * CONS = Connection-oriented NS, but is called Connection-Mode NS. And the Protocol is called CLNP (Connection-less Network Protocol) and ... no such protocol for the connection-mode network service, this would be for example provided by ISDN or X.25 - see X.223 "Use of X.25 to provide the OSI connection-mode network service for ITU-T applications", which also contains the 4.1 Network service abbreviations:  CONS = Connection-Mode Network Service. Ha, found an inconsistency.
  * Also see, X.213 7. Types and classes of network service: "There are two types of Network Service:
    a) a connection-mode service (defined in Section 2); and
    b) a connectionless-mode service (defined in Section 3)."
  * Then there is the "connection-oriented" Session and Presentation Protocol as well as "connection-less mode" versions of these two.
  * Also, there is a connection-oriented and connectionless-mode Transport Service mentioned in X.225 Connection-oriented Session Protocol 5.3 Services assumed from the transport layer referencing X.214 TS, but X.214 calls the two variants section 2 "connection-mode service" and section 3 "connectionless-mode service".
  * Connectionless-Mode is often abbreviated "Connectionless".
  * Neither X.200 (basic reference model), X.210 (basic reference model: conventions for the definition of OSI services), neither X.233 (CLNP) nor X.213 (NS) mention an explicit abbreviation for the connection-mode network service, but all never mention "connection-oriented" only "connection-mode" network service.
  * So the consistent naming schema would be "CM" and "CL", despite the wording "connection-oriented" showing up in offical standard names.
* How to define OSI-services, see X.210 Annex F pp.


## Do

* ...
