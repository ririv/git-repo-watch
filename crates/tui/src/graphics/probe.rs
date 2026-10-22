//! Capability probing finishes before Crossterm starts reading terminal input.
use anyhow::Result;
use graph::pixels::Metrics;
use image::{DynamicImage, RgbaImage};
use ratatui::layout::Size;
use ratatui_image::{
    picker::cap_parser::{Parser, QueryStdioOptions, Response},
    protocol::{Protocol, iterm2::Iterm2, kitty::Kitty, sixel::Sixel},
};
use std::{
    io::{self, Write},
    num::NonZeroU16,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy)]
enum Transport {
    Direct,
    Tmux,
}
impl Transport {
    fn tmux(self) -> bool {
        matches!(self, Self::Tmux)
    }
}
#[derive(Debug, Clone, Copy)]
enum Wire {
    Kitty { compression: bool },
    Iterm2,
    Sixel,
}
#[derive(Debug, Clone)]
pub(crate) struct Backend {
    pub metrics: Metrics,
    wire: Wire,
    transport: Transport,
}
pub(crate) enum Support {
    Pixels(Backend),
    Unavailable(String),
}
impl Backend {
    pub fn prepare(&self, image: RgbaImage, id: u32) -> Result<Protocol> {
        let size = Size::new(
            (image.width() / u32::from(self.metrics.width.get())).try_into()?,
            (image.height() / u32::from(self.metrics.height.get())).try_into()?,
        );
        let image = DynamicImage::ImageRgba8(image);
        Ok(match self.wire {
            Wire::Kitty { compression } => Protocol::Kitty(Kitty::new(
                image,
                size,
                id,
                self.transport.tmux(),
                compression,
            )?),
            Wire::Iterm2 => Protocol::ITerm2(Iterm2::new(image, size, self.transport.tmux())?),
            Wire::Sixel => Protocol::Sixel(Sixel::new(image, size, self.transport.tmux())?),
        })
    }
    pub fn delete(&self, output: &mut impl Write, ids: &[u32]) -> io::Result<()> {
        if matches!(self.wire, Wire::Kitty { .. }) {
            let (start, escape, end) = Parser::tmux_start_escape_end(self.transport.tmux());
            for id in ids {
                write!(output, "{start}{escape}_Ga=d,d=I,i={id},q=2{escape}\\{end}")?;
            }
            output.flush()?;
        }
        Ok(())
    }
    #[cfg(test)]
    pub fn test() -> Self {
        Self {
            metrics: Metrics {
                width: NonZeroU16::new(10).unwrap(),
                height: NonZeroU16::new(20).unwrap(),
            },
            wire: Wire::Kitty { compression: true },
            transport: Transport::Direct,
        }
    }
}
pub(super) fn window_metrics() -> Option<Metrics> {
    let size = crossterm::terminal::window_size().ok()?;
    Some(Metrics {
        width: NonZeroU16::new(size.width.checked_div(size.columns)?)?,
        height: NonZeroU16::new(size.height.checked_div(size.rows)?)?,
    })
}
fn interpret(responses: &[Response], transport: Transport) -> Result<Backend> {
    let metrics = responses
        .iter()
        .find_map(|response| {
            if let Response::CellSize(Some((w, h))) = response {
                Some(Metrics {
                    width: NonZeroU16::new(*w)?,
                    height: NonZeroU16::new(*h)?,
                })
            } else {
                None
            }
        })
        .or_else(window_metrics)
        .ok_or_else(|| anyhow::anyhow!(i18n::tr!(GraphicsMetricsUnavailable)))?;
    let wire = if responses.contains(&Response::Kitty) {
        Wire::Kitty {
            compression: responses.contains(&Response::KittyCompression),
        }
    } else if responses.contains(&Response::Sixel) {
        Wire::Sixel
    } else if std::env::var("TERM_PROGRAM")
        .is_ok_and(|value| matches!(value.as_str(), "iTerm.app" | "WezTerm" | "rio"))
    {
        Wire::Iterm2
    } else {
        anyhow::bail!(i18n::tr!(GraphicsUnsupported));
    };
    Ok(Backend {
        metrics,
        wire,
        transport,
    })
}
pub(super) fn detect() -> Support {
    let transport = if std::env::var_os("TMUX").is_some() {
        Transport::Tmux
    } else {
        Transport::Direct
    };
    match query(transport).and_then(|responses| interpret(&responses, transport)) {
        Ok(backend) => Support::Pixels(backend),
        Err(error) => Support::Unavailable(error.to_string()),
    }
}
#[cfg(unix)]
fn query(transport: Transport) -> Result<Vec<Response>> {
    let query = Parser::query(
        transport.tmux(),
        QueryStdioOptions {
            timeout: Duration::from_millis(400),
            kitty_compression: true,
            ..QueryStdioOptions::default()
        },
    );
    io::stdout().write_all(query.as_bytes())?;
    io::stdout().flush()?;
    let deadline = Instant::now() + Duration::from_millis(400);
    let mut parser = Parser::new();
    let mut responses = Vec::new();
    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        let mut fd = libc::pollfd {
            fd: libc::STDIN_FILENO,
            events: libc::POLLIN,
            revents: 0,
        };
        // 在有期限的 poll 之后读取，避免无响应终端留下持续抢占 stdin 的后台线程。
        let ready = unsafe {
            libc::poll(
                &mut fd,
                1,
                remaining.as_millis().min(i32::MAX as u128) as i32,
            )
        };
        if ready == 0 {
            break;
        }
        if ready < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error.into());
        }
        let mut bytes = [0u8; 256];
        let count =
            unsafe { libc::read(libc::STDIN_FILENO, bytes.as_mut_ptr().cast(), bytes.len()) };
        if count <= 0 {
            anyhow::bail!(i18n::tr!(GraphicsUnsupported));
        }
        for byte in &bytes[..count as usize] {
            for response in parser.push(char::from(*byte)) {
                if response == Response::Status {
                    return Ok(responses);
                }
                responses.push(response);
            }
        }
    }
    Ok(responses)
}
#[cfg(not(unix))]
fn query(_: Transport) -> Result<Vec<Response>> {
    anyhow::bail!(i18n::tr!(GraphicsUnsupported))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn kitty_probe_requires_reported_pixel_metrics_and_keeps_compression_capability() {
        let backend = interpret(
            &[
                Response::Kitty,
                Response::KittyCompression,
                Response::CellSize(Some((18, 36))),
            ],
            Transport::Direct,
        )
        .unwrap();
        assert_eq!(backend.metrics.width.get(), 18);
        assert!(matches!(backend.wire, Wire::Kitty { compression: true }));
        let mut output = Vec::new();
        backend.delete(&mut output, &[42]).unwrap();
        assert_eq!(output, b"\x1b_Ga=d,d=I,i=42,q=2\x1b\\");
    }
}
