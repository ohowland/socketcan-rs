
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