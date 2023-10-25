#[repr(C)]
#[derive(AscType)]
pub struct Array<T> {
    buffer: AscPtr<ArrayBuffer>,
    length: u32,
    ty: PhantomData<T>,
}

impl<T: AscValue> Array<T> {
    pub fn new<H: AscHeap + ?Sized>(
        content: &[T],
        heap: &mut H,
        gas: &GasCounter,
    ) -> Result<Self, HostExportError> {
        let arr_buffer = class::ArrayBuffer::new(content, heap.api_version())?;
        let arr_buffer_ptr = AscPtr::alloc_obj(arr_buffer, heap, gas)?;
        Ok(Array {
            buffer: AscPtr::new(arr_buffer_ptr.wasm_ptr()),
            // If this cast would overflow, the above line has already panicked.
            length: content.len() as u32,
            ty: PhantomData,
        })
    }

    pub(crate) fn to_vec<H: AscHeap + ?Sized>(
        &self,
        heap: &H,
        gas: &GasCounter,
    ) -> Result<Vec<T>, DeterministicHostError> {
        self.buffer
            .read_ptr(heap, gas)?
            .get(0, self.length, heap.api_version())
    }
}
