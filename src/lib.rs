#![warn(clippy::pedantic)]

extern crate minivec;

pub mod png;

/// `RgbImg` represents an 8-bit RGB image. The image data is stored in a channel-major order so
/// that is more easily translates to SIMD/SIMT architectures (consider something like CUDA).
///
pub struct RgbImg {
    r: minivec::MiniVec<u8>,
    g: minivec::MiniVec<u8>,
    b: minivec::MiniVec<u8>,
    height: usize,
    width: usize,
}

impl RgbImg {
    /// `new` returns an empty `RgbImg` that has allocated no memory and has a height and width of
    /// `0`.
    ///
    pub fn new() -> RgbImg {
        RgbImg {
            r: minivec::MiniVec::new(),
            g: minivec::MiniVec::new(),
            b: minivec::MiniVec::new(),
            height: 0,
            width: 0,
        }
    }

    /// `from_packed_buf` will construct an `RgbImg` from a user-supplied buffer, using the provided
    /// `height` and `width` for image dimensions.
    ///
    /// Internally `RgbImg` stores its image data across 3 allocations in a channel-major ordering.
    /// But many libraries operate natively in terms of row-major densely packed image data so this
    /// function is meant to help inter-operate with them.
    ///
    /// Note: this function _copies_ the supplied buf so its time and space complexity are both
    /// `O(buf.len())`.
    ///
    pub fn from_packed_buf(buf: &[u8], height: usize, width: usize) -> RgbImg {
        let total = height * width;

        let mut r = minivec::MiniVec::<u8>::with_capacity(total);
        let mut g = minivec::MiniVec::<u8>::with_capacity(total);
        let mut b = minivec::MiniVec::<u8>::with_capacity(total);

        let (r_buf, g_buf, b_buf) = (
            r.spare_capacity_mut(),
            g.spare_capacity_mut(),
            b.spare_capacity_mut(),
        );

        buf.chunks_exact(3)
            .enumerate()
            .for_each(|(idx, pixel)| -> () {
                unsafe {
                    r_buf[idx].as_mut_ptr().write(pixel[0]);
                    g_buf[idx].as_mut_ptr().write(pixel[1]);
                    b_buf[idx].as_mut_ptr().write(pixel[2]);
                }
            });

        unsafe {
            r.set_len(total);
            g.set_len(total);
            b.set_len(total);
        }

        Self {
            r,
            g,
            b,
            height,
            width,
        }
    }

    /// `to_packed_buf` writes the contained RGB data into a single contiguous buffer and then
    /// returns it to the caller.
    ///
    /// Internally, `RgbImg` stores all of its data in a channel-major order using 3 separate
    /// allocations. While this is an ideal layout for most operations, sometimes a densely-packed
    /// row-major ordering of image data is required.
    ///
    pub fn to_packed_buf(&self) -> Vec<u8> {
        let (r, g, b) = (self.r(), self.g(), self.b());
        let len = self.total() as usize * 3;
        let mut vec = vec![std::mem::MaybeUninit::<u8>::uninit(); len];

        for idx in 0..self.total() as usize {
            let base_offset = idx as usize * 3;

            vec[base_offset + 0] = std::mem::MaybeUninit::new(r[idx]);
            vec[base_offset + 1] = std::mem::MaybeUninit::new(g[idx]);
            vec[base_offset + 2] = std::mem::MaybeUninit::new(b[idx]);
        }

        let mut vec = core::mem::ManuallyDrop::new(vec);
        unsafe { Vec::from_raw_parts(vec.as_mut_ptr() as *mut u8, vec.len(), vec.capacity()) }
    }

    /// `r` will return a read-only slice pointing to the image data's red channel.
    ///
    pub fn r(&self) -> &[u8] {
        &self.r
    }

    /// `g` will return a read-only slice pointing to the image data's green channel.
    ///
    pub fn g(&self) -> &[u8] {
        &self.g
    }

    /// `b` will return a read-only slice pointing to the image data's blue channel.
    pub fn b(&self) -> &[u8] {
        &self.b
    }

    /// `height` returns the number of rows contained in the image data.
    ///
    pub fn height(&self) -> usize {
        self.height
    }

    /// `width` returns the number of columns contained in the image data.
    ///
    pub fn width(&self) -> usize {
        self.width
    }

    /// `total` is named after its OpenCV counterpart and returns the number of pixels contained
    /// in the image data.
    ///
    pub fn total(&self) -> usize {
        self.height() * self.width()
    }

    /// `iter` returns an `RgbIter` over the current image data. The iterator returns a tuple
    /// `(u8, u8, u8)` in `(R, G, B)` ordering.
    ///
    pub fn iter(&self) -> RgbIter {
        RgbIter {
            r_: self.r().iter(),
            g_: self.g().iter(),
            b_: self.b().iter(),
        }
    }
}

pub struct RgbIter<'a> {
    r_: std::slice::Iter<'a, u8>,
    g_: std::slice::Iter<'a, u8>,
    b_: std::slice::Iter<'a, u8>,
}

impl<'a> std::iter::Iterator for RgbIter<'a> {
    type Item = (u8, u8, u8);

    fn next(&mut self) -> Option<Self::Item> {
        match (self.r_.next(), self.g_.next(), self.b_.next()) {
            (Some(r), Some(g), Some(b)) => Some((*r, *g, *b)),
            _ => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.r_.size_hint()
    }
}

pub struct GreyscaleIter<Iter>
where
    Iter: std::iter::Iterator<Item = (u8, u8, u8)>,
{
    iter_: Iter,
}

impl<Iter> GreyscaleIter<Iter>
where
    Iter: std::iter::Iterator<Item = (u8, u8, u8)>,
{
    fn new(iter: Iter) -> Self {
        Self { iter_: iter }
    }
}

impl<Iter> std::iter::Iterator for GreyscaleIter<Iter>
where
    Iter: std::iter::Iterator<Item = (u8, u8, u8)>,
{
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter_.next().map(|(r, g, b)| -> u8 {
            let r = (r as f64) / 255.0;
            let g = (g as f64) / 255.0;
            let b = (b as f64) / 255.0;

            let grey = 0.21263901 * r + 0.71516868 * g + 0.07219232 * b;

            if grey >= 1.0 {
                255
            } else {
                (255.0 * grey) as u8
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter_.size_hint()
    }
}

pub trait Greyscale: std::iter::Iterator<Item = (u8, u8, u8)>
where
    Self: Sized,
{
    fn greyscale(self) -> GreyscaleIter<Self> {
        GreyscaleIter::new(self)
    }
}

impl<Iter> Greyscale for Iter where Iter: std::iter::Iterator<Item = (u8, u8, u8)> {}
