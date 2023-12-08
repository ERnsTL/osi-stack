## Plan

Goal:

* Set up source code structure, git, library, first command etc.
* Send and receive CLNP-tagged packets on data link level (layer 2) via Ethernet.
* Do that probably using a TAP device, since the program would synthesize packets in formats which are unknown to the Linux kernel.
* Need a way to create a TAP device, assign a MAC address to it.
* Send and receive packets from/to TAP device.
* Parse Ethernet frames.

## Do

* CLNP has two possible encapsulations on Layer 2:
  * Inside IEEE 802.2 Ethernet frames with LLC ID 0xFE which is the native format to carry CLNP PDUs. (Or via LLC+SNAP extension header, which can carry an EtherType.)
  * Inside IEEE 802.3 Ethernet2 ("Ethernet II") frames, but there is only a Memo detailing the use of EtherType 0x8872 which is to date unlisted in the list of EtherTypes kept by IANA (note, that this list does not include the common 0x0800 EtherType for IPv4 as well).

* Found out that sending and receiving IEEE 802.2 frames especially with LLC is not well-supported in programming libraries. Also the parsing is klunky - LLC header, possibly SNAP extension header, VLAN extension header. Ethernet2 is the modern format, where only the possibility of a VLAN extension header has to be dealt with.

* Found suitable Ethernet2 packet parsers, but parsers dont serialize well ;-) so filtered for parser+serializer.

* Found suitable TAP device crate for Rust, but this does not handle setting addresses, bringing the interface up etc. So either this will be done by Network-Manager or some ifup/ifdown scripts, but this needed a better solution.

  * Developer information on TUN/TAP device programming:  https://backreference.org/2010/03/26/tuntap-interface-tutorial/index.html

* Found that there are ioctl's for setting MTU, MAC address (DLSAP), adding and removing IP addresses, starting the interface up or shutting it down.

* Engaged the Ethernet2 packet (DLPDU) parser and also sent some packets in return.

* Problem, these packets were sent, but did not get forwarded from tap0 to tap1 AKA from one program instance (OSI stack) to the other.

* But they are visible in packet logger, so they must get dropped somewhere:
  ```
  sudo tcpdump -i tap1 -v -v
  ```

* Show dropped packets:  https://www.cyberciti.biz/faq/linux-show-dropped-packets-per-interface-command/
  ```
  sudo watch ip -s link show tap0
  ```

* This shows that indeed all packets received from the tap interface into the kernel were dropped :-(

* But why do they get dropped?

* Found good tools for finding out why packets get dropped:

  * https://developers.redhat.com/articles/2023/07/19/how-retrieve-packet-drop-reasons-linux-kernel
  * https://serverfault.com/questions/1015896/linux-server-dropping-rx-packets-in-netif-receive-skb-core
  * https://optiver.com/working-at-optiver/career-hub/searching-for-the-cause-of-dropped-packets-on-linux/
  * https://blogs.oracle.com/linux/post/taming-tracepoints-in-the-linux-kernel

* These tools use kernel tracepoints, a bit like Solaris dtrace and Plan9front dtracy. The events then get logged in certain /proc files.
* Start tracing with the "perf" tool, which needs to fit the currently-running kernel version exactly. Relevant is the skb (Socket Kernel Buffer = buffer for a packet) when it frees up such a buffer AKA when it gets dropped. Alternatives are BPF and Dtrace.
  ```
  perf record -e skb:kfree_skb
  ```
* Or, just do it via sysfs files ;-) instead of learning many parameters of the perf tool and seeing that it matches with the kernel versions.
  ```
  echo 1 > /sys/kernel/debug/tracing/events/skb/kfree_skb/enable
  ```
* Then watch the outputs:
  ```
  tail -F cat /sys/kernel/debug/tracing/trace_pipe
  ```
* Dropwatch is also a useful tool for diagnosing packet drops, but it was not needed.

* It shows in which kernel function the packets get dropped and for what reason.
* In this case it was UNHANDLED_PROTO - it seems nobody feels responsible for this protocol - meaning for this EtherType.

* Quickly find the kernel source tree:  on the skb (Socket Kernel Buffer = buffer for a packet) wh
* git clone it via --depth 1 and grep in the kernel tree for "UNHANDLED_PROTO".
* It happens in net/core somewhere:

