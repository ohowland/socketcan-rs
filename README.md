# SocketCAN

[SocketCAN Man Page]https://www.kernel.org/doc/Documentation/networking/can.txt

## Netlink
[Netlink Wikipedia]https://en.wikipedia.org/wiki/Netlink

My understanding of netlink is that it is used to configure communication between the userspace and the kernelspace. When using commands like `ip link set up can0`, I believe we are invoking netlink.

If we want to bring up the can0 interface, we'll need to interface with netlink. This is not a priority for us, in the past we've configured the machine to enable these interfaces at startup using systemd.  

- [ ] pull in information from gateway on what exactly we do to setup the can interface.


### Migrate netlink-rs to netlink
The author of the original socketcan library expresses concern about the state of the netlink-rs library. He actually doesn't end up using it in the lib.rs (he copys a few lines, but leaves the library in the .toml, ostesibly to credit work). the netlink library is used in unittests (cargo test -feature vcan-tests).

It seems like github.com/little-dude is currently leading the netlink charge.

### mem::MaybeUninit
There is a pattern used for dealing with uninitialied datastructures.

```
let mut ts = MaybeUninit::<T>::uninit(); 
err = unsafe { libc::ioctl(self.fd, SIOCGSTAMPNS as c_ulong, ts.as_mut_ptr()) }

if err == -1 { return Err(io:Error:last_os_error());}

let ts = unsafe { ts.assume_init() };
```

### Constants required to interface is ioctl
Author says he stole thiese from the C headers. Where are they used?

`const AF_CAN: libc::c_int = 29` The CAN address family  
`const PF_CAN: libc::c_int = 29` The CAN protocol family  
`const CAN_RAW: ...`  
`const SOL_CAN_BASE: ...`  
`const SOL_CAN_RAW: ...`  
`const CAN_RAW_FILTER: ...`  
`const CAN_RAW_ERR_FILTER: ...`  
`const CAN_RAW_LOOPBACK: ...`  
`const CAN_RAW_RECV_OWN_MSGS: ...`   
`const CAN_RAW_JOIN_FILTERS: ...`  

`const SIOCGSTAMPNS: ...` This is used in an ioctl call to get current system timestamp in ns.  


`pub const EFF_FLAG: u32` Extended frame flag  
`pub const RTR_FLAG: u32` Remote transmission request flag
`pub const ERR_FLAG: u32` Error flag
`pub const SFF_MASK: u32` Mask valid frame bits
`pub const EFF_MASK: u32` Mask extended frame bits
`pub const ERR_MASK: u32` Mask error frame bits
`pub const ERR_MASK_ALL: u32` Report all errors flag
`pub const ERR_MASK_NONE: u32` Report no errors flag

## Opening and Binding to a CAN Socket
[Example C SocketCAN Code]https://www.beyondlogic.org/example-c-socketcan-code/
[Beej's Guide to Network Programming]https://beej.us/guide/bgnet

### Open Socket

Opening a CAN socket in C
```
int s;

if ((s = socket(PF_CAN, SOCK_RAW, CAN_RAW)) < 0) {
    perror("Socket");
    return 1;
}
```

Translated to Rust:
```
let fd: i32;
unsafe {
    fd = libc::socket(libc::PF_CAN, libc::SOCK_RAW, CAN_RAW);
}

if fd == -1 { error }
```

### Retrieve the interface index (struct CanAddress)

AF_CAN is the `Address Family`, which is a const. _if_index is the Interface Index, which can be retrieved from a ioctl call. We need to send over a ifreq struct to retrieve this data.

*I suspect we can also get this from netlink.*
The auther is calling `nix::net::if_::if_nametoindex;` to perform the psudocode listed below.

```
[RUST]
let ifr = libc::ifstructs::ifreq {
    ifr_name libc::c_short,
    ifr_ifru: libc::ifr_ifru,
}

let r = unsafe { libc::ioctl(fd, SIOCGIFINDEX, &ifr) };
```

the ifr_name is the interface name, like "vcan0" or "can1"

### Bind to Socket
This struct uses repr(C), so it's passing the FFI boundary.
```
[RUST]
struct CanAddr {
    _af_can: libc::c_short,
    _if_index: libc::c_int,
    rx_id: u32,
    tx_id: u32,
}

let addr = CanAddr {
    _af_can: AF_CAN,
    _if_index: ifr.ifr_ifru.ifr_ifindex,
    rx_id: ???
    tx_id: ???
}

let r = unsafe { libc::bind(fd, addr.as_ptr(), sizeof(addr) }

if r < 0 {
  return some binding error
}
```

### Socket Time Stamp
SIOCGSTAMPNS: Check last socket timestamp (ns)
`let r = libc::ioctl(self.fd, SIOCGSTAMPNS, ts.as_mut_ptr())`

## Methods on CanSocket

### Set nonblocking
Non-blocking is set using file status flags.
`libc::fcntl(fd, cmd)`

the pattern is to retrieve the current flags, flip bits, and set the modified flags.
```
oldfl = unsafe { libc::fcntl(self.fd, libc::F_GETFL) };
newfl = oldfl | libc::O_NONBLOCK;
let r = unsafe { libc::fcntl(self.fd, libc::F_SETFL) };
```

### Set Timeouts
both of these are heavily dependent on the utils mod. it seems like utils should be part of the lib?
set_read_timeout  
set_write_timeout  
I think I would move the set_socket_option into the lib.rs, and leave these conversions in the utils.

Why can't I implement a timeval as Duration and Duration as timeval? This is the orphan rule?


## Read CAN Frame
Why read without a timestamp?

## Write CAN Frame
Write is split into to pieces, a function that can fail, and a function that will block until it is successful. This seems like the wrong place for this utility, I'd like to move Should Retry to some sort of message manager module.
