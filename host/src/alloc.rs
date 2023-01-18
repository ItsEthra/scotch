use parking_lot::Mutex;
use wasmer::{AsStoreMut, Memory, MemoryError, WASM_PAGE_SIZE};

#[derive(Debug, Clone, Copy)]
pub struct WasmAllocatorOptions {
    pub page_count: u32,
    pub block_size: u32,

    max_level: u32,
}

impl WasmAllocatorOptions {
    #[inline]
    pub fn new(page_count: u32, block_size: u32) -> Self {
        Self {
            page_count,
            block_size,
            // Recomputed when passed to `WasmAllocator::new`
            max_level: 0,
        }
    }
}

impl Default for WasmAllocatorOptions {
    #[inline]
    fn default() -> Self {
        Self::new(1, 512)
    }
}

#[derive(Debug)]
struct Node {
    order: u32,
    start: u32,
    state: NodeState,
}

#[derive(Debug)]
enum NodeState {
    Occupied,
    Vacant,
    Seeding(Box<(Node, Node)>),
}

impl Node {
    fn deoccupy_block_at(&mut self, address: u32) {
        if let NodeState::Seeding(ref mut children) = self.state {
            let (left, right) = (&mut children.0, &mut children.1);

            let step_in = |node: &mut Node| {
                match node.state {
                    NodeState::Occupied if node.start == address => node.state = NodeState::Vacant,
                    NodeState::Seeding(_) => node.deoccupy_block_at(address),
                    _ => {}
                };
            };

            step_in(left);
            step_in(right);

            if matches!(
                (&left.state, &right.state),
                (NodeState::Vacant, NodeState::Vacant)
            ) {
                self.state = NodeState::Vacant;
            }
        }
    }

    fn occupy_free_block(&mut self, order: u32, opts: &WasmAllocatorOptions) -> Option<u32> {
        if order > opts.max_level {
            return None;
        }

        match self.state {
            NodeState::Vacant if self.order == order => {
                self.state = NodeState::Occupied;
                Some(self.start)
            }
            NodeState::Vacant => {
                if self.order == 0 {
                    return None;
                }

                let left = Node {
                    order: self.order - 1,
                    start: self.start,
                    state: NodeState::Vacant,
                };

                let right = Node {
                    order: self.order - 1,
                    start: self.start + self.order * opts.block_size,
                    state: NodeState::Vacant,
                };

                self.state = NodeState::Seeding(Box::new((left, right)));
                self.occupy_free_block(order, opts)
            }
            NodeState::Seeding(ref mut children) => {
                let (left, right) = (&mut children.0, &mut children.1);

                if let Some(addr) = left.occupy_free_block(order, opts) {
                    Some(addr)
                } else if let Some(addr) = right.occupy_free_block(order, opts) {
                    Some(addr)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct WasmAllocator {
    base: u32,
    root: Mutex<Node>,
    options: WasmAllocatorOptions,
}

impl WasmAllocator {
    pub fn new(
        store: &mut impl AsStoreMut,
        memory: &Memory,
        options: WasmAllocatorOptions,
    ) -> Result<Self, MemoryError> {
        let max_level = options.page_count * WASM_PAGE_SIZE as u32 / options.block_size;
        let last_page = memory.grow(store, 2u32.pow(options.page_count))?.0;
        let root = Node {
            order: max_level,
            start: 0,
            state: NodeState::Vacant,
        }
        .into();

        Ok(Self {
            base: last_page * WASM_PAGE_SIZE as u32,
            options: WasmAllocatorOptions {
                max_level,
                ..options
            },
            root,
        })
    }

    // TODO: Currenly when all space is exhaused allocator does not grow wasm memory which
    // is probably not desired.
    pub fn alloc(&self, size: u32) -> Option<u32> {
        self.root
            .lock()
            .occupy_free_block(size / self.options.block_size, &self.options)
            .map(|offset| self.base + offset)
    }

    pub fn free(&self, address: u32) {
        self.root.lock().deoccupy_block_at(address)
    }
}
