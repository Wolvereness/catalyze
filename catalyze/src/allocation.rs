use std::alloc::{Layout, LayoutError};
use std::iter::{Copied, repeat_with};
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ptr::{slice_from_raw_parts, slice_from_raw_parts_mut};
use std::slice;
use std::sync::Arc;

pub struct Lengths<T> {
    lengths: Vec<(usize, usize)>,
    layout: Layout,
    _ghost: PhantomData<fn() -> T>,
}

impl<T> Clone for Lengths<T> {
    fn clone(&self) -> Self {
        Lengths {
            lengths: self.lengths.clone(),
            layout: self.layout,
            _ghost: PhantomData,
        }
    }
}

impl Default for Lengths<()> {
    fn default() -> Self {
        Lengths::new()
    }
}

impl Lengths<()> {
    pub const fn new() -> Self {
        Lengths {
            lengths: Vec::new(),
            layout: Layout::new::<()>(),
            _ghost: PhantomData,
        }
    }
}

impl<T> Lengths<T> {
    pub fn and<V>(mut self, len: usize) -> Result<Lengths<(V, T)>, LayoutError> {
        let layout = Layout::array::<V>(len)?;
        let (layout, offset) = layout.extend(self.layout)?;
        self.lengths.push((len, offset));
        Ok(Lengths {
            lengths: self.lengths,
            layout,
            _ghost: PhantomData,
        })
    }
}

impl<T1, T2> Lengths<(T1, T2)> {
    #[inline(always)]
    pub fn backing(&self) -> Arc<[MaybeUninit<u8>]> {
        backing(self.layout)
    }

    pub fn write<'a, 'b>(&'a self, data: &'b mut [MaybeUninit<u8>]) -> SlicesWrite<'a, 'b, (T1, T2)> {
        assert_eq!(data.len(), len(self.layout), "Improperly sized alloc");
        let data = data.split_at_mut(data.as_mut_ptr().align_offset(self.layout.align())).1;
        SlicesWrite {
            iter: self.lengths.iter().copied(),
            data,
            _ghost: PhantomData,
        }
    }

    pub fn read<'a, 'b>(&'a self, data: &'b [MaybeUninit<u8>]) -> SlicesRead<'a, 'b, (T1, T2)> {
        assert_eq!(data.len(), len(self.layout), "Improperly sized alloc");
        let data = data.split_at(data.as_ptr().align_offset(self.layout.align())).1;
        SlicesRead {
            iter: self.lengths.iter().copied(),
            data,
            _ghost: PhantomData,
        }
    }
}

fn len(layout: Layout) -> usize {
    layout.size()
        .checked_add(layout.align()).expect("Memory overflow")
        .checked_sub(1).expect("Unexpected ZST")
}

fn backing(layout: Layout) -> Arc<[MaybeUninit<u8>]> {
    // TrustedLen implies single-alloc
    // (maybe compiler optimizes away the memcopy?)
    repeat_with(MaybeUninit::uninit).take(len(layout)).collect()
}

/// Constructed from [`Lengths::write()`](Lengths@write())
pub struct SlicesWrite<'a, 'b, T> {
    iter: Copied<slice::Iter<'a, (usize, usize)>>,
    data: &'b mut [MaybeUninit<u8>],
    _ghost: PhantomData<fn() -> &'b mut T>,
}

impl<'a, 'b, T1, T2> SlicesWrite<'a, 'b, (T1, T2)> {
    pub fn next(self) -> (&'b mut [MaybeUninit<T1>], SlicesWrite<'a, 'b, T2>) {
        let SlicesWrite {
            mut iter,
            data,
            ..
        } = self;
        let (length, offset) = iter.next_back().unwrap();
        let (slice, data) = data.split_at_mut(offset);
        // SAFETY: ptr is aligned and correct length per offsets of Lengths construction
        let slice = unsafe { &mut *slice_from_raw_parts_mut::<MaybeUninit<T1>>(slice.as_mut_ptr() as *mut MaybeUninit<T1>, length) };
        (
            slice,
            SlicesWrite {
                iter,
                data,
                _ghost: PhantomData,
            },
        )
    }
}

/// Constructed from [`Lengths::read()`](Lengths@read())
pub struct SlicesRead<'a, 'b, T> {
    iter: Copied<slice::Iter<'a, (usize, usize)>>,
    data: &'b [MaybeUninit<u8>],
    _ghost: PhantomData<fn() -> &'b T>,
}

impl<T> Clone for SlicesRead<'_, '_, T> {
    fn clone(&self) -> Self {
        SlicesRead {
            iter: self.iter.clone(),
            data: self.data,
            _ghost: PhantomData,
        }
    }
}

impl<'a, 'b, T1, T2> SlicesRead<'a, 'b, (T1, T2)> {
    pub fn next(self) -> (&'b [MaybeUninit<T1>], SlicesRead<'a, 'b, T2>) {
        let SlicesRead {
            mut iter,
            data,
            ..
        } = self;
        let (length, offset) = iter.next_back().unwrap();
        let (slice, data) = data.split_at(offset);
        // SAFETY: ptr is aligned and correct length per offsets of Lengths construction
        let slice = unsafe { &*slice_from_raw_parts::<MaybeUninit<T1>>(slice.as_ptr() as *const MaybeUninit<T1>, length) };
        (
            slice,
            SlicesRead {
                iter,
                data,
                _ghost: PhantomData,
            },
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn syntax() {
        let lengths = Lengths::new()
            .and::<u8>(19).unwrap()
            .and::<String>(10).unwrap()
            .and::<(u16, u32)>(7).unwrap();
        let mut allocation: Arc<[MaybeUninit<u8>]> = lengths.backing();
        assert_eq!(allocation.len(), 322);
        let slices = lengths.write(Arc::get_mut(&mut allocation).unwrap());
        let (nums, slices): (&mut [MaybeUninit<(u16, u32)>], _) = slices.next();
        let (strings, slices): (&mut [MaybeUninit<String>], _) = slices.next();
        let (bytes, _): (&mut [MaybeUninit<u8>], _) = slices.next();

        assert_eq!(nums.len(), 7);
        assert_eq!(strings.len(), 10);
        assert_eq!(bytes.len(), 19);
    }
}
