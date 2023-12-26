# Iteration 5


## Plan

* Change clnp echo_request() to use n_unitdata_request().
* Find out it echo request function should use n-unitdata-request primitive, because latter one does not have parameter to select PDU type.
---
* Solving 2 borrows needs to be done:
  * [Thread 1](https://www.reddit.com/r/rust/comments/ah6fhi/mutably_borrowing_two_things_simultaneously_from/)
  * [Thread 2](https://stackoverflow.com/questions/70050258/multiple-mutable-borrows-in-rust)
  * For example ns.get_serviced_nsap() and ns.echo_request() results in 2 borrows on ns :-(
* Implement correct PDU composition function for Inactive Protocol subset.
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

* ...