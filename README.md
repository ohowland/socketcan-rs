# SocketCAN

[SocketCAN Man Page]https://www.kernel.org/doc/Documentation/networking/can.txt

## Netlink
[Netlink Wikipedia]https://en.wikipedia.org/wiki/Netlink

My understanding of netlink is that it is used to configure communication between the userspace and the kernelspace. When using commands like `ip link set up can0`, I believe we are invoking netlink.

If we want to bring up the can0 interface, we'll need to interface with netlink. This is not a priority for us, in the past we've configured the machine to enable these interfaces at startup using systemd.  

- [ ] pull in information from gateway on what exactly we do to setup the can interface.

## Sockets
[Beej's Guide to Network Programming]https://beej.us/guide/bgnet

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

