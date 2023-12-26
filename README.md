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
* Simple static resolving of system-title based NSAP to SNPA address.
* Simple OSI ping application

Working on:

* Full Echo Request and Echo Response handling
* Finishing CLNP primitives (N unitdata request and indication).
* Finishing CLNP functions.
* Finishing SN functions.
* CLNP options part detailed support.
* CLNP non-segmenting subset and then
* CLNP full protocol.

Goal:

* Full coverage of the OSI protocols up to CASE (ACSE, RTSE, ROSE).
  * Exceptions: Connection-oriented Subnetwork (because Ethernet is not connection-oriented but may be relevant for an other SN/DL providing the service - have to see), Connection-oriented Network Service (does that even exist?). (TODO Are these needed for any major Standards based on OSI?)
* Routing protocols ES-IS, IS-IS and IDRP.
* LLC header support.
* Use of IP suite of protocols for carrying OSI PDUs
* Carrying IPv4+IPv6 payload in routing protocols.
* Management support.
* Transport Layer Security, Network Layer Security.
* Application-layer protocols FTAM, VT, ...
* X.400
* X.500
* X.400 EDI
* Linux kernel module for layer 2,3,4
* multi-platform support (Linux, Mac, Windows)

Written in Rust, with planned binding for C and further, more programming languages.

## License

GNU LGPLv3+

## Contributing

1. open an issue or pick an existing one
2. discuss your idea
3. send pull request
4. quality check
5. merged!
