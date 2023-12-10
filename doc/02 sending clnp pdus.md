## Plan

Goal:

* Analyze X.233 standard document and structure the work.
* Add CLNP PDU structs and its parts, minus options parameter details.
* Add sub-bit handling of fixed part of NPDU.
* Add header format analysis function for inactive protocol subset.
* Add PDU composition function for Inactive Protocol subset.
* Add PDU decomposition function for Inactive Protocol subset.
* Send NPDU (CLNP inactive protocol subset) instead of empty Ethernet frame.
* Parse CLNP instead upon reception.

References:

* https://www.itu.int/rec/T-REC-X.233/en specifically https://www.itu.int/rec/T-REC-X.233-199708-I/en
* for NSAP syntax and semantics, multicast:  https://www.itu.int/rec/T-REC-X.213/en specifically https://www.itu.int/rec/T-REC-X.213-200110-I/en

* Copied the full standard analysis to next document (03) to continue working on it.

Document research:

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

* For Inactive CLNP only a few clauses are really needed, that is good.
* It is a nice that the implementation is pretty straightforward using the standards structure.
* It seems unclear what the integration between "the expeted lower datalink protocol" and the network service is.
* There seems to be no ARP. It seems, OSI pushes this function to a Directory instead of having a separate low-level layer 2/3 local resolution protocol (ARP).
* Many datastructures are needed to parse the various protocol structs, even if they are not used yet. But then, the infrastructure is there already to build on.


## Check

* Can send and receive CLNP PDUs with the inactive protocol subset.
* Simple lookup works.
* Composition and decomposition is implemented in basic form.

In terms of standard clauses:

* Implemented 5.2 Inactive Network Layer Protocol Subset
* 5.3.1 basic NSAP format
* 5.3.2 network entity titles very basic in-memory hostsfile-like "resolution" of host-titles
* 5.4 N-UNITDATA request and indication functions
* 5.5 underlying service (data link) assumed implemented (raw sockets) but not abstracted yet
* 6.22 checked what must be implemented for Inactive protocol subset
* 6.1 minimal version of PDU composition function
* 6.2 minimal version of PDU decomposition function
* 6.3 most of Header analysics function for inactive protcol subset support
* 7.1 - 7.7 data structures for fixed part, "mini fixed part" for inactive network protocol subset, address, segmentation, options, data part


## Act

* Nothing to do.