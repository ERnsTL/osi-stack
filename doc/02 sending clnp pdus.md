## Plan

Goal:

* Analyze X.233 standard document and structure the work.
* Add PDU composition for Inactive Protocol subset.
* Add PDU decomposition function for Inactive Protocol subset.
* Add header analysis function for inactive protocol subset.
* Send NPDU (CLNP inactive protocol subset) instead of empty Ethernet frame.
* Parse CLNP instead upon reception.
---
03:
* Change implementation to start of Non-Segmenting Protocol subset:
* Add Echo Request function.
* Add Echo Response function.
* Add Error Reporting for everything else.
* Send echo request, echo response.

References:

* https://www.itu.int/rec/T-REC-X.233/en specifically https://www.itu.int/rec/T-REC-X.233-199708-I/en
* for NSAP syntax and semantics, multicast:  https://www.itu.int/rec/T-REC-X.213/en specifically https://www.itu.int/rec/T-REC-X.213-200110-I/en

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

...

