# Iteration 4


## Plan

* Achieve ability for two systems to send and receive Echo Requests and Echo Responses to each other.
* Perform necessary code restructuring to allow that.
* Add Echo Response function to actually respond to incoming Echo Request PDUs.
* Add Echo Request correlation table.
* Add CLNP NS maintenance thread to clean up timed-out Echo Requests from correlation table.
* Add wake calls to handler threads instead of spinning or sleeping x milliseconds.
* Add handover of the handler thread joinhandles.
* Add popping multiple events off the inter-layer connections.


## Do

* The chosen architecture are bounded ringbuffers, one for each direction between SN and NS.
* Rust is clunky in regards to tracing which fields of "self" are actually touched in a method - it just locks whole self. This leads to requiring inner mutability and passing the required fields from self via parameters. I hope that long-term, there is a better way. Maybe the Application layer is the top object can hierarchically own all layers below it, after all.
* Anyway, it works for now. Looking forward to see how this new architecture will behave when a layer needs certain internal state - without access to self.


## Check

* The iteration led to the goals being met, with round-trip times in the same order of magnitude as calling the ping command with the kernel-integrated IP stack, for example IP 350/400 ms, osi-stack 500/700ms without any optimizations - to the contrary.


## Act

* The internal structure and issue of lifetimes still needs to be improved as an implementation try of Clnp::echo_request() using n_unitdata_request() as its backend has failed.
* Restructuring should have a direction, ie. be based on requirements from the standard, so the next step is to implement further Network Service functions and adjust the structure accordingly, as well as clean up along the way.