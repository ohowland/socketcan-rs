use std::{mem, io, time};
use log::debug;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};

use frame::CanFrame;
use filter::CanFilter;
use util::{set_socket_option, set_socket_option_mult, system_time_from_timespec, timeval_from_duration};
use errors::CanSocketOpenError;
use constants::*;

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
        match nix::net::if_::if_nametoindex(ifname) {
            Ok(ifindex) => CanSocket::open_interface(ifindex),
            Err(e) => Err(CanSocketOpenError::from(e)),
        }
    }

    /// Open CAN device by interface number.
    ///
    /// Opens a CAN device by kernel interface number.
    fn open_interface(if_index: libc::c_uint) -> Result<CanSocket, CanSocketOpenError> {
        match CanSocket::open_socket() {
            Ok(fd) => CanSocket::bind_socket(if_index, fd), 
            Err(e) => Err(e),
        }
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

    fn bind_socket(if_index: libc::c_uint, fd: i32) -> Result<CanSocket, CanSocketOpenError> { 
        let socketaddr = CanAddr {
            af_can: libc::AF_CAN as libc::c_short,
            if_index: if_index as libc::c_int,
            rx_id: 0,
            tx_id: 0,
        };

        let r: i32;
        unsafe {
            let p = &socketaddr as *const CanAddr;
            r = libc::bind(fd,
                           p as *const libc::sockaddr,
                           mem::size_of::<CanAddr>() as u32
            );
        }

        if r == -1 {
            let e = io::Error::last_os_error();
            // clean up resource if failure to open
            unsafe { libc::close(fd); }
            return Err(CanSocketOpenError::from(e));
        }
        
        Ok(CanSocket { fd: fd })
    }

    pub fn close(&mut self) -> io::Result<()> {
        let r: i32;
        unsafe {
            r = libc::close(self.fd);
        }

        if r == -1 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
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
                        SIOCGSTAMP as libc::c_ulong,
                        ts.as_mut_ptr())
        };

        if r == -1 {
            return Err(io::Error::last_os_error());
        }

        let ts = unsafe { ts.assume_init() };
        
        Ok(system_time_from_timespec(ts))
    }
    
    /// Blocking read a single can frame.
    fn read_socket(&self) -> io::Result<CanFrame> {
        let mut frame = CanFrame::empty();

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
        set_socket_option(
            self.fd,
            libc::SOL_SOCKET,
            libc::SO_RCVTIMEO,
            &timeval_from_duration(duration)
        )
    }

    /// Set the write timeout on the socket
    pub fn set_write_timeout(&self, duration: time::Duration) -> io::Result<()> {
        set_socket_option(
            self.fd,
            libc::SOL_SOCKET,
            libc::SO_SNDTIMEO,
            &timeval_from_duration(duration)
        )
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
        let loopback: libc::c_int = match enabled {
            true => 1,
            false => 0,
        };
        set_socket_option(self.fd, SOL_CAN_RAW, CAN_RAW_LOOPBACK, &loopback)
    }

    /// Enable or disable receiving of own frames.
    ///
    /// When loopback is enabled, this settings controls if CAN frames sent
    /// are received back immediately by sender. Default is off.
    pub fn set_recv_own_msgs(&self, enabled: bool) -> io::Result<()> {
        let recv_own_msgs: libc::c_int = match enabled {
            true => 1,
            false => 0,
        };
        set_socket_option(self.fd, SOL_CAN_RAW, CAN_RAW_RECV_OWN_MSGS, &recv_own_msgs)
    }

    /// Enable or disable join filters.
    ///
    /// By default a frame is accepted if it matches any of the filters set
    /// with `set_filters`. If join filters is enabled, a frame has to match
    /// _all_ filters to be accepted.
    pub fn set_join_filters(&self, enabled: bool) -> io::Result<()> {
        let join_filters: libc::c_int = match enabled {
            true => 1,
            false => 0,
        };
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
        match self.close() {
            Ok(_) => debug!("Socket dropped (fd: {})", self.fd),
            Err(e) => debug!("Error dropping socket {}", e),
        };
    }
}