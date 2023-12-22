# Iteration 3


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


## Check

* Much was achieved in this iteration: A significant part of the non-segmenting CLNP protocol subset was achieved, at its core the PDU composition and decomposition functions, especially for Echo Request PDU and Echo Response PDU, and much improved structure and abstraction between services.
* Limitations in the current call structure were discovered and limit moving forward.

* Currently, the ownership structure of structs is made so that it does not include circles, according to Rust ownership rules, which are mostly hierarchical. As soon as sharing and multiple ownership are involved, then either Mutexes are needed or splitting into multiple ownership domains, which communicate using a channel or shared memory or similar. This will have to be the next step, because even receiving an N Echo Request needs to be able to also send a response, thereby creating a loop back towards the underlying SN (Subnetwork).
* It is not possible to realize the required timers, correlation between segmented PDUs and sendig using a stack-based call structure going in one directon -
  1. from application down to the Subnetwork and
  2. from Subnetwork only propagating up.
* There have to be connections and loops, for example the echo request function sending back a an echo response.
* Therefore, the goal of ability to send an Echo Response NPDU in response to an Echo Request NPDU has to be achieved using restructuring of the code.
* The layers have to have their own calling stacks, have to be able to on their own triggers.

* It is also not possible to have a reader thread responsible for receiving SN PDUs (Ethernet packets) and simply calling functions deeper up the stack and waiting until that Ethernet packet is fully processed and a response has come back from up in the stack.


## Act

* The plan for the next iteration has to be adjusted, to first create the according communication structure between layers in order to allow layers to be active on their own.
* The stack has to become multi-threaded, thus with multiple active objects, one for each service.