```
/dev/shm/linux$ grep -R UNHANDLED_PROTO *
include/net/dropreason-core.h:	FN(UNHANDLED_PROTO)		\
include/net/dropreason-core.h:	/** @SKB_DROP_REASON_UNHANDLED_PROTO: protocol not implemented or not supported */
include/net/dropreason-core.h:	SKB_DROP_REASON_UNHANDLED_PROTO,
net/ipv6/ip6_input.c:		SKB_DR_SET(reason, UNHANDLED_PROTO);
net/ipv6/exthdrs.c:					 SKB_DROP_REASON_UNHANDLED_PROTO);
net/ipv6/exthdrs.c:	kfree_skb_reason(skb, SKB_DROP_REASON_UNHANDLED_PROTO);
net/ipv4/icmp.c:		reason = SKB_DROP_REASON_UNHANDLED_PROTO;
net/core/dev.c:		kfree_skb_reason(skb, SKB_DROP_REASON_UNHANDLED_PROTO);
/dev/shm/linux$ less net/core/dev.c 
```

* I did not analyze the code properly, but it seems Linux does not have a handler registered for this protocol.

* Tried to add the protocol to the list of EtherTypes in /etc/ethertypes, but also did not help.
  * Note, the IP-based protocols are listed in /etc/protocols see https://www.man7.org/linux/man-pages/man5/protocols.5.html

* Logical method to back off towards the well-trodden path of sending IP packets, just sending them raw this time. Sending some IPv4 header and this time it shows drop because of OTHERHOST. Meaning the sender and receiver IP addresses are not known and belong to some "other host", because the given fake IP address in the raw packet is not known on any local interface.
* Good, so the TAP direction generally works, but it does not work on 0x8872 EtherType packets.

* I just want the Ethernet packet to be routed out into the LAN and received on the other interface.
* Expectation is that, if the Layer 2 route to the destination is not known, that the packet is flooded out on all interfaces (link domain / broadcast domain), hoping to reach its destination. But that does not happen. I dont want Linux to "handle" the packet, just forward it stupidly on Layer 2!

* Tried solving it via various bridge interface trickery, connecting eth0 and tap0 to a br0 bridge interface and hoping that it would get routed.

* Linux ip link bridge:  https://developers.redhat.com/articles/2022/04/06/introduction-linux-bridging-commands-and-features
  * Interesting, but did not solve the issue.
* Side note, it is not easily possible to enslave/add a WLAN ("wifi") interface to a bridge, because the WLAN access point expects only Ethernet packets from our own DLSAP/ MAC address and not some forwarded packets: https://serverfault.com/questions/152363/bridging-wlan0-to-eth0

* Tried adding a manual entry to the kernel layer 2 Forwarding DB (FDB), which is a table of known "this MAC address is reachable via this interface".
  * sudo bridge fdb add de:ad:be:ef:11:11 dev tap0 via wlp1s0
  * man page:  https://www.man7.org/linux/man-pages/man8/bridge.8.html
* But this also did not work.

* I tried using some iptables trickery - but iptables does not support matching on EtherType.
* Linux filter/forward chains:  https://serverfault.com/questions/1046353/how-does-iptables-filtering-in-the-forward-or-input-chain-interact-with-nat
  * iptables ethertype filter impossible:  https://serverfault.com/questions/1019460/how-can-i-use-iptables-to-drop-packages-for-an-invalid-ether-type
  * thus using nftables
* Switched to nftables:  https://serverfault.com/questions/1015896/linux-server-dropping-rx-packets-in-netif-receive-skb-core#1016113
  * In the hopes of setting "forward via this interface XY".
  * nftables = successor to iptables, which unifies iptables (IPv4 only), ip6tables (IPv6 only), arptables (only for ARP cache and routing) and ebtables (only for Layer 2 routing), and tc (traffic conditioning / traffic shaping) as well as ethertool to some degree, as well as a few other tools.

* Found a nice graphic on what paths network packets go through the Linux kernel:  https://wiki.nftables.org/wiki-nftables/index.php/Netfilter_hooks

* I took a step back and said, basically we want to "bind" to an interface:
  * bind to an IP address on layer 3 using a socket and a certain layer 4 TCP/UDP port
  * bind to an Ethernet interface on layer 2 using a certain "port" = DLSAP, which is the EtherType.
* Does such a thing exist?

* Magic, that actually exists. It is called a "raw" socket, why on Earth is it called that. Lets just call it layer 2 socket or data link socket or whatever. Anyway, "raw" means Ethernet layer 2 level.
* Solution:  https://stackoverflow.com/questions/3366812/linux-raw-ethernet-socket-bind-to-specific-protocol
  * fd = socket(PF_PACKET, SOCK_RAW, htons(MY_ETH_PROTOCOL));
* This way it should be possible to tell the kernel "I am for now the responsible protocol handler for EtherType 0x8872 on interface tap0" and let us hope that the kernel then knows how to further forward these Ethernet packets.

* But it makes sense somehow:  The TAP device is in full promiscuous mode and we did not register with the kernel. It does not know about which EtherTypes we are working with, whether we send layer 2 packets or really layer2+3+4 (Ethernet + IP + UDP) and just assemble the packets manually in userspace for some whatever reason...
* Trying these "raw" sockets.

TODO To be continued...