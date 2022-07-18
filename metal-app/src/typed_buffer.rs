use metal::{Buffer, DeviceRef, HeapRef, MTLResourceOptions, MTLSizeAndAlign};
use std::marker::PhantomData;

pub trait MetalBufferAllocator {
    fn with_capacity<T: Sized>(&self, capacity: usize, options: MTLResourceOptions) -> Buffer;
}

impl MetalBufferAllocator for DeviceRef {
    fn with_capacity<T: Sized>(&self, capacity: usize, options: MTLResourceOptions) -> Buffer {
        self.new_buffer((std::mem::size_of::<T>() * capacity) as _, options)
    }
}

impl MetalBufferAllocator for HeapRef {
    fn with_capacity<T: Sized>(&self, capacity: usize, options: MTLResourceOptions) -> Buffer {
        self.new_buffer((std::mem::size_of::<T>() * capacity) as _, options)
            .expect("Failed to heap allocate buffer")
    }
}

pub struct TypedBufferSizer<T: Sized + Copy + Clone> {
    pub num_of_elements: usize,
    pub options: MTLResourceOptions,
    _phantom: PhantomData<T>,
}

impl<T: Sized + Copy + Clone> TypedBufferSizer<T> {
    pub fn new(num_of_elements: usize, options: MTLResourceOptions) -> Self {
        Self {
            num_of_elements,
            options,
            _phantom: PhantomData,
        }
    }

    pub fn heap_aligned_byte_size(&self, device: &DeviceRef) -> usize {
        #[inline(always)]
        pub const fn align_size(MTLSizeAndAlign { size, align }: MTLSizeAndAlign) -> usize {
            (size + (align - (size & (align - 1)))) as _
        }
        align_size(device.heap_buffer_size_and_align(
            (std::mem::size_of::<T>() * self.num_of_elements) as _,
            self.options,
        ))
    }

    pub fn allocate<A: MetalBufferAllocator>(&self, label: &str, allocator: &A) -> TypedBuffer<T> {
        TypedBuffer::with_capacity(label, allocator, self.num_of_elements, self.options)
    }
}
pub struct TypedBuffer<T: Sized + Copy + Clone> {
    pub buffer: Buffer,
    pub len: usize,
    _type: PhantomData<T>,
}

impl<T: Sized + Copy + Clone> TypedBuffer<T> {
    #[inline]
    pub fn with_capacity<A: MetalBufferAllocator>(
        label: &str,
        allocator: &A,
        capacity: usize,
        options: MTLResourceOptions,
    ) -> Self {
        let buffer = allocator.with_capacity::<T>(capacity, options);
        buffer.set_label(label);
        Self {
            buffer,
            len: capacity,
            _type: PhantomData,
        }
    }

    #[inline]
    pub fn from_data<A: MetalBufferAllocator>(
        label: &str,
        allocator: &A,
        data: &[T],
        options: MTLResourceOptions,
    ) -> Self {
        let tb = Self::with_capacity(label, allocator, data.len(), options);
        tb.get_mut().clone_from_slice(data);
        tb
    }

    pub fn element_size(&self) -> usize {
        std::mem::size_of::<T>()
    }
    // TODO: Add update_data or get_mut function

    pub fn get_mut(&self) -> &mut [T] {
        let contents = self.buffer.contents() as *mut T;
        unsafe { std::slice::from_raw_parts_mut(contents, self.len) }
    }
}
