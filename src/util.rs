use std::{io, ptr, mem, time};

/// `setsockopt` wrapper
///
/// The libc `setsockopt` function is set to set various options on a socket.
/// `set_socket_option` offers a somewhat type-safe wrapper that does not
/// require messing around with `*const c_void`s.
///
/// A proper `std::io::Error` will be returned on failure.
///
/// Example use:
///
/// ```text
/// let fd = ...;  // some file descriptor, this will be stdout
/// set_socket_option(fd, SOL_TCP, TCP_NO_DELAY, 1 as c_int)
/// ```
///
/// Note that the `val` parameter must be specified correctly; if an option
/// expects an integer, it is advisable to pass in a `c_int`, not the default
/// of `i32`.
pub fn set_socket_option<T>(fd: libc::c_int, 
                            level: libc::c_int, 
                            name: libc::c_int, 
                            val: &T) -> io::Result<()> {
    let r = unsafe {
        let val_ptr: *const T = val as *const T;
        libc::setsockopt(fd,
                         level,
                         name,
                         val_ptr as *const libc::c_void,
                         mem::size_of::<T>() as libc::socklen_t)
    };

    if r != 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(())
}

pub fn set_socket_option_mult<T>(fd: libc::c_int,
                                 level: libc::c_int,
                                 name: libc::c_int,
                                 values: &[T])
                                 -> io::Result<()> {

    let r = if values.len() < 1 {
        // can't pass in a pointer to the first element if a 0-length slice,
        // pass a nullpointer instead
        unsafe { libc::setsockopt(fd, level, name, ptr::null(), 0) }
    } else {
        unsafe {
            let val_ptr = &values[0] as *const T;

            libc::setsockopt(
                fd, 
                level,
                name,
                val_ptr as *const libc::c_void,
                (mem::size_of::<T>() * values.len()) as libc::socklen_t)
        }
    };

    if r != 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(())
}

pub fn timeval_from_duration(t: std::time::Duration) -> libc::timeval {
    libc::timeval {
        tv_sec: t.as_secs() as libc::time_t,
        tv_usec: (t.subsec_micros()) as libc::suseconds_t,
    }
}

pub fn duration_from_timespec(ts: libc::timespec) -> time::Duration {
    time::Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32)
}

pub fn system_time_from_timespec(ts: libc::timespec) -> time::SystemTime {
    time::UNIX_EPOCH + duration_from_timespec(ts)
}
