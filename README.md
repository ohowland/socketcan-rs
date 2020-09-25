# SocketCAN

[SocketCAN Man Page]https://www.kernel.org/doc/Documentation/networking/can.txt

## Netlink
[Netlink Wikipedia]https://en.wikipedia.org/wiki/Netlink

## Sockets
[Beej's Guide to Network Programming]https://beej.us/guide/bgnet

### Migrate netlink-rs to netlink
The author of the original socketcan library expresses concern about the state of the netlink-rs library. He actually doesn't end up using it in the lib.rs (he copys a few lines, but leaves the library in the .toml, ostesibly to credit work).

It seems like github.com/little-dude is currently leading the netlink charge.
