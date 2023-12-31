# osi-stack

ISO/ITU-T/IETF OSI (Open Systems Interconnection) stack implementation with application protocols.

Status:  Work in progress.

Currently featuring:

* parts of CLNP over ...
* Ethernet Subnetwork Service
* via Ethernet2 header.
* Ethernet SN:
  * PDUs implemented.
  * Functions working, but need to check conformance.
  * Primitives implemented (SN Unitdata request, SN Unitdata indication).
* CLNP NS:
  * Most CLNP NS PDUs implemented (Data PDU, Echo Request, Echo Response).
  * Functions partly implemented
  * Primitives mostly implemented.
  * Echo Request and Echo Response handling and "ping" roundtrip
* Simple static resolving of system-title based NSAP to SNPA address.
* Simple OSI ping application

Working on:

* Finishing CLNP primitives (N unitdata request and indication).
* Finishing CLNP functions in detail.
* Finishing SN functions.
* CLNP options part detailed support.
* CLNP non-segmenting subset and then
* CLNP full protocol.

Goal:

* Full coverage of the OSI protocols up to CASE (ACSE, RTSE, ROSE).
  * Maybe support for connection-oriented Data Link/Subnetwork Service and Connection-oriented Network Service.
* Routing protocols ES-IS, IS-IS and IDRP.
* LLC header support.
* Option to use IP suite of protocols for carrying OSI PDUs
* Carrying IPv4+IPv6 payload in routing protocols and in NLPID.
* Management support.
* Transport Layer Security, Network Layer Security.
* Application-layer protocols FTAM, VT, ...
* X.400
* X.500
* X.509 (note, consider https://en.wikipedia.org/wiki/X.509#Security)
* X.400 EDI
* X.601
* X.605
* X.609
* and others in X-Series
* Linux kernel module for layer 2,3,4
* multi-platform support (Linux, Mac, Windows)

Related:

* SS7 support (built on OSI)
* ATN support (built on OSI, transitioning from OSI as base to IP in newer airplanes)
* [OPC UA](https://en.wikipedia.org/wiki/OPC_UA) (running via TCP, but may run via TP4 as well)
* energy exchange protocols

Written in Rust, with planned binding for C and further, more programming languages.

## License

GNU LGPLv3+

## Contributing

1. open an issue or pick an existing one
2. discuss your idea
3. send pull request
4. quality check
5. merged!
