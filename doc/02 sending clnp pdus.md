## Plan

Goal:

* Define CLNP PDU struct.
* Add PDU serialization.
* Send PDU instead of empty Ethernet frame.

References:

* https://www.itu.int/rec/T-REC-X.233/en specifically https://www.itu.int/rec/T-REC-X.233-199708-I/en
* for NSAP syntax and semantics, multicast:  https://www.itu.int/rec/T-REC-X.213/en specifically https://www.itu.int/rec/T-REC-X.213-200110-I/en

1. General requirements
2. inactive
3. non-segmenting
4. full

* 5.2
  * Inactive Network Layer Protocol Subset
  * Non-segmenting Protocol Subset
* 5.3.1
  * NSAP addresses in preferred format.
  * multicast optional, if multiast then destination address = a group NSAP and source != group NSAP.
* 5.4
  * service primitive (API)
  * NSDU max length = 64512 octects
  * N-UNITDATA request and indication, parameters:
    * NS-Source-Address
    * NS-Destination-Address
    * NS-Quality-of-Service
    * NS-Userdata
* ... TODO
* 7.1
  * big endian
  * 

## Do

...

