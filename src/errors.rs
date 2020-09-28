use frame::CanFrame;

// information from https://raw.githubusercontent.com/torvalds/linux/master/
//                  /include/uapi/linux/can/error.h

use std::convert::TryFrom;
use std::{error, fmt};

#[derive(Debug)]
/// Errors opening socket
pub enum CanSocketOpenError {
    /// Device could not be found
    LookupError(nix::Error),

    /// System error while trying to look up device name
    IOError(std::io::Error),
}

impl fmt::Display for CanSocketOpenError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CanSocketOpenError::LookupError(ref e) => write!(f, "CAN Device not found: {}", e),
            CanSocketOpenError::IOError(ref e) => write!(f, "IO: {}", e),
        }
    }
}

impl error::Error for CanSocketOpenError {}

impl From<nix::Error> for CanSocketOpenError {
    fn from(e: nix::Error) -> CanSocketOpenError {
        CanSocketOpenError::LookupError(e)
    }
}

impl From<std::io::Error> for CanSocketOpenError {
    fn from(e: std::io::Error) -> CanSocketOpenError {
        CanSocketOpenError::IOError(e)
    }
}

#[derive(Debug, Copy, Clone)]
/// Error that occurs when creating CAN packets
pub enum ConstructionError {
    /// CAN ID was outside the range of valid IDs
    IDTooLarge,
    /// More than 8 Bytes of payload data were passed in
    TooMuchData,
}

impl fmt::Display for ConstructionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ConstructionError::IDTooLarge => write!(f, "CAN ID too large"),
            ConstructionError::TooMuchData => {
                write!(f, "Payload is larger than CAN maximum of 8 bytes")
            }
        }
    }
}

impl error::Error for ConstructionError {
    fn description(&self) -> &str {
        match *self {
            ConstructionError::IDTooLarge => "can id too large",
            ConstructionError::TooMuchData => "too much data",
        }
    }
}



#[inline]
/// Helper function to retrieve a specific byte of frame data or returning an
/// `Err(..)` otherwise.
fn get_data(frame: &CanFrame, idx: u8) -> Result<u8, CanErrorDecodingFailure> {
    Ok(*(frame.data()
        .get(idx as usize)
        .ok_or_else(|| CanErrorDecodingFailure::NotEnoughData(idx)))?)
}


/// Error decoding a CanError from a CanFrame.
#[derive(Copy, Clone, Debug)]
pub enum CanErrorDecodingFailure {
    /// The supplied CanFrame did not have the error bit set.
    NotAnError,

    /// The error type is not known and cannot be decoded.
    UnknownErrorType(u32),

    /// The error type indicated a need for additional information as `data`,
    /// but the `data` field was not long enough.
    NotEnoughData(u8),

    /// The error type `ControllerProblem` was indicated and additional
    /// information found, but not recognized.
    InvalidControllerProblem,

    /// The type of the ProtocolViolation was not valid
    InvalidViolationType,

    /// A location was specified for a ProtocolViolation, but the location
    /// was not valid.
    InvalidLocation,

    /// The supplied transciever error was invalid.
    InvalidTransceiverError,
}
impl fmt::Display for CanErrorDecodingFailure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            CanErrorDecodingFailure::NotAnError => "CAN frame is not an error",
            CanErrorDecodingFailure::UnknownErrorType(_) => "unknown error type",
            CanErrorDecodingFailure::NotEnoughData(_) => "not enough data",
            CanErrorDecodingFailure::InvalidControllerProblem => "not a valid controller problem",
            CanErrorDecodingFailure::InvalidViolationType => "not a valid violation type",
            CanErrorDecodingFailure::InvalidLocation => "not a valid location",
            CanErrorDecodingFailure::InvalidTransceiverError => "not a valid transceiver error",
        })
    }
}
impl error::Error for CanErrorDecodingFailure {}

#[derive(Copy, Clone, Debug)]
pub enum CanError {
    /// TX timeout (by netdevice driver)
    TransmitTimeout,

    /// Arbitration was lost. Contains the number after which arbitration was
    /// lost or 0 if unspecified
    LostArbitration(u8),

    /// Controller problem, see `ControllerProblem`
    ControllerProblem(ControllerProblem),

