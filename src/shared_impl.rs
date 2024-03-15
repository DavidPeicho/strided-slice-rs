use std::marker::PhantomData;

#[derive(Copy, Clone, PartialEq)]
pub enum SliceError {
    OffsetOutOfBounds,
    SliceSizeNotMatchingStride,
    StrideOutOfBounds,
    AlignmentFault,
}

impl std::fmt::Debug for SliceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OffsetOutOfBounds => write!(f, "Offset out of range"),
            Self::SliceSizeNotMatchingStride => {
                write!(f, "Slice byte len isn't a multiple of the stride")
            }
            Self::StrideOutOfBounds => write!(f, "Stride out-of-bounds"),
            Self::AlignmentFault => write!(f, "Alignment fault"),
        }
    }
}

#[derive(Clone, Copy)]
pub struct SliceData<Attr: Sized> {
    pub(crate) data: *const u8,
    bytes: usize,
    stride: usize,
    _phantom: PhantomData<Attr>,
}

impl<Attr: Sized> SliceData<Attr> {
    pub(crate) fn new_typed<V: Pod>(
        data: &[V],
        offset: usize,
        elt_count: usize,
    ) -> Result<Self, SliceError> {
        let stride = std::mem::size_of::<V>() * elt_count;
        let bytes = data.len() * std::mem::size_of::<V>();
        let ptr = data.as_ptr().cast::<u8>();
        Self::new(ptr, offset, stride, bytes)
    }

    pub(crate) fn new(
        ptr: *const u8,
        offset: usize,
        stride: usize,
        bytes: usize,
    ) -> Result<Self, SliceError> {
        println!("stride {}, offset {}, bytes {},", stride, offset, bytes);
        let ptr: *const u8 = unsafe { ptr.add(offset) };
        // Empty slice are allowed, but we need to ensure that
        // the offset and stride are valid.
        if std::mem::size_of::<Attr>() > stride {
            Err(SliceError::OffsetOutOfBounds)
        } else if bytes > 0 && stride > bytes {
            Err(SliceError::StrideOutOfBounds)
        } else if ptr.align_offset(std::mem::align_of::<Attr>()) != 0 {
            Err(SliceError::AlignmentFault)
        } else {
            Ok(Self {
                data: ptr,
                bytes,
                stride,
                _phantom: PhantomData,
            })
        }
    }

    pub(crate) fn start(&self) -> *const u8 {
        self.data
    }

    pub(crate) fn end(&self) -> *const u8 {
        let count = self.len();
        if count > 0 {
            self.get(count - 1).unwrap()
        } else {
            self.data
        }
    }

    /// Number of elements in the slice
    pub fn len(&self) -> usize {
        self.bytes.div_ceil(self.stride)
    }

    /// `true` if the slice has size `0`, `false` otherwise
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a pointer to the element at index `index`
    pub fn get(&self, index: usize) -> Option<*const u8> {
        if index < self.len() {
            let start = self.stride * index;
            Some(unsafe { self.data.add(start) })
        } else {
            None
        }
    }

    /// Slice strides, in **bytes**
    pub fn stride(&self) -> usize {
        self.stride
    }
}

macro_rules! impl_iterator {
    ($name: ident -> $elem: ty) => {
        impl<'a, T: Pod> Iterator for $name<'a, T> {
            type Item = $elem;

            fn next(&mut self) -> Option<$elem> {
                // `end` is inclusive to avoid adding some chance of making
                // an invalid read, causing undefined behavior.
                if self.start > self.end {
                    return None;
                }
                unsafe {
                    let ret = Some(std::mem::transmute::<_, $elem>(self.start));
                    self.start = self.start.add(self.stride);
                    ret
                }
            }
        }
    };
}

use bytemuck::Pod;
pub(super) use impl_iterator;
