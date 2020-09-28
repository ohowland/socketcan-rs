use std::fmt;
use errors::{ConstructionError, CanError, CanErrorDecodingFailure};
use constants::*;

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
