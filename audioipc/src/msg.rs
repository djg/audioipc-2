use libc;
use std::io;
use std::mem;
use std::os::unix::io::RawFd;
use std::ptr;

// Note: The following fields must be laid out together, the OS expects them
// to be part of a single allocation.
#[repr(C)]
struct CmsgSpace {
    cmsghdr: libc::cmsghdr,
    data: libc::c_int
}

pub fn sendmsg(fd: RawFd, to_send: &[u8], fd_to_send: Option<RawFd>) -> io::Result<usize> {
    let mut msghdr: libc::msghdr = unsafe { mem::zeroed() };
    let mut iovec: libc::iovec = unsafe { mem::zeroed() };
    let mut cmsg: CmsgSpace = unsafe { mem::zeroed() };

    msghdr.msg_iov = &mut iovec as *mut _;
    msghdr.msg_iovlen = 1;
    if fd_to_send.is_some() {
        msghdr.msg_control = &mut cmsg.cmsghdr as *mut _ as *mut _;
        msghdr.msg_controllen = mem::size_of::<CmsgSpace>() as _;
    }

    iovec.iov_base = if to_send.is_empty() {
        // Empty Vecs have a non-null pointer.
        ptr::null_mut()
    } else {
        to_send.as_ptr() as *const _ as *mut _
    };
    iovec.iov_len = to_send.len();

    cmsg.cmsghdr.cmsg_len = msghdr.msg_controllen;
    cmsg.cmsghdr.cmsg_level = libc::SOL_SOCKET;
    cmsg.cmsghdr.cmsg_type = libc::SCM_RIGHTS;

    cmsg.data = fd_to_send.unwrap_or(-1);

    let result = unsafe { libc::sendmsg(fd, &msghdr, 0) };
    if result >= 0 {
        Ok(result as usize)
    } else {
        Err(io::Error::last_os_error())
    }
}

pub fn recvmsg(fd: RawFd, to_recv: &mut [u8]) -> io::Result<(usize, Option<RawFd>)> {
    let mut msghdr: libc::msghdr = unsafe { mem::zeroed() };
    let mut iovec: libc::iovec = unsafe { mem::zeroed() };
    let mut cmsg: CmsgSpace = unsafe { mem::zeroed() };

    msghdr.msg_iov = &mut iovec as *mut _;
    msghdr.msg_iovlen = 1;
    msghdr.msg_control = &mut cmsg.cmsghdr as *mut _ as *mut _;
    msghdr.msg_controllen = mem::size_of::<CmsgSpace>() as _;

    iovec.iov_base = if to_recv.is_empty() {
        // Empty Vecs have a non-null pointer.
        ptr::null_mut()
    } else {
        to_recv.as_ptr() as *const _ as *mut _
    };
    iovec.iov_len = to_recv.len();

    let result = unsafe { libc::recvmsg(fd, &mut msghdr, 0) };
    if result >= 0 {
        let fd = if msghdr.msg_controllen == mem::size_of::<CmsgSpace>() as _ &&
            cmsg.cmsghdr.cmsg_len == mem::size_of::<CmsgSpace>() as _ &&
            cmsg.cmsghdr.cmsg_level == libc::SOL_SOCKET &&
            cmsg.cmsghdr.cmsg_type == libc::SCM_RIGHTS
        {
            Some(cmsg.data)
        } else {
            None
        };

        Ok((result as usize, fd))
    } else {
        Err(io::Error::last_os_error())
    }
}
