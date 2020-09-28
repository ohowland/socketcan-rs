use errors::ConstructionError;

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
