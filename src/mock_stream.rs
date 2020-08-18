/// Contains tooling for mocking various Iron objects.

use hyper::net::NetworkStream;
use std::any::Any;
use std::io::{Read, Write, Result};
use std::net::SocketAddr;
use std::time::Duration;

/// A mock network stream
#[derive(Clone)]
pub struct MockStream<T> {
    peer_addr: SocketAddr,
    data: T
}

impl<T> MockStream<T> {
    /// Create a new mock stream that originates from the given peer address and reads from the
    /// given data
    pub fn new(peer_addr: SocketAddr, data: T) -> MockStream<T> {
        MockStream { peer_addr, data }
    }
}

impl<T: Send + Read + Write + Clone + Any> NetworkStream for MockStream<T> {
    fn peer_addr(&mut self) -> Result<SocketAddr> {
        Ok(self.peer_addr)
    }

    fn set_read_timeout(&self, _: Option<Duration>) -> Result<()> {
        Ok(())
    }

    fn set_write_timeout(&self, _: Option<Duration>) -> Result<()> {
        Ok(())
    }
}

impl<T: Read> Read for MockStream<T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.data.read(buf)
    }
}

impl<T: Write> Write for MockStream<T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.data.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        self.data.flush()
    }
}
