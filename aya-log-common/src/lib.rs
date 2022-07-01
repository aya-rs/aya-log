#![no_std]

use core::{cmp, mem, ptr, slice};

pub const LOG_BUF_CAPACITY: usize = 8192;

pub const LOG_FIELDS: usize = 6;

#[repr(usize)]
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum Level {
    /// The "error" level.
    ///
    /// Designates very serious errors.
    Error = 1,
    /// The "warn" level.
    ///
    /// Designates hazardous situations.
    Warn,
    /// The "info" level.
    ///
    /// Designates useful information.
    Info,
    /// The "debug" level.
    ///
    /// Designates lower priority information.
    Debug,
    /// The "trace" level.
    ///
    /// Designates very low priority, often extremely verbose, information.
    Trace,
}

#[repr(usize)]
#[derive(Copy, Clone, Debug)]
pub enum RecordField {
    Target = 1,
    Level,
    Module,
    File,
    Line,
    NumArgs,
}

#[repr(usize)]
#[derive(Copy, Clone, Debug)]
pub enum ArgType {
    I8,
    I16,
    I32,
    I64,
    Isize,

    U8,
    U16,
    U32,
    U64,
    Usize,

    F32,
    F64,

    ArrU8Len16,
    ArrU16Len8,

    Str,
}

#[cfg(feature = "userspace")]
mod userspace {
    use super::*;

    unsafe impl aya::Pod for RecordField {}
    unsafe impl aya::Pod for ArgType {}
    unsafe impl aya::Pod for DisplayHint {}
}

/// All display hints
#[repr(usize)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DisplayHint {
    /// Default string representation.
    Default = 1,
    /// `:x`
    LowerHex,
    /// `:X`
    UpperHex,
    /// `:ipv4`, `:IPv4`
    IPv4,
    /// `:ipv6`, `:IPv6`
    IPv6,
}

pub struct TagLenValue<'a, T> {
    tag: T,
    hint: DisplayHint,
    value: &'a [u8],
}

impl<'a, T> TagLenValue<'a, T>
where
    T: Copy,
{
    #[inline(always)]
    pub fn new(tag: T, value: &'a [u8], hint: DisplayHint) -> TagLenValue<'a, T> {
        TagLenValue { tag, value, hint }
    }

    pub(crate) fn write(&self, mut buf: &mut [u8]) -> Result<usize, ()> {
        let size = mem::size_of::<T>()
            + mem::size_of::<DisplayHint>()
            + mem::size_of::<usize>()
            + self.value.len();
        if buf.len() < size {
            return Err(());
        }

        unsafe { ptr::write_unaligned(buf.as_mut_ptr() as *mut _, self.tag) };
        buf = &mut buf[mem::size_of::<T>()..];

        unsafe { ptr::write_unaligned(buf.as_mut_ptr() as *mut _, self.hint) };
        buf = &mut buf[mem::size_of::<usize>()..];

        unsafe { ptr::write_unaligned(buf.as_mut_ptr() as *mut _, self.value.len()) };
        buf = &mut buf[mem::size_of::<usize>()..];

        let len = cmp::min(buf.len(), self.value.len());
        buf[..len].copy_from_slice(&self.value[..len]);
        Ok(size)
    }
}

pub trait WriteToBuf {
    #[allow(clippy::result_unit_err)]
    fn write(&self, buf: &mut [u8], hint: DisplayHint) -> Result<usize, ()>;
}

macro_rules! impl_write_to_buf {
    ($type:ident, $arg_type:expr) => {
        impl WriteToBuf for $type {
            fn write(&self, buf: &mut [u8], hint: DisplayHint) -> Result<usize, ()> {
                TagLenValue::<ArgType>::new($arg_type, &self.to_ne_bytes(), hint).write(buf)
            }
        }
    };
}

impl_write_to_buf!(i8, ArgType::I8);
impl_write_to_buf!(i16, ArgType::I16);
impl_write_to_buf!(i32, ArgType::I32);
impl_write_to_buf!(i64, ArgType::I64);
impl_write_to_buf!(isize, ArgType::Isize);

impl_write_to_buf!(u8, ArgType::U8);
impl_write_to_buf!(u16, ArgType::U16);
impl_write_to_buf!(u32, ArgType::U32);
impl_write_to_buf!(u64, ArgType::U64);
impl_write_to_buf!(usize, ArgType::Usize);

impl_write_to_buf!(f32, ArgType::F32);
impl_write_to_buf!(f64, ArgType::F64);

impl WriteToBuf for [u8; 16] {
    fn write(&self, buf: &mut [u8], hint: DisplayHint) -> Result<usize, ()> {
        TagLenValue::<ArgType>::new(ArgType::ArrU8Len16, self, hint).write(buf)
    }
}

impl WriteToBuf for [u16; 8] {
    fn write(&self, buf: &mut [u8], hint: DisplayHint) -> Result<usize, ()> {
        let len = self.len() * 2;
        let ptr = self.as_ptr().cast::<u8>();
        let bytes = unsafe { slice::from_raw_parts(ptr, len) };
        TagLenValue::<ArgType>::new(ArgType::ArrU16Len8, bytes, hint).write(buf)
    }
}

impl WriteToBuf for str {
    fn write(&self, buf: &mut [u8], hint: DisplayHint) -> Result<usize, ()> {
        TagLenValue::<ArgType>::new(ArgType::Str, self.as_bytes(), hint).write(buf)
    }
}

#[allow(clippy::result_unit_err)]
#[doc(hidden)]
#[inline(always)]
pub fn write_record_header(
    buf: &mut [u8],
    target: &str,
    level: Level,
    module: &str,
    file: &str,
    line: u32,
    num_args: usize,
) -> Result<usize, ()> {
    let mut size = 0;
    for attr in [
        TagLenValue::<RecordField>::new(
            RecordField::Target,
            target.as_bytes(),
            DisplayHint::Default,
        ),
        TagLenValue::<RecordField>::new(
            RecordField::Level,
            &(level as usize).to_ne_bytes(),
            DisplayHint::Default,
        ),
        TagLenValue::<RecordField>::new(
            RecordField::Module,
            module.as_bytes(),
            DisplayHint::Default,
        ),
        TagLenValue::<RecordField>::new(RecordField::File, file.as_bytes(), DisplayHint::Default),
        TagLenValue::<RecordField>::new(
            RecordField::Line,
            &line.to_ne_bytes(),
            DisplayHint::Default,
        ),
        TagLenValue::<RecordField>::new(
            RecordField::NumArgs,
            &num_args.to_ne_bytes(),
            DisplayHint::Default,
        ),
    ] {
        size += attr.write(&mut buf[size..])?;
    }

    Ok(size)
}
