# osi-stack

ISO/ITU-T/IETF OSI (Open Systems Interconnection) stack implementation with application protocols.

Status:  Work in progress.

Currently featuring:

* parts of CLNP over ...
* Ethernet Subnetwork Service
* via Ethernet2 header.
* Simple static resolving of system-title based NSAP to SNPA address.
* Simple OSI ping application

Working on:

* CLNP non-segmenting subset and then
* CLNP full protocol.

Goal:

* Full coverage of the OSI protocols up to CASE (ACSE, RTSE, ROSE).
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
