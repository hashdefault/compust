use anyhow::{Context, Result, ensure};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use std::{
    io::{Read, Write},
    net::{Shutdown, TcpListener},
    os::unix::net::UnixStream,
    thread::{self, JoinHandle},
    time::Duration,
};
use x11rb::{
    errors::ParseError,
    protocol::xproto::SetupRequest,
    x11_utils::{BigRequests, RequestHeader, TryParse, parse_request_header},
};

pub(crate) struct Proxy {
    stop: UnixStream,
    worker: Option<JoinHandle<Result<()>>>,
}

impl Proxy {
    pub(crate) fn start(
        display: &str,
        intercept: impl FnMut(RequestHeader, &mut [u8]) -> Result<()> + Send + 'static,
    ) -> Result<(String, Self)> {
        let number = display
            .strip_prefix(':')
            .context("expected local Xvfb display")?;
        let server = UnixStream::connect(format!("/tmp/.X11-unix/X{number}"))?;
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let display_number = listener
            .local_addr()?
            .port()
            .checked_sub(6000)
            .context("proxy port below X11 display range")?;
        let (stop, signal) = UnixStream::pair()?;
        let relay = Relay {
            listener,
            server,
            stop: signal,
            intercept,
            pending: Vec::new(),
            setup: true,
        };
        let worker = thread::spawn(move || relay.run());
        Ok((
            format!("127.0.0.1:{display_number}"),
            Self {
                stop,
                worker: Some(worker),
            },
        ))
    }

    pub(crate) fn finish(&mut self) -> Result<()> {
        if let Some(worker) = self.worker.take() {
            let signal = self.stop.shutdown(Shutdown::Both);
            let result = worker
                .join()
                .map_err(|_| anyhow::anyhow!("X11 proxy worker panicked"))?;
            signal?;
            result?;
        }
        Ok(())
    }
}

impl Drop for Proxy {
    fn drop(&mut self) {
        if let Err(error) = self.finish() {
            eprintln!("X11 proxy cleanup: {error:#}");
        }
    }
}

struct Relay<F> {
    listener: TcpListener,
    server: UnixStream,
    stop: UnixStream,
    intercept: F,
    pending: Vec<u8>,
    setup: bool,
}

impl<F: FnMut(RequestHeader, &mut [u8]) -> Result<()>> Relay<F> {
    fn run(mut self) -> Result<()> {
        let timeout = Timespec::try_from(Duration::from_secs(10))?;
        let mut fds = [
            PollFd::new(&self.stop, PollFlags::IN),
            PollFd::new(&self.listener, PollFlags::IN),
        ];
        ensure!(
            poll(&mut fds, Some(&timeout))? > 0,
            "compositor did not connect to proxy"
        );
        let [stop, _] = fds;
        if !stop.revents().is_empty() {
            return Ok(());
        }
        let (mut client, _) = self.listener.accept()?;
        client.set_nodelay(true)?;
        client.set_write_timeout(Some(Duration::from_secs(5)))?;
        self.server
            .set_write_timeout(Some(Duration::from_secs(5)))?;
        let mut bytes = [0; 16384];
        loop {
            let ready = {
                let mut fds = [
                    PollFd::new(&self.stop, PollFlags::IN),
                    PollFd::new(&client, PollFlags::IN),
                    PollFd::new(&self.server, PollFlags::IN),
                ];
                ensure!(poll(&mut fds, Some(&timeout))? > 0, "X11 proxy stalled");
                fds.map(|fd| !fd.revents().is_empty())
            };
            let [stop, from_client, from_server] = ready;
            if stop {
                return Ok(());
            }
            if from_client {
                let count = client.read(&mut bytes)?;
                if count == 0 {
                    return Ok(());
                }
                self.pending
                    .extend_from_slice(bytes.get(..count).context("invalid client read length")?);
                self.forward()?;
            }
            if from_server {
                let count = self.server.read(&mut bytes)?;
                if count == 0 {
                    return Ok(());
                }
                client.write_all(bytes.get(..count).context("invalid server read length")?)?;
            }
        }
    }

    fn forward(&mut self) -> Result<()> {
        loop {
            let length = if self.setup {
                let (setup, remaining) = match SetupRequest::try_parse(&self.pending) {
                    Ok(value) => value,
                    Err(ParseError::InsufficientData) => return Ok(()),
                    Err(error) => return Err(error.into()),
                };
                let byte_order = if cfg!(target_endian = "little") {
                    b'l'
                } else {
                    b'B'
                };
                ensure!(
                    setup.byte_order == byte_order,
                    "proxy requires native-endian client"
                );
                self.setup = false;
                self.pending.len() - remaining.len()
            } else {
                let (header, remaining) =
                    match parse_request_header(&self.pending, BigRequests::Enabled) {
                        Ok(value) => value,
                        Err(ParseError::InsufficientData) => return Ok(()),
                        Err(error) => return Err(error.into()),
                    };
                let body_length = usize::try_from(header.remaining_length)?
                    .checked_mul(4)
                    .context("X11 request length overflow")?;
                let header_length = self.pending.len() - remaining.len();
                let length = header_length + body_length;
                let Some(body) = self.pending.get_mut(header_length..length) else {
                    return Ok(());
                };
                (self.intercept)(header, body)?;
                length
            };
            self.server.write_all(
                self.pending
                    .get(..length)
                    .context("incomplete X11 request")?,
            )?;
            self.pending.drain(..length);
        }
    }
}
