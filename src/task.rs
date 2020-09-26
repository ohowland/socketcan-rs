
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

impl ShouldRetry<T> for std::io::Result<T> {
    fn should_retry(&self) -> bool {
        match *self {
            Ok(_) => false,
            Err(e) => match e.kind() {
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


/// Blocking write a single can frame, retrying until it gets sent
/// successfully.
pub fn write_retry(&self, frame: &CanFrame) -> io::Result<()> {
    loop {
        match self.write_retry(frame) {
            Ok(v) => return Ok(v),
            Err(e) => {
                if !e.should_retry() {
                    return Err(e);
                }
            }
        }
    }
}