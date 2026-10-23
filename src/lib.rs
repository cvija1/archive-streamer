use libarchive2::{CallbackReader, FileType, ReadArchive};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use std::path::{Component, PathBuf};
use std::sync::mpsc::Sender;

#[cfg(target_os = "linux")]
// XFS (since Linux 2.6.38), ext4 (since Linux 3.0), Btrfs (since Linux 3.7), tmpfs(5) (since Linux 3.5), gfs2(5) (since Linux 4.16) supported fs from linux documentation
fn punch_hole(file: &File, offset: u64, length: u64) -> io::Result<()> {
    use nix::fcntl::{FallocateFlags, fallocate};
    use std::os::unix::io::AsRawFd;

    let fd = file.as_raw_fd();
    let flags = FallocateFlags::FALLOC_FL_PUNCH_HOLE | FallocateFlags::FALLOC_FL_KEEP_SIZE;
    fallocate(fd, flags, offset as i64, length as i64)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
}
// APFS concluded on the basis of some github projects
#[cfg(target_os = "macos")]
fn punch_hole(file: &File, offset: u64, length: u64) -> io::Result<()> {
    use libc;
    use std::os::unix::io::AsRawFd;

    #[repr(C)]
    struct FPunchHole {
        fp_flags: u32,
        reserved: u32,
        fp_offset: i64,
        fp_length: i64,
    }

    if length == 0 {
        return Ok(());
    }

    let offset = i64::try_from(offset)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "offset exceeds i64::MAX"))?;

    let length = i64::try_from(length)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "length exceeds i64::MAX"))?;

    let mut hole = FPunchHole {
        fp_flags: 0,
        reserved: 0,
        fp_offset: offset,
        fp_length: length,
    };

    let result = unsafe { libc::fcntl(file.as_raw_fd(), 99, &mut hole) };

    if result == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

//NTFS, ReFS, SMB 3.0, CsvFS per windows documentation
#[cfg(target_os = "windows")]
fn punch_hole(file: &File, offset: u64, length: u64) -> io::Result<()> {
    use std::ffi::c_void;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use windows_sys::Win32::System::Ioctl::{FILE_ZERO_DATA_INFORMATION, FSCTL_SET_ZERO_DATA};

    let handle = file.as_raw_handle() as HANDLE;

    let mut zero_info = FILE_ZERO_DATA_INFORMATION {
        FileOffset: offset as i64,
        BeyondFinalZero: (offset + length) as i64,
    };

    let mut bytes_returned: u32 = 0;

    let result = unsafe {
        DeviceIoControl(
            handle,
            FSCTL_SET_ZERO_DATA,
            &mut zero_info as *mut _ as *mut c_void,
            std::mem::size_of::<FILE_ZERO_DATA_INFORMATION>() as u32,
            std::ptr::null_mut(),
            0,
            &mut bytes_returned,
            std::ptr::null_mut(),
        )
    };

    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn punch_hole(_file: &File, _offset: u64, _length: u64) -> io::Result<()> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn init_sparse_file(file: &File) -> io::Result<()> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use windows_sys::Win32::System::Ioctl::FSCTL_SET_SPARSE;

    let handle = file.as_raw_handle() as HANDLE;
    let mut bytes_returned: u32 = 0;

    let result = unsafe {
        DeviceIoControl(
            handle,
            FSCTL_SET_SPARSE,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            0,
            &mut bytes_returned,
            std::ptr::null_mut(),
        )
    };

    if result == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn init_sparse_file(_file: &File) -> io::Result<()> {
    Ok(())
}

const PUNCH_THRESHOLD: u64 = 50 * 1024 * 1024; // 50 MB

struct DestructiveReader {
    file: File,
    bytes_read_total: u64,
    last_punched_offset: u64,
}

impl DestructiveReader {
    fn new(file: File) -> Self {
        Self {
            file,
            bytes_read_total: 0,
            last_punched_offset: 0,
        }
    }
}

impl Read for DestructiveReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.file.read(buf)?;

        if n == 0 {
            return Ok(0);
        }

        self.bytes_read_total += n as u64;

        if self.bytes_read_total - self.last_punched_offset >= PUNCH_THRESHOLD {
            let punch_length = self.bytes_read_total - self.last_punched_offset;
            if punch_hole(&self.file, self.last_punched_offset, punch_length).is_ok() {
                self.last_punched_offset = self.bytes_read_total;
            }
        }

        Ok(n)
    }
}

pub enum AppMessage {
    Progress(String),
    Finished(Result<(), String>),
}

pub fn extract_archive(
    file_path: &Path,
    output_dir: &Path,
    tx: &Sender<AppMessage>,
) -> Result<(), String> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(file_path)
        .map_err(|e| return e.to_string() + " - while opening chosen archive")?;

    init_sparse_file(&file).map_err(|e| e.to_string())?;

    std::fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;

    let destructive = DestructiveReader::new(file);
    let callback = CallbackReader::new(destructive);

    let mut archive = ReadArchive::open_callback(callback).map_err(|e| e.to_string())?;

    while let Some(entry) = archive.next_entry().map_err(|e| e.to_string())? {
        let path_str = entry.pathname().unwrap_or_default();
        let mut relative_path = PathBuf::new();

        for component in Path::new(&path_str).components() {
            if let Component::Normal(part) = component {
                relative_path.push(part);
            }
        }

        if relative_path.as_os_str().is_empty() {
            continue;
        }

        let output_path = output_dir.join(&relative_path);

        let _ = tx.send(AppMessage::Progress(format!(
            "Extracting: {}",
            relative_path.display()
        )));

        if entry.file_type() == FileType::Directory {
            std::fs::create_dir_all(&output_path).map_err(|e| e.to_string())?;
            continue;
        }

        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let mut out_file = File::create(&output_path).map_err(|e| e.to_string())?;

        let mut buf = vec![0u8; 1024 * 1024];

        loop {
            let n = archive.read_data(&mut buf).map_err(|e| e.to_string())?;

            if n == 0 {
                break;
            }

            out_file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        }
    }

    drop(archive);

    std::fs::remove_file(file_path).map_err(|e| e.to_string())?;

    Ok(())
}
