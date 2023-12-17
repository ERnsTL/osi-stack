## Plan

Goals:

* Move CLNP and Ethernet source code into modules.
* Add Network Service trait which the CLNP implements in order to make the Network Service exchangeable - since IS-IS and ES-IS etc. are also on the network layer.
* Correct abstraction and encapsulation of Data Link Layer (Subnetwork Service) instead of being built-in into network service.
* Request and indication between NS and SN - request for outgoing, indications for incoming.
  * X.233 clause 8.4 says that the requests go down the stack and indications go back up the stack.
* Pull up NS and SN protocol support and internal architecture by implementing an incomplete version of Echo Request and Echo Response.
* Add NPDU Echo Request and Echo Response composition, which incidentially is also for any normal Data PDU and possibly even Multicast Data PDU.
* Change implementation to start of Non-Segmenting Protocol subset by working on Echo Request and Echo Response.
* Add first Echo Request function and all the dependencies needed to achieve that.
* Add first Echo Response function and all the dependencies needed to achieve that.
* Achieve ability for two systems to send and receive Echo Requests and Echo Responses to each other.

---

* Solving 2 borrows needs to be done:  https://www.reddit.com/r/rust/comments/ah6fhi/mutably_borrowing_two_things_simultaneously_from/
  * For example ns.get_serviced_nsap() and ns.echo_request() results in 2 borrows on ns :-(
* TODO the subnetwork service must be an active component (thread) blocking on socket.read() and being able to propagate up the stack.
  * read up is active
* TODO the network service is called by the upper layers "please deliver this", so it can be a dead method/function.
  * push down is active
* TODO certain services need to be active in and of themselves to check on timers, purge routing tables ...
  * services do periodic tasks themselves
-> 2 threads per layer.
* and we will need mailboxes - shared memory handover of PDUs.
   * down the stack we package &upperpdu into larger PDU and pass it & down the stack. Upon write() the kernel makes a copy and returns the buffer.
   * up the stack we trim things from the buffer - by creating an ever smaller view/slice into the layer2 buffer. The buffer remains in ownership of the SubnetworkService. Arriving in destination layer, the layer makes a copy and returns the buffer.

* add protocol server to be contacted via library and unix socket or should each application create its own stack? well, the network layer handles the network protocol for the whole system and we cannot have 1,2,3,5,10 sockets all bound to the Ethernet interface, each receiving the same packets and handling them x times.

* Implement correct PDU composition function for Inactive Protocol subset.
* Implement correct PDU decomposition function for Inactive Protocol subset.
* Implement correct Header Analysis function for Inactive Protocol subset.
* Check if Inactive Protocol subset should report errors - if ER bit is set, send an error report? But in 6.22 functions table it is not listed that error reports are mandatory for the Inactive subset.
* Add "osi hosts.txt" support.
* Add correct NSAP support <-> quick parsing of 49.1.1.macaddress
* Add discovery of link-local hosts reachable via Ethernet.
* Allow parallel receiving and sending via inner_read and inner_write structs.
* What about ARP and DNS to find hosts on the network?
* Keeping track of connections DID (? TODO find it again)
* Routing table dump as in https://datatracker.ietf.org/doc/rfc1574/
* Implementation of ES-IS protocol (subnetwork coordination? finding other hosts and routes?).
* Do End systems (ES) also send Hello's to each other? What do they do when they receive such and no Intermediate System (IS AKA Router) is present?
  * Yes, they do -> ISO 9542 availabe for free via ISO "Publicly Available Standards".
---
04:
* Change implementation to start of Non-Segmenting Protocol subset:
* Add full Echo Request function.
* Add full Echo Response function.
* Add Error Reporting for everything else.
* Send echo request, echo response.
---
05:
* Add full struct and decomposition of options part resp. parameter meanings.
* Implement full protocol support.

Document research in X.233:
Goal:  list of requirements (must, shall, may) for implementation work packages
Goal:  list of tests (PICS)

* 1 scope
* 2 normative references
* 3 definitions
* 4 abbreviations
* 5 overview of the protocol
* 5.1 internal organization of the network layer
* 5.2 subsets of the protocol
  * Inactive Network Layer Protocol Subset
  * Non-segmenting Protocol Subset
* 5.3.1 addresses
  * NSAP addresses in preferred format.
  * multicast optional, if multiast then destination address = a group NSAP and source != group NSAP.
* 5.3.2 network entity titles
* 5.4 service provided
  * service primitive (API)
  * NSDU max length = 64512 octects
  * N-UNITDATA request and indication, parameters:
    * NS-Source-Address
    * NS-Destination-Address
    * NS-Quality-of-Service
    * NS-Userdata
* 5.5 underlying service assumed
  * assumed underlying "subnetwork protocol" = Ethernet in most cases, but this does not have QoS and is the same as the DL service. described in clause 6.
  * TODO convergence function? research.
* 6 protocol functions
  * reference to table what must be implemented in 6.21 (TODO actually 6.22)
* 6.1 pdu composition function
  * ...TODO
* 6.2 pdu decomposition function
  * TODO
* 6.3 header format analyses function
  * TODO
* 6.4 TODO
* 6.5 TODO
* 6.6 TODO
* 6.7 TODO
* 6.8 TODO
* 6.9 TODO
* 6.10 TODO
* 6.11 TODO
* 6.12 TODO
* 6.13 TODO
* 6.14 TODO
* 6.15 TODO
* 6.16 TODO
* 6.17 TODO
* 6.18 TODO
* 6.19 echo request function
  * ...TODO
* 6.20 echo response function
  * ...TODO
* 6.21 scope control function
  * ...TODO
* 6.22 classification of functions
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
* 7 structure and encoding of PDUs
* 7.1 structure
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
* 7.2 fixed part
  * TODO
* 7.3 address part
  * TODO
* 7.4 segmentation part
  * TODO
* 7.5 options part
  * TODO
* 7.6 data part
  * TODO
* 7.7 data PDU (!) = total packet structure
  * TODO
* 7.8 inactive protocol subset header
  * TODO
* 7.9 error report PDU (ER PDU)
  * TODO
* 7.10 echo request PDU (...TODO abbrev)
  * TODO
* 7.11 echo response PDU (TODO short name)
  * TODO
* 7.12 multicast data PDU (TODO abbrev)
  * TODO
* 8 provision of the underlying service
  * TODO
* 9 conformance
  * TODO
* annex A - PICS proforma
  * TODO

## Do

* Observation that Linux delivers us more bytes than the DL / Ethernet frame has. For example 60 bytes instead of 14 bytes Ethernet2 header + CLNP Inactive subset header + few bytes test data. Not sure if this is an artefact of the Rust library or Linux behavior. Should get sorted out by Transport protocol parsing, which includes a header length, I hope.
* Observation about the SNPA = der point of connection to the network AKA the Switchport on the local subnet, where Subnet is not in IPv4/6 meaning like a /24 subnet but OSI means the local Ethernet bridging domain on DL layer.
  * Read the introduction to ISO 9542 ES-IS, then it becomes clear.
  * In the end, this whole OSI thing is always about delivery of the requested payload + metainformation to and from the SNPA.
* The X.233 CLNP Specification in 5.5 Underlying service assumed by the protocol says that: "It is intended that this protocol be capable of operating over connectionless-mode services derived from a wide variety of real subnetworks and data links. Therefore, in order to simplify the specification of the protocol, its operation is defined (in clause 6) with respect to an abstract “underlying subnetwork service” rather than any particular real subnetwork service."
  * So the Subnetwork Service is situated on the Data Link Layer.
  * Ethernet provides a Subnet Service and is located on the Data Link Layer.
* On Linux/Unix, the reading of SN-UNITDATA indications is actually just reading from the socket, but it must be done in a preparatory converter method outside, because the parameters for the SN-UNITDATA primitive are not yet parsed AKA the DL PCI (header) with the source + destination address, EtherType are not parsed yet. Unfortunately, the Linux kernel nor libc do not directly call our OSI stack sn_unitdata_indication() method with the nicely-prepared parameters...
* Observation that Wireshark can be modified to use the CLNP dissector for EtherType 0x8872 via Lua. Nice.
* The OSI protocol byte-order is big-endian.