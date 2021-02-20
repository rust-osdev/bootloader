#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct InterruptVectorTable {
	// Goes up to 255
    pub entries: [Entry; 32],
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Entry {
    pub segment: u32,
    pub offset: u32,
}