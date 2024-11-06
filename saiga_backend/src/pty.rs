use std::{
    ffi::CString,
    io,
    os::fd::{AsFd, AsRawFd, OwnedFd, RawFd},
};

pub use nix::Result;
use nix::{pty::ForkptyResult, unistd::Pid};

pub struct Pty {
    master: OwnedFd,
    child: Pid,
}

impl io::Read for Pty {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let result = nix::unistd::read(self.master.as_raw_fd(), buf);

        match result {
            Ok(count) => Ok(count),
            Err(e) if e == nix::errno::Errno::EAGAIN => Ok(0),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        }
    }
}

impl io::Write for Pty {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let result = nix::unistd::write(self.master.as_fd(), buf);

        result.map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    fn flush(&mut self) -> io::Result<()> {
        todo!()
    }
}

impl Pty {
    pub fn try_new() -> Result<Pty> {
        let result = unsafe { nix::pty::forkpty(None, None)? };

        let pty = match result {
            ForkptyResult::Parent { child, master } => {
                set_nonblocking_mode(master.as_raw_fd());

                Pty { master, child }
            }
            ForkptyResult::Child => {
                // TODO: change shell
                nix::unistd::execvp::<CString>(c"fish", &[])?;

                std::process::exit(0);
            }
        };

        Ok(pty)
    }
}

fn set_nonblocking_mode(fd: RawFd) {
    let flags = nix::fcntl::fcntl(fd, nix::fcntl::F_GETFL).unwrap();
    let mut flags = nix::fcntl::OFlag::from_bits(flags).expect("must be valid flags");
    flags.set(nix::fcntl::OFlag::O_NONBLOCK, true);

    nix::fcntl::fcntl(fd, nix::fcntl::F_SETFL(flags)).unwrap();
}
