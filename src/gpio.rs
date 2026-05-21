use std::fs::{File, OpenOptions};
use std::io;
use std::mem;
use std::os::fd::{AsRawFd, FromRawFd};
use std::path::Path;

const GPIO_MAX_NAME_SIZE: usize = 32;
const GPIO_V2_LINES_MAX: usize = 64;
const GPIO_V2_LINE_NUM_ATTRS_MAX: usize = 10;

const GPIO_V2_LINE_FLAG_INPUT: u64 = 1 << 2;
const GPIO_V2_LINE_FLAG_OUTPUT: u64 = 1 << 3;
const GPIO_V2_LINE_ATTR_ID_OUTPUT_VALUES: u32 = 2;

const IOC_NRBITS: u64 = 8;
const IOC_TYPEBITS: u64 = 8;
const IOC_SIZEBITS: u64 = 14;

const IOC_NRSHIFT: u64 = 0;
const IOC_TYPESHIFT: u64 = IOC_NRSHIFT + IOC_NRBITS;
const IOC_SIZESHIFT: u64 = IOC_TYPESHIFT + IOC_TYPEBITS;
const IOC_DIRSHIFT: u64 = IOC_SIZESHIFT + IOC_SIZEBITS;

const IOC_READ: u64 = 2;
const IOC_WRITE: u64 = 1;

const fn ioc(dir: u64, ty: u64, nr: u64, size: usize) -> libc::c_ulong {
    ((dir << IOC_DIRSHIFT)
        | (ty << IOC_TYPESHIFT)
        | (nr << IOC_NRSHIFT)
        | ((size as u64) << IOC_SIZESHIFT)) as libc::c_ulong
}

const fn iowr<T>(ty: u64, nr: u64) -> libc::c_ulong {
    ioc(IOC_READ | IOC_WRITE, ty, nr, mem::size_of::<T>())
}

const GPIO_IOCTL_TYPE: u64 = 0xb4;
const GPIO_V2_GET_LINE_IOCTL: libc::c_ulong = iowr::<GpioV2LineRequest>(GPIO_IOCTL_TYPE, 0x07);
const GPIO_V2_LINE_GET_VALUES_IOCTL: libc::c_ulong =
    iowr::<GpioV2LineValues>(GPIO_IOCTL_TYPE, 0x0e);
const GPIO_V2_LINE_SET_VALUES_IOCTL: libc::c_ulong =
    iowr::<GpioV2LineValues>(GPIO_IOCTL_TYPE, 0x0f);

#[repr(C)]
#[derive(Clone, Copy)]
union GpioV2LineAttributeValue {
    flags: u64,
    values: u64,
    debounce_period_us: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct GpioV2LineAttribute {
    id: u32,
    padding: u32,
    value: GpioV2LineAttributeValue,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct GpioV2LineConfigAttribute {
    attr: GpioV2LineAttribute,
    mask: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct GpioV2LineConfig {
    flags: u64,
    num_attrs: u32,
    padding: [u32; 5],
    attrs: [GpioV2LineConfigAttribute; GPIO_V2_LINE_NUM_ATTRS_MAX],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct GpioV2LineRequest {
    offsets: [u32; GPIO_V2_LINES_MAX],
    consumer: [u8; GPIO_MAX_NAME_SIZE],
    config: GpioV2LineConfig,
    num_lines: u32,
    event_buffer_size: u32,
    padding: [u32; 5],
    fd: i32,
}

#[repr(C)]
struct GpioV2LineValues {
    bits: u64,
    mask: u64,
}

pub struct GpioController {
    line: File,
}

impl GpioController {
    pub const GPIO09_CHIP_PATH: &'static str = "/dev/gpiochip0";
    pub const GPIO09_LINE_OFFSET: u32 = 9;

    pub fn open_gpio09_input() -> io::Result<Self> {
        Self::open_input(Self::GPIO09_CHIP_PATH, Self::GPIO09_LINE_OFFSET)
    }

    pub fn open_gpio09_output(initial_high: bool) -> io::Result<Self> {
        Self::open_output(
            Self::GPIO09_CHIP_PATH,
            Self::GPIO09_LINE_OFFSET,
            initial_high,
        )
    }

    pub fn open_input(chip_path: impl AsRef<Path>, line_offset: u32) -> io::Result<Self> {
        Self::request_line(chip_path, line_offset, GPIO_V2_LINE_FLAG_INPUT, None)
    }

    pub fn open_output(
        chip_path: impl AsRef<Path>,
        line_offset: u32,
        initial_high: bool,
    ) -> io::Result<Self> {
        Self::request_line(
            chip_path,
            line_offset,
            GPIO_V2_LINE_FLAG_OUTPUT,
            Some(initial_high),
        )
    }

    pub fn read(&self) -> io::Result<bool> {
        let mut values = GpioV2LineValues { bits: 0, mask: 1 };
        ioctl_line_values(
            self.line.as_raw_fd(),
            GPIO_V2_LINE_GET_VALUES_IOCTL,
            &mut values,
        )?;
        Ok((values.bits & 1) != 0)
    }

    pub fn write(&self, high: bool) -> io::Result<()> {
        let mut values = GpioV2LineValues {
            bits: u64::from(high),
            mask: 1,
        };
        ioctl_line_values(
            self.line.as_raw_fd(),
            GPIO_V2_LINE_SET_VALUES_IOCTL,
            &mut values,
        )
    }

    fn request_line(
        chip_path: impl AsRef<Path>,
        line_offset: u32,
        flags: u64,
        initial_high: Option<bool>,
    ) -> io::Result<Self> {
        let chip = OpenOptions::new().read(true).write(true).open(chip_path)?;
        let mut request: GpioV2LineRequest = unsafe { mem::zeroed() };

        request.offsets[0] = line_offset;
        request.num_lines = 1;
        request.config.flags = flags;
        copy_consumer_label(&mut request.consumer, b"cn0102");

        if let Some(high) = initial_high {
            request.config.num_attrs = 1;
            request.config.attrs[0].attr.id = GPIO_V2_LINE_ATTR_ID_OUTPUT_VALUES;
            request.config.attrs[0].attr.value.values = u64::from(high);
            request.config.attrs[0].mask = 1;
        }

        let result = unsafe {
            libc::ioctl(
                chip.as_raw_fd(),
                GPIO_V2_GET_LINE_IOCTL,
                &mut request as *mut GpioV2LineRequest,
            )
        };

        if result < 0 {
            return Err(io::Error::last_os_error());
        }

        let line = unsafe { File::from_raw_fd(request.fd) };
        Ok(Self { line })
    }
}

fn ioctl_line_values(
    fd: i32,
    request: libc::c_ulong,
    values: &mut GpioV2LineValues,
) -> io::Result<()> {
    let result = unsafe { libc::ioctl(fd, request, values as *mut GpioV2LineValues) };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn copy_consumer_label(target: &mut [u8; GPIO_MAX_NAME_SIZE], source: &[u8]) {
    let len = source.len().min(GPIO_MAX_NAME_SIZE - 1);
    target[..len].copy_from_slice(&source[..len]);
}
