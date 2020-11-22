#![warn(clippy::pedantic)]

extern crate minivec;

pub mod png;

/// `RgbImg` represents an 8-bit RGB image.
///
pub struct RgbImg {
    r: minivec::MiniVec<u8>,
    g: minivec::MiniVec<u8>,
    b: minivec::MiniVec<u8>,
    height: u32,
    width: u32,
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
    pub fn from_packed_buf(buf: &[u8], height: u32, width: u32) -> RgbImg {
        let total = height as usize * width as usize;

        let mut r = minivec::MiniVec::<u8>::with_capacity(total);
        let mut g = minivec::MiniVec::<u8>::with_capacity(total);
        let mut b = minivec::MiniVec::<u8>::with_capacity(total);

        let r_ptr = r.as_mut_ptr();
        let g_ptr = g.as_mut_ptr();
        let b_ptr = b.as_mut_ptr();

        buf.chunks_exact(3)
            .enumerate()
            .for_each(|(idx, pixel)| -> () {
                unsafe {
                    r_ptr.add(idx).write(pixel[0]);
                    g_ptr.add(idx).write(pixel[1]);
                    b_ptr.add(idx).write(pixel[2]);
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
            unsafe {
                let base_offset = idx as usize * 3;
                vec[base_offset + 0].as_mut_ptr().write(r[idx]);
                vec[base_offset + 1].as_mut_ptr().write(g[idx]);
                vec[base_offset + 2].as_mut_ptr().write(b[idx]);
            }
        }

        unsafe { std::mem::transmute::<_, Vec<u8>>(vec) }
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
    pub fn height(&self) -> u32 {
        self.height
    }

    /// `width` returns the number of columns contained in the image data.
    ///
    pub fn width(&self) -> u32 {
        self.width
    }

    /// `total` is named after its OpenCV counterpart and returns the number of pixels contained
    /// in the image data.
    ///
    pub fn total(&self) -> u32 {
        self.height() * self.width()
    }
}
