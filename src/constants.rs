// Protocol of the PF_CAN Family: Standard?
pub const CAN_RAW: libc::c_int = 1;

// Protool of the PF_CAN Family: Broadcast Manager
//const CAN_BCM: libc::c_int = 2;

pub const SOL_CAN_BASE: libc::c_int = 100;
pub const SOL_CAN_RAW: libc::c_int = SOL_CAN_BASE + CAN_RAW;
pub const CAN_RAW_FILTER: libc::c_int = 1;
pub const CAN_RAW_ERR_FILTER: libc::c_int = 2;
pub const CAN_RAW_LOOPBACK: libc::c_int = 3;
pub const CAN_RAW_RECV_OWN_MSGS: libc::c_int = 4;
pub const CAN_RAW_JOIN_FILTERS: libc::c_int = 6;
// const CAN_RAW_FD_FRAMES: c_int = 5;

// get timestamp from ioctl in a struct timespec (ns accuracy)
//pub const SIOCGSTAMPNS: libc::c_int = 0x8907;
pub const SIOCGSTAMP: libc::c_int = 0x8906;

/// Special address description flags for the CAN_ID
///
/// EFF/SFF is set in the MSB
pub const EFF_FLAG: u32 = 0x80000000;
/// remote transmission request
pub const RTR_FLAG: u32 = 0x40000000;
/// error message frame
pub const ERR_FLAG: u32 = 0x20000000;

/// valid bits in CAN ID for frame formats
/// standard frame format (SFF)
pub const SFF_MASK: u32 = 0x000007ff;
/// extended frame format (EFF)
pub const EFF_MASK: u32 = 0x1fffffff;
/// omit EFF, RTR, ERR flags
pub const ERR_MASK: u32 = 0x1fffffff;

// an error mask that will cause SocketCAN to report all errors
// pub const ERR_MASK_ALL: u32 = ERR_MASK;

// an error mask that will cause SocketCAN to silently drop all errors
// pub const ERR_MASK_NONE: u32 = 0;
