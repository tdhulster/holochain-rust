use holochain_wasm_utils::memory::{
    allocation::{AllocationError, AllocationResult, Length, WasmAllocation},
    stack::WasmStack,
    MemoryBits, MemoryInt,
};
use wasmer_runtime::Instance;

//--------------------------------------------------------------------------------------------------
// WASM Memory Manager
//--------------------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
/// Struct for managing a WASM Memory Instance as a single page memory stack
pub struct WasmPageManager {
    stack: WasmStack,
}

/// A Memory Manager limited to one wasm memory page that works like a stack.
/// With this Memory Manager, the WASM host (i.e. the Ribosome) and WASM module (i.e. the Zome)
/// only need to pass around an i64 to communicate any data.
/// That u64 is the last memory allocation on the stack:
/// it is split in an i16 'offset' in the upper bits and an i16 'length' in the lower bits.
/// This fits with the 64KiB sized of a memory Page.
/// Complex input arguments should be stored on the latest allocation on the stack.
/// Complex output arguments can be stored anywhere on stack.
/// Since zero sized allocations are not allowed,
/// it is possible to pass around a return and/or error code with the following convention:
/// using the i16 'offset' as return code and i16 'length' set to zero
/// to indicate its a return code.
/// Return code of 0 means success, while any other value means a failure and gives the error code.
/// In the future, to handle bigger memory needs, we could do same with an i64 instead
/// and handle multiple memory Pages.
impl WasmPageManager {
    pub fn new() -> Self {
        // get wasm memory reference from module
        // let wasm_memory = wasm_instance
        //     .export_by_name("memory")
        //     .expect("all modules compiled with rustc should have an export named 'memory'; qed")
        //     .as_memory()
        //     .expect("in module generated by rustc export named 'memory' should be a memory; qed")
        //     .clone();

        WasmPageManager {
            stack: WasmStack::default(),
        }
    }

    /// Allocate on stack without writing in it
    pub fn allocate(&mut self, length: Length) -> AllocationResult {
        let allocation = self.stack.next_allocation(length)?;
        let top = self.stack.allocate(allocation)?;
        Ok(WasmAllocation::new(MemoryInt::from(top).into(), length)?)
    }

    /// Write data on top of stack
    pub fn write(&mut self, instance: &mut Instance, data: &[u8]) -> AllocationResult {
        if data.len() as MemoryBits > WasmAllocation::max() {
            return Err(AllocationError::OutOfBounds);
        }

        if data.is_empty() {
            return Err(AllocationError::ZeroLength);
        }

        // scope for mutable borrow of self
        let allocation = self.allocate((data.len() as MemoryInt).into())?;

        // @TODO make this work when wasmer is used consistently inside/outside wasm
        // let top_bytes = Bytes(MemoryInt::from(self.stack.top()) as usize);
        // let top_pages: Pages = top_bytes.round_up_to();
        // let current_pages: Pages = self.wasm_memory.current_size();

        // if current_pages < top_pages {
        //     match self.wasm_memory.grow(top_pages - current_pages) {
        //         Ok(new_pages) => assert_eq!(new_pages, top_pages),
        //         Err(_) => return Err(AllocationError::OutOfBounds),
        //     }
        // }

        let memory = instance.context_mut().memory(0);

        for (byte, cell) in data.iter().zip(
            memory.view()[MemoryInt::from(allocation.start()) as usize
                ..MemoryInt::from(allocation.end()) as usize]
                .iter(),
        ) {
            cell.set(byte.to_owned());
        }

        // .set(MemoryInt::from(mem_buf.offset()), &data)
        // .expect("memory should be writable");

        Ok(allocation)
    }

    /// Read data somewhere in stack
    pub fn read(&self, instance: &Instance, allocation: WasmAllocation) -> Vec<u8> {
        // self.wasm_memory
        //     .get(
        //         MemoryInt::from(allocation.offset()),
        //         MemoryInt::from(allocation.length()) as usize,
        //     )
        //     .expect("Successfully retrieve the result")
        let memory = instance.context().memory(0);

        memory.view()[MemoryInt::from(allocation.start()) as usize
            ..MemoryInt::from(allocation.end()) as usize]
            .iter()
            .map(|cell| cell.get())
            .collect()
    }
}
