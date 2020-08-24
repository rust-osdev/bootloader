#[derive(Debug)]
pub struct Config {
    pub map_physical_memory: bool,
    pub map_page_table_recursively: bool,
    pub kernel_stack_size: Option<u64>,
    pub physical_memory_offset: Option<u64>,
    pub kernel_stack_address: Option<u64>,
    pub boot_info_address: Option<u64>,
    pub framebuffer_address: Option<u64>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            map_physical_memory: false,
            map_page_table_recursively: false,
            physical_memory_offset: None,
            kernel_stack_address: None,
            kernel_stack_size: None,
            boot_info_address: None,
            framebuffer_address: None,
        }
    }
}
