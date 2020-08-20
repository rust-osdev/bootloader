use core::mem;

const PAGE_SIZE: usize = 4096;

pub struct MemoryMapBuilder {
    memory_map: &'static mut MemoryMap,
    next_free: usize,
}

impl MemoryMapBuilder {
    pub fn new(memory_map: &'static mut MemoryMap) -> Self {
        assert_eq!(*memory_map, MemoryMap::new());
        MemoryMapBuilder {
            memory_map,
            next_free: 0,
        }
    }

    pub fn add_region(&mut self, new_region: MemoryRegion) -> Result<(), ()> {
        let node_index = self.next_free / REGIONS_PER_NODE;
        let inner_index = self.next_free % REGIONS_PER_NODE;
        let node = self.memory_map.get_node_mut(node_index).ok_or(())?;
        let region = node.get_region_mut(inner_index).ok_or(())?;
        if *region == MemoryRegion::empty() {
            *region = new_region;
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn add_node(&mut self, node: &'static mut MemoryMapNode) {
        self.memory_map.add_node(node);
    }

    pub fn finalize(self) -> &'static mut MemoryMap {
        self.memory_map
    }
}

#[derive(Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct MemoryMap {
    pub head: MemoryMapNode,
}

impl MemoryMap {
    pub fn new() -> Self {
        Self {
            head: MemoryMapNode::new(),
        }
    }

    pub fn get_node_mut(&mut self, node_index: usize) -> Option<&mut MemoryMapNode> {
        self.head.get_node_mut(node_index)
    }

    pub fn add_node(&mut self, node: &'static mut MemoryMapNode) {
        self.head.add_node(node);
    }
}

#[derive(Debug, Eq, PartialEq)]
#[repr(C)]
pub struct MemoryMapNode {
    regions: [MemoryRegion; REGIONS_PER_NODE],
    next: Option<&'static mut MemoryMapNode>,
}

impl MemoryMapNode {
    fn new() -> Self {
        MemoryMapNode {
            regions: [MemoryRegion::empty(); REGIONS_PER_NODE],
            next: None,
        }
    }

    pub fn get_node_mut(&mut self, node_index: usize) -> Option<&mut MemoryMapNode> {
        if node_index == 0 {
            Some(self)
        } else {
            self.next
                .as_mut()
                .and_then(|node| node.get_node_mut(node_index - 1))
        }
    }

    pub fn get_region_mut(&mut self, index: usize) -> Option<&mut MemoryRegion> {
        self.regions.get_mut(index)
    }

    pub fn add_node(&mut self, node: &'static mut MemoryMapNode) {
        match &mut self.next {
            None => self.next = Some(node),
            Some(next) => next.add_node(node),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(C)]
pub struct MemoryRegion {
    pub start: u64,
    pub end: u64,
    pub kind: MemoryRegionKind,
}

impl MemoryRegion {
    pub const fn empty() -> Self {
        MemoryRegion {
            start: 0,
            end: 0,
            kind: MemoryRegionKind::Empty,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(C)]
pub enum MemoryRegionKind {
    Usable,
    Empty,
    Reserved,
    Bootloader,
}

const REGIONS_PER_NODE: usize = (PAGE_SIZE - mem::size_of::<Option<&'static mut MemoryMapNode>>())
    / mem::size_of::<MemoryRegion>();
const _PADDING: usize = 8;
const _ASSERT_SIZE: [(); 4096] = [(); mem::size_of::<MemoryMapNode>() + _PADDING];

extern "C" fn _assert_ffi(_memory_map: MemoryMap) {}