    /// Protocol violation at the specified `Location`. See `ProtocolViolation`
    /// for details.
    ProtocolViolation {
        vtype: ViolationType,
        location: Location,
    },

    /// Transceiver Error.
    TransceiverError,

    /// No ACK received for current CAN frame.
    NoAck,

    /// Bus off (due to too many detected errors)
    BusOff,

    /// Bus error (due to too many detected errors)
    BusError,

    /// The bus has been restarted
    Restarted,

    /// Unknown, possibly invalid, error
    Unknown(u32),
}

impl error::Error for CanError {}

impl fmt::Display for CanError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CanError::TransmitTimeout => write!(f, "transmission timeout"),
            CanError::LostArbitration(n) => write!(f, "arbitration lost after {} bits", n),
            CanError::ControllerProblem(e) => write!(f, "controller problem: {}", e),
            CanError::ProtocolViolation { vtype, location } => write!(f, "protocol violation at {}: {}", location, vtype),
            CanError::TransceiverError => write!(f, "transceiver error"),
            CanError::NoAck => write!(f, "no ack"),
            CanError::BusOff => write!(f, "bus off"),
            CanError::BusError => write!(f, "bus error"),
            CanError::Restarted => write!(f, "restarted"),
            CanError::Unknown(errno) => write!(f, "unknown error ({})", errno),
        }
    }
}


#[derive(Copy, Clone, Debug)]
pub enum ControllerProblem {
    // unspecified
    Unspecified,

    // RX buffer overflow
    ReceiveBufferOverflow,

    // TX buffer overflow
    TransmitBufferOverflow,

    // reached warning level for RX errors
    ReceiveErrorWarning,

    // reached warning level for TX errors
    TransmitErrorWarning,

    // reached error passive status RX
    ReceiveErrorPassive,

    // reached error passive status TX
    TransmitErrorPassive,

    // recovered to error active state
    Active,
}

impl error::Error for ControllerProblem {}

impl fmt::Display for ControllerProblem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            ControllerProblem::Unspecified => "unspecified controller problem",
            ControllerProblem::ReceiveBufferOverflow => "receive buffer overflow",
            ControllerProblem::TransmitBufferOverflow => "transmit buffer overflow",
            ControllerProblem::ReceiveErrorWarning => "ERROR WARNING (receive)",
            ControllerProblem::TransmitErrorWarning => "ERROR WARNING (transmit)",
            ControllerProblem::ReceiveErrorPassive => "ERROR PASSIVE (receive)",
            ControllerProblem::TransmitErrorPassive => "ERROR PASSIVE (transmit)",
            ControllerProblem::Active => "ERROR ACTIVE",
        })
    }
}

impl TryFrom<u8> for ControllerProblem {
    type Error = CanErrorDecodingFailure;

    fn try_from(val: u8) -> Result<Self, Self::Error> {
        Ok(match val {
            0x00 => ControllerProblem::Unspecified,
            0x01 => ControllerProblem::ReceiveBufferOverflow,
            0x02 => ControllerProblem::TransmitBufferOverflow,
            0x04 => ControllerProblem::ReceiveErrorWarning,
            0x08 => ControllerProblem::TransmitErrorWarning,
            0x10 => ControllerProblem::ReceiveErrorPassive,
            0x20 => ControllerProblem::TransmitErrorPassive,
            0x40 => ControllerProblem::Active,
            _ => return Err(CanErrorDecodingFailure::InvalidControllerProblem),
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ViolationType {
    /// Unspecified Violation
    Unspecified,

    /// Single Bit Error
    SingleBitError,

    /// Frame formatting error
    FrameFormatError,

    /// Bit stuffing error
    BitStuffingError,

    /// A dominant bit was sent, but not received
    UnableToSendDominantBit,

    /// A recessive bit was sent, but not received
    UnableToSendRecessiveBit,

    /// Bus overloaded
    BusOverload,

    /// Bus is active (again)
    Active,

    /// Transmission Error
    TransmissionError,
}

impl error::Error for ViolationType {}

impl fmt::Display for ViolationType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            ViolationType::Unspecified => "unspecified",
            ViolationType::SingleBitError => "single bit error",
            ViolationType::FrameFormatError => "frame format error",
            ViolationType::BitStuffingError => "bit stuffing error",
            ViolationType::UnableToSendDominantBit => "unable to send dominant bit",
            ViolationType::UnableToSendRecessiveBit => "unable to send recessive bit",
            ViolationType::BusOverload => "bus overload",
            ViolationType::Active => "active",
            ViolationType::TransmissionError => "transmission error",
        })
    }
}

impl TryFrom<u8> for ViolationType {
    type Error = CanErrorDecodingFailure;

    fn try_from(val: u8) -> Result<Self, Self::Error> {
        Ok(
            match val {
                0x00 => ViolationType::Unspecified,
                0x01 => ViolationType::SingleBitError,
                0x02 => ViolationType::FrameFormatError,
                0x04 => ViolationType::BitStuffingError,
                0x08 => ViolationType::UnableToSendDominantBit,
                0x10 => ViolationType::UnableToSendRecessiveBit,
                0x20 => ViolationType::BusOverload,
                0x40 => ViolationType::Active,
                0x80 => ViolationType::TransmissionError,
                _ => return Err(CanErrorDecodingFailure::InvalidViolationType),
        })
    }
}

/// Location
///
/// Describes where inside a received frame an error occured.
#[derive(Copy, Clone, Debug)]
pub enum Location {
    /// Unspecified
    Unspecified,

    /// Start of frame.
    StartOfFrame,

    /// ID bits 28-21 (SFF: 10-3)
    Id2821,

    /// ID bits 20-18 (SFF: 2-0)
    Id2018,

    /// substitute RTR (SFF: RTR)
    SubstituteRtr,

    /// extension of identifier
    IdentifierExtension,

    /// ID bits 17-13
    Id1713,

    /// ID bits 12-5
    Id1205,

    /// ID bits 4-0
    Id0400,

    /// RTR bit
    Rtr,

    /// Reserved bit 1
    Reserved1,

    /// Reserved bit 0
    Reserved0,

    /// Data length
    DataLengthCode,

    /// Data section
    DataSection,

    /// CRC sequence
    CrcSequence,

    /// CRC delimiter
    CrcDelimiter,

    /// ACK slot
    AckSlot,

    /// ACK delimiter
    AckDelimiter,

    /// End-of-frame
    EndOfFrame,

    /// Intermission (between frames)
    Intermission,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            Location::Unspecified => "unspecified location",
            Location::StartOfFrame => "start of frame",
            Location::Id2821 => "ID, bits 28-21",
            Location::Id2018 => "ID, bits 20-18",
            Location::SubstituteRtr => "substitute RTR bit",
            Location::IdentifierExtension => "ID, extension",
            Location::Id1713 => "ID, bits 17-13",
            Location::Id1205 => "ID, bits 12-05",
            Location::Id0400 => "ID, bits 04-00",
            Location::Rtr => "RTR bit",
            Location::Reserved1 => "reserved bit 1",
            Location::Reserved0 => "reserved bit 0",
            Location::DataLengthCode => "data length code",
            Location::DataSection => "data section",
            Location::CrcSequence => "CRC sequence",
            Location::CrcDelimiter => "CRC delimiter",
            Location::AckSlot => "ACK slot",
            Location::AckDelimiter => "ACK delimiter",
            Location::EndOfFrame => "end of frame",
            Location::Intermission => "intermission",
        })
    }
}

impl TryFrom<u8> for Location {
    type Error = CanErrorDecodingFailure;

