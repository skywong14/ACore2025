// os/src/mm/memory_set.rs
// implementation of MapArea and MemorySet

use alloc::vec::Vec;
use crate::mm::area::MapArea;
use crate::mm::page_table::PageTable;


// ----- MemorySet -----
pub struct MemorySet {
    pub(crate) page_table: PageTable,
    pub(crate) areas: Vec<MapArea>,
}

impl MemorySet {
    // ----- constructor -----
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
        }
    }

    // ----- methods -----
    // map a new MapArea to the MemorySet
    // 'data' as the initial data (when map_type is Framed)
    fn map_area(&mut self, mut area: MapArea, data: Option<&[u8]>) {
        println!(
            "[mem] Map area of [{:#x}, {:#x})",
            area.vpn_range.start.0,
            area.vpn_range.end.0,
        );
        area.map_page_table(&mut self.page_table);
        if let Some(data) = data {
            area.copy_data(&mut self.page_table, data);
        }
        self.areas.push(area);
    }


}