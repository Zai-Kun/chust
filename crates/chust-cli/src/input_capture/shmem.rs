use std::num::NonZeroUsize;
use std::os::fd::OwnedFd;

use nix::fcntl::OFlag;
use nix::sys::mman::{self, MapFlags, ProtFlags, shm_unlink};
use nix::sys::stat::Mode;
use nix::unistd::ftruncate;

const FLINK_NAME: &str = "/shm_please_dont_exist";

pub unsafe fn create_shmem(size: usize) -> (OwnedFd, *mut u8) {
    let shm_fd = mman::shm_open(
        FLINK_NAME,
        OFlag::O_RDWR | OFlag::O_CREAT,
        Mode::from_bits(0o666).unwrap(),
    )
    .unwrap();
    shm_unlink(FLINK_NAME).unwrap();

    ftruncate(&shm_fd, size as i64).unwrap();

    let ptr: *mut u8 = unsafe {
        mman::mmap(
            None,
            NonZeroUsize::new(size).unwrap(),
            ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            MapFlags::MAP_SHARED,
            &shm_fd,
            0,
        )
        .unwrap()
        .as_ptr() as *mut u8
    };

    return (shm_fd, ptr);
}