    fn try_from(val: u8) -> Result<Self, Self::Error> {
        Ok(match val {
            0x00 => Location::Unspecified,
            0x03 => Location::StartOfFrame,
            0x02 => Location::Id2821,
            0x06 => Location::Id2018,
            0x04 => Location::SubstituteRtr,
            0x05 => Location::IdentifierExtension,
            0x07 => Location::Id1713,
            0x0F => Location::Id1205,
            0x0E => Location::Id0400,
            0x0C => Location::Rtr,
            0x0D => Location::Reserved1,
            0x09 => Location::Reserved0,
            0x0B => Location::DataLengthCode,
            0x0A => Location::DataSection,
            0x08 => Location::CrcSequence,
            0x18 => Location::CrcDelimiter,
            0x19 => Location::AckSlot,
            0x1B => Location::AckDelimiter,
            0x1A => Location::EndOfFrame,
            0x12 => Location::Intermission,
            _ => return Err(CanErrorDecodingFailure::InvalidLocation),
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub enum TransceiverError {
    Unspecified,
    CanHighNoWire,
    CanHighShortToBat,
    CanHighShortToVcc,
    CanHighShortToGnd,
    CanLowNoWire,
    CanLowShortToBat,
    CanLowShortToVcc,
    CanLowShortToGnd,
    CanLowShortToCanHigh,
}

impl fmt::Display for TransceiverError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            TransceiverError::Unspecified => "Unspecified",
            TransceiverError::CanHighNoWire => "CANbus High Wire Open",
            TransceiverError::CanHighShortToBat => "CANbus High Short to Battery",
            TransceiverError::CanHighShortToVcc => "CANbus High Short to VCC",
            TransceiverError::CanHighShortToGnd => "CANbus High Short to Ground",
            TransceiverError::CanLowNoWire => "CANbus Low Wire Open",
            TransceiverError::CanLowShortToBat => "CANbus Low Short to Battery",
            TransceiverError::CanLowShortToVcc => "CANbus Low Short to VCC",
            TransceiverError::CanLowShortToGnd => "CANbus Low Short to Ground",
            TransceiverError::CanLowShortToCanHigh => "CANbus Low and High Shorted"
        })
    }
}

impl error::Error for TransceiverError {}

impl TryFrom<u8> for TransceiverError {
    type Error = CanErrorDecodingFailure;

    fn try_from(val: u8) -> Result<Self, Self::Error> {
        match val {
            0x00 => Ok(TransceiverError::Unspecified),
            0x04 => Ok(TransceiverError::CanHighNoWire),
            0x05 => Ok(TransceiverError::CanHighShortToBat),
            0x06 => Ok(TransceiverError::CanHighShortToVcc),
            0x07 => Ok(TransceiverError::CanHighShortToGnd),
            0x40 => Ok(TransceiverError::CanLowNoWire),
            0x50 => Ok(TransceiverError::CanLowShortToBat),
            0x60 => Ok(TransceiverError::CanLowShortToVcc),
            0x70 => Ok(TransceiverError::CanLowShortToGnd),
            0x80 => Ok(TransceiverError::CanLowShortToCanHigh),
            _ => Err(CanErrorDecodingFailure::InvalidTransceiverError),
        }
    }
}

//TODO: can we convert this to the try_from pattern?
impl CanError {
    pub fn from_frame(frame: &CanFrame) -> Result<CanError, CanErrorDecodingFailure> {
        if !frame.is_error() {
            return Err(CanErrorDecodingFailure::NotAnError);
        }

        match frame.err() {
            0x00000001 => Ok(CanError::TransmitTimeout),
            0x00000002 => Ok(CanError::LostArbitration(get_data(frame, 0)?)),
            0x00000004 => {
                Ok(CanError::ControllerProblem(ControllerProblem::try_from(get_data(frame, 1)?)?))
            }

            0x00000008 => {
                Ok(CanError::ProtocolViolation {
                    vtype: ViolationType::try_from(get_data(frame, 2)?)?,
                    location: Location::try_from(get_data(frame, 3)?)?,
                })
            }

            0x00000010 => Ok(CanError::TransceiverError),
            0x00000020 => Ok(CanError::NoAck),
            0x00000040 => Ok(CanError::BusOff),
            0x00000080 => Ok(CanError::BusError),
            0x00000100 => Ok(CanError::Restarted),
            e => Err(CanErrorDecodingFailure::UnknownErrorType(e)),
        }
    }
}

pub trait ControllerSpecificErrorInformation {
    fn get_ctrl_err(&self) -> Option<&[u8]>;
}

impl ControllerSpecificErrorInformation for CanFrame {
    #[inline]
    fn get_ctrl_err(&self) -> Option<&[u8]> {
        let data = self.data();

        if data.len() != 8 {
            None
        } else {
            Some(&data[5..])
        }
    }
}
