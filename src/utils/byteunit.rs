use std::fmt::{Display, Formatter, Result};

#[allow(dead_code)]
pub const IEC: ByteUnit<()> = ByteUnit::IEC(());
pub const SI: ByteUnit<()> = ByteUnit::SI(());

pub trait ToByteUnit<T: Copy> {
    fn to_byteunit(self, byte: ByteUnit<()>) -> ByteUnit<T>;
}

impl ToByteUnit<u32> for u32 where i64: From<u32> {
    fn to_byteunit(self, unit: ByteUnit<()>) -> ByteUnit<u32> {
        match unit {
            ByteUnit::IEC(()) => ByteUnit::IEC(self),
            ByteUnit::SI(()) => ByteUnit::SI(self),
        } 
    }
}

impl ToByteUnit<i64> for i64 where i64: From<i64> {
    fn to_byteunit(self, unit: ByteUnit<()>) -> ByteUnit<i64> {
        match unit {
            ByteUnit::IEC(()) => ByteUnit::IEC(self),
            ByteUnit::SI(()) => ByteUnit::SI(self),
        } 
    }
}

pub enum ByteUnit<T: Copy> {
    IEC(T),
    SI(T),
}

impl <T>ByteUnit<T> where i64: From<T>, T: Copy {
    fn val(&self) -> T {
        match self {
            Self::IEC(val) => *val, Self::SI(val) => *val
        }
    }
    
    fn diviser(&self) -> (f64, f64) {
        match self {
            Self::IEC(_) => (1024.0, -1024.0), Self::SI(_) => (1000.0, -1000.0)
        }
    }

    fn unit_suffix<'a>(&self, i: i8) -> &'a str {
        match self {
            Self::IEC(_) => match i {
                0 => "KiB",
                1 => "MiB",
                2 => "GiB",
                3 => "TiB",
                4 => "PiB",
                5 => "EiB",
                _ => "B"
            },
            Self::SI(_) => match i {
                0 => "KB",
                1 => "MB",
                2 => "GB",
                3 => "TB",
                4 => "PB",
                5 => "EB",
                _ => "B"
            }
        }
    }
}

impl<T> Display for ByteUnit<T> where i64: From<T>, T: Copy {
    fn fmt(&self, f:&mut Formatter<'_>) -> Result {
        let bytes: i64 = self.val().into();
        let diviser: (f64, f64) = self.diviser(); 
        let conditional: f64 = if bytes > -1 { diviser.0 } else { diviser.1 };
        let mut size: f64 = bytes as f64;
        let mut idx: i8 = -1;

        while if bytes > -1 { size > conditional } else { size < conditional } {
            size = size / diviser.0;
            idx += 1;
        }
   
        if idx == -1 {
            write!(f, "{:.0} {}", size, self.unit_suffix(idx))
        } else {
            write!(f, "{:.2} {}", size, self.unit_suffix(idx)) 
        }
    }
}
