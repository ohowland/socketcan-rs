//! SocketCAN support.
//!
//! The Linux kernel supports using CAN-devices through a network-like API
//! (see https://www.kernel.org/doc/Documentation/networking/can.txt). This
//! crate allows easy access to this functionality without having to wrestle
//! libc calls.
//!
//! # An introduction to CAN
//!
//! The CAN bus was originally designed to allow microcontrollers inside a
//! vehicle to communicate over a single shared bus. Messages called
//! *frames* are multicast to all devices on the bus.
//!
//! Every frame consists of an ID and a payload of up to 8 bytes. If two
//! devices attempt to send a frame at the same time, the device with the
//! higher ID will notice the conflict, stop sending and reattempt to sent its
//! frame in the next time slot. This means that the lower the ID, the higher
//! the priority. Since most devices have a limited buffer for outgoing frames,
//! a single device with a high priority (== low ID) can block communication
//! on that bus by sending messages too fast.
//!
//! The Linux socketcan subsystem makes the CAN bus available as a regular
//! networking device. Opening an network interface allows receiving all CAN
//! messages received on it. A device CAN be opened multiple times, every
//! client will receive all CAN frames simultaneously.
//!
//! Similarly, CAN frames can be sent to the bus by multiple client
//! simultaneously as well.
//!
//! # Hardware and more information
//!
//! More information on CAN [can be found on Wikipedia](). When not running on
//! an embedded platform with already integrated CAN components,
//! [Thomas Fischl's USBtin](http://www.fischl.de/usbtin/) (see
//! [section 2.4](http://www.fischl.de/usbtin/#socketcan)) is one of many ways
//! to get started.
//!
//! # RawFd
//!
//! Raw access to the underlying file descriptor and construction through
//! is available through the `AsRawFd`, `IntoRawFd` and `FromRawFd`
//! implementations.

extern crate libc;
extern crate nix;
extern crate itertools;

mod errors;
mod util;

use std::{fmt, mem, io, time};
use itertools::Itertools;

use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
pub use errors::{
    CanError, 
    CanErrorDecodingFailure, 
    CanSocketOpenError, 
    ConstructionError
};

#[cfg(test)]
mod tests;



/// Check an error return value for timeouts.
///
/// Due to the fact that timeouts are reported as errors, calling `read_frame`
/// on a socket with a timeout that does not receive a frame in time will
/// result in an error being returned. This trait adds a `should_retry` method
/// to `Error` and `Result` to check for this condition.
pub trait ShouldRetry {
    /// Check for timeout
    ///
    /// If `true`, the error is probably due to a timeout.
    fn should_retry(&self) -> bool;
}

