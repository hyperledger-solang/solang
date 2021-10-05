use std::alloc::Layout;

#[derive(Debug)]
pub struct Allocator {
    start: u64,
    len: u64,
    pos: u64,
}

impl Allocator {
    pub fn new(len: u64, virtual_address: u64) -> Self {
        Allocator {
            start: virtual_address,
            len,
            pos: 0,
        }
    }
}

impl Allocator {
    pub fn alloc(&mut self, layout: Layout) -> u64 {
        let bytes_to_align = (self.pos as *const u8).align_offset(layout.align()) as u64;
        if self
            .pos
            .saturating_add(layout.size() as u64)
            .saturating_add(bytes_to_align)
            <= self.len
        {
            self.pos += bytes_to_align;
            let addr = self.start + self.pos;
            self.pos += layout.size() as u64;
            addr
        } else {
            panic!("out of heap");
        }
    }

    pub fn dealloc(&mut self, _addr: u64, _layout: Layout) {
        // It's a bump allocator, free not supported
    }
}
