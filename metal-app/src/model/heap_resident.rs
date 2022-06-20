use crate::metal::*;

pub(crate) trait HeapResident<T: Sized> {
    fn heap_size(&self) -> usize;
    fn allocate_and_encode(
        &mut self,
        heap: &Heap,
        device: &Device,
        arg_size: u32,
    ) -> (Buffer, u32, T);
}