impl ShouldRetry for std::io::Error {
    fn should_retry(&self) -> bool {
        match self.kind() {
            // EAGAIN, EINPROGRESS and EWOULDBLOCK are the three possible codes
            // returned when a timeout occurs. the stdlib already maps EAGAIN
            // and EWOULDBLOCK os WouldBlock
            std::io::ErrorKind::WouldBlock => true,
            // however, EINPROGRESS is also valid
            std::io::ErrorKind::Other => {
                if let Some(i) = self.raw_os_error() {
                    i == libc::EINPROGRESS
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

impl<E: std::fmt::Debug> ShouldRetry for std::io::Result<E> {
    fn should_retry(&self) -> bool {
        if let Err(ref e) = *self {
            e.should_retry()
        } else {
            false
        }
    }
}

// Protocol of the PF_CAN Family: Standard?
const CAN_RAW: libc::c_int = 1;

// Protool of the PF_CAN Family: Broadcast Manager
const CAN_BCM: libc::c_int = 2;

const SOL_CAN_BASE: libc::c_int = 100;
const SOL_CAN_RAW: libc::c_int = SOL_CAN_BASE + CAN_RAW;
const CAN_RAW_FILTER: libc::c_int = 1;
const CAN_RAW_ERR_FILTER: libc::c_int = 2;
const CAN_RAW_LOOPBACK: libc::c_int = 3;
const CAN_RAW_RECV_OWN_MSGS: libc::c_int = 4;
const CAN_RAW_JOIN_FILTERS: libc::c_int = 6;
// const CAN_RAW_FD_FRAMES: c_int = 5;

// get timestamp from ioctl in a struct timespec (ns accuracy)
const SIOCGSTAMPNS: libc::c_int = 0x8907;

/// Indicate 29 bit extended format
pub const EFF_FLAG: u32 = 0x80000000;

/// remote transmission request flag
pub const RTR_FLAG: u32 = 0x40000000;

/// error flag
pub const ERR_FLAG: u32 = 0x20000000;

/// valid bits in standard frame id
pub const SFF_MASK: u32 = 0x000007ff;

/// valid bits in extended frame id
pub const EFF_MASK: u32 = 0x1fffffff;

/// valid bits in error frame
pub const ERR_MASK: u32 = 0x1fffffff;

/// an error mask that will cause SocketCAN to report all errors
pub const ERR_MASK_ALL: u32 = ERR_MASK;

/// an error mask that will cause SocketCAN to silently drop all errors
pub const ERR_MASK_NONE: u32 = 0;

/// A socket for a CAN device.
///
/// Will be closed upon deallocation. To close manually, use std::drop::Drop.
/// Internally this is just a wrapped file-descriptor.
#[derive(Debug)]
pub struct CanSocket {
    fd: libc::c_int,
}

/// A CAN address struct for binding a socket
#[derive(Debug)]
#[repr(C)]
struct CanAddr {
    af_can: libc::c_short,
    if_index: libc::c_int,
    rx_id: libc::c_uint, // transport protocol class address information (e.g. ISOTP)
    tx_id: libc::c_uint,
}

impl CanSocket {
    /// Open a named CAN device.
    ///
    /// Usually the more common case, opens a socket can device by name, such
    /// as "vcan0" or "socan0".
    pub fn open(ifname: &str) -> Result<CanSocket, CanSocketOpenError> {
        let if_index = nix::net::if_::if_nametoindex(ifname)?;
        CanSocket::open_interface(if_index)
    }

    /// Open CAN device by interface number.
    ///
    /// Opens a CAN device by kernel interface number.
    pub fn open_interface(if_index: libc::c_uint) -> Result<CanSocket, CanSocketOpenError> {
        let fd = CanSocket::open_socket()?;
        let fd = CanSocket::bind_socket(if_index, fd)?;
        Ok(CanSocket { fd: fd })
    }

    fn open_socket() -> Result<i32, CanSocketOpenError> {
        let fd: i32;
        unsafe {
            fd = libc::socket(libc::PF_CAN, libc::SOCK_RAW, CAN_RAW);
        }

        if fd == -1 {
            return Err(CanSocketOpenError::from(io::Error::last_os_error()));
        }

        Ok(fd)
    }

    fn bind_socket(if_index: libc::c_uint, fd: i32) -> Result<i32, CanSocketOpenError> { 
        let socketaddr = CanAddr {
            af_can: libc::AF_CAN as libc::c_short,
            if_index: if_index as libc::c_int,
            rx_id: 0,
            tx_id: 0,
        };

        let r: i32;
        unsafe {
            let socketaddr_ptr = &socketaddr as *const CanAddr;
            r = libc::bind(
                fd,
                socketaddr_ptr as *const libc::sockaddr,
                mem::size_of::<CanAddr>() as u32
            );
        }

        if r == -1 {
            let e = io::Error::last_os_error();
            // clean up resource if failure to open
            unsafe { libc::close(fd); }
            return Err(CanSocketOpenError::from(e));
        }

        Ok(fd)
    }

    pub fn close(&mut self) -> io::Result<()> {
        unsafe {
            let r = libc::close(self.fd);
            if r != -1 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }

    /// Change socket to non-blocking mode
    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        // retrieve current file status flags
        let old_flags = unsafe { libc::fcntl(self.fd, libc::F_GETFL) };

        if old_flags == -1 {
            return Err(io::Error::last_os_error());
        }

        let new_flags = if nonblocking {
            old_flags | libc::O_NONBLOCK
        } else {
            old_flags & !libc::O_NONBLOCK
        };

        let r = unsafe { libc::fcntl(self.fd, libc::F_SETFL, new_flags) };

        if r != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// Set the read timeout on the socket
    ///
    /// For convenience, the result value can be checked using
    /// `ShouldRetry::should_retry` when a timeout is set.
    pub fn set_read_timeout(&self, duration: time::Duration) -> io::Result<()> {
        util::set_socket_option(
            self.fd,
            libc::SOL_SOCKET,
            libc::SO_RCVTIMEO,
            &util::timeval_from_duration(duration)
        )
    }

    /// Set the write timeout on the socket
    pub fn set_write_timeout(&self, duration: time::Duration) -> io::Result<()> {
        util::set_socket_option(
            self.fd,
            libc::SOL_SOCKET,
            libc::SO_SNDTIMEO,
            &util::timeval_from_duration(duration)
        )
    }


    /// Blocking read a single can frame with timestamp
    ///
    /// Note that reading a frame and retrieving the timestamp requires two
    /// consecutive syscalls.
    pub fn read(&self) -> io::Result<(CanFrame, time::SystemTime)> {
        let frame = self.read_socket()?;
        let ts = self.socket_timestamp()?;

        Ok((frame, ts))
    }

    fn socket_timestamp(&self) -> io::Result<time::SystemTime> {
        let mut ts = mem::MaybeUninit::<libc::timespec>::uninit();
        let r = unsafe { 
            libc::ioctl(self.fd,
                        SIOCGSTAMPNS as libc::c_ulong,
                        ts.as_mut_ptr())
        };

        if r == -1 {
            return Err(io::Error::last_os_error());
        }

        let ts = unsafe { ts.assume_init() };
        
        Ok(util::system_time_from_timespec(ts))
    }
    
    /// Blocking read a single can frame.
    fn read_socket(&self) -> io::Result<CanFrame> {
        let mut frame = CanFrame {
            _id: 0,
            _data_len: 0,
            _pad: 0,
            _res0: 0,
            _res1: 0,
            _data: [0; 8],
        };

        let r = unsafe {
            let frame_ptr = &mut frame as *mut CanFrame;
            libc::read(self.fd, frame_ptr as *mut libc::c_void, mem::size_of::<CanFrame>())
        };

        if r as usize != mem::size_of::<CanFrame>() {
            return Err(io::Error::last_os_error());
        }

        Ok(frame)
    }

    /// Write a single can frame.
    ///
    /// Note that this function can fail with an `EAGAIN` error or similar.
    /// Use `write_frame_insist` if you need to be sure that the message got
    /// sent or failed.
    pub fn write(&self, frame: &CanFrame) -> io::Result<()> {
        let r = unsafe {
            let frame_ptr = frame as *const CanFrame;
            libc::write(self.fd, frame_ptr as *const libc::c_void, mem::size_of::<CanFrame>())
        };

        if r as usize != mem::size_of::<CanFrame>() {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    /// Sets filters on the socket.
    ///
    /// CAN packages received by SocketCAN are matched against these filters,
    /// only matching packets are returned by the interface.
    ///
    /// See `CanFilter` for details on how filtering works. By default, all
    /// single filter matching all incoming frames is installed.
    pub fn set_filters(&self, filters: &[CanFilter]) -> io::Result<()> {
        set_socket_option_mult(self.fd, SOL_CAN_RAW, CAN_RAW_FILTER, filters)
    }

    /// Sets the error mask on the socket.
    ///
    /// By default (`ERR_MASK_NONE`) no error conditions are reported as
    /// special error frames by the socket. Enabling error conditions by
    /// setting `ERR_MASK_ALL` or another non-empty error mask causes the
    /// socket to receive notification about the specified conditions.
    #[inline]
    pub fn set_error_mask(&self, mask: u32) -> io::Result<()> {
        set_socket_option(self.fd, SOL_CAN_RAW, CAN_RAW_ERR_FILTER, &mask)
    }

    /// Enable or disable loopback.
    ///
    /// By default, loopback is enabled, causing other applications that open
    /// the same CAN bus to see frames emitted by different applications on
    /// the same system.
    #[inline]
    pub fn set_loopback(&self, enabled: bool) -> io::Result<()> {
        let loopback: c_int = if enabled { 1 } else { 0 };
        set_socket_option(self.fd, SOL_CAN_RAW, CAN_RAW_LOOPBACK, &loopback)
    }

    /// Enable or disable receiving of own frames.
    ///
    /// When loopback is enabled, this settings controls if CAN frames sent
    /// are received back immediately by sender. Default is off.
    pub fn set_recv_own_msgs(&self, enabled: bool) -> io::Result<()> {
        let recv_own_msgs: c_int = if enabled { 1 } else { 0 };
        set_socket_option(self.fd, SOL_CAN_RAW, CAN_RAW_RECV_OWN_MSGS, &recv_own_msgs)
    }

    /// Enable or disable join filters.
    ///
    /// By default a frame is accepted if it matches any of the filters set
    /// with `set_filters`. If join filters is enabled, a frame has to match
    /// _all_ filters to be accepted.
    pub fn set_join_filters(&self, enabled: bool) -> io::Result<()> {
        let join_filters: c_int = if enabled { 1 } else { 0 };
        set_socket_option(self.fd, SOL_CAN_RAW, CAN_RAW_JOIN_FILTERS, &join_filters)
    }
}


impl AsRawFd for CanSocket {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl FromRawFd for CanSocket {
    unsafe fn from_raw_fd(fd: RawFd) -> CanSocket {
        CanSocket { fd: fd }
    }
}

impl IntoRawFd for CanSocket {
    fn into_raw_fd(self) -> RawFd {
        self.fd
    }
}

impl Drop for CanSocket {
    fn drop(&mut self) {
        self.close().ok(); // ignore result
    }
}

/// CanFrame
///
/// Uses the same memory layout as the underlying kernel struct for performance
/// reasons.
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct CanFrame {
    /// 32 bit CAN_ID + EFF/RTR/ERR flags
    _id: u32,

    /// data length. Bytes beyond are not valid
    _data_len: u8,

    /// padding
    _pad: u8,

    /// reserved
    _res0: u8,

    /// reserved
    _res1: u8,

    /// buffer for data
    _data: [u8; 8],
}

impl CanFrame {
    pub fn new(id: u32, data: &[u8], rtr: bool, err: bool) -> Result<CanFrame, ConstructionError> {
        let mut _id = id;

        if data.len() > 8 {
            return Err(ConstructionError::TooMuchData);
        }

        if id > EFF_MASK {
            return Err(ConstructionError::IDTooLarge);
        }

        // set EFF_FLAG on large message
        if id > SFF_MASK {
            _id |= EFF_FLAG;
        }


        if rtr {
            _id |= RTR_FLAG;
        }

        if err {
            _id |= ERR_FLAG;
        }

        let mut full_data = [0; 8];

        // not cool =/
        for (n, c) in data.iter().enumerate() {
            full_data[n] = *c;
        }

        Ok(CanFrame {
               _id: _id,
               _data_len: data.len() as u8,
               _pad: 0,
               _res0: 0,
               _res1: 0,
               _data: full_data,
           })
    }

    /// Return the actual CAN ID (without EFF/RTR/ERR flags)
    #[inline]
    pub fn id(&self) -> u32 {
        if self.is_extended() {
            self._id & EFF_MASK
        } else {
            self._id & SFF_MASK
        }
    }

    /// Return the error message
    #[inline]
    pub fn err(&self) -> u32 {
        self._id & ERR_MASK
    }

    /// Check if frame uses 29 bit extended frame format
    #[inline]
    pub fn is_extended(&self) -> bool {
        self._id & EFF_FLAG != 0
    }

    /// Check if frame is an error message
    #[inline]
    pub fn is_error(&self) -> bool {
        self._id & ERR_FLAG != 0
    }

    /// Check if frame is a remote transmission request
    #[inline]
    pub fn is_rtr(&self) -> bool {
        self._id & RTR_FLAG != 0
    }

    /// A slice into the actual data. Slice will always be <= 8 bytes in length
    #[inline]
    pub fn data(&self) -> &[u8] {
        &self._data[..(self._data_len as usize)]
    }

    /// Read error from message and transform it into a `CanError`.
    ///
    /// SocketCAN errors are indicated using the error bit and coded inside
    /// id and data payload. Call `error()` converts these into usable
    /// `CanError` instances.
    ///
    /// If the frame is malformed, this may fail with a
    /// `CanErrorDecodingFailure`.
    #[inline]
    pub fn error(&self) -> Result<CanError, CanErrorDecodingFailure> {
        CanError::from_frame(self)
    }
}

impl fmt::UpperHex for CanFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:X}#", self.id())?;

        let mut parts = self.data().iter().map(|v| format!("{:02X}", v));

        let sep = if f.alternate() { " " } else { "" };
        write!(f, "{}", parts.join(sep))
    }
}

/// CanFilter
///
/// Contains an internal id and mask. Packets are considered to be matched by
/// a filter if `received_id & mask == filter_id & mask` holds true.
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct CanFilter {
    _id: u32,
    _mask: u32,
}

impl CanFilter {
    /// Construct a new CAN filter.
    pub fn new(id: u32, mask: u32) -> Result<CanFilter, ConstructionError> {
        Ok(CanFilter {
               _id: id,
               _mask: mask,
           })
    }
}
