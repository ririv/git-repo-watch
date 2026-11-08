//! Shared RGBA terminal surface with bounded, background-prepared image tiles.
mod probe;
mod worker;
use anyhow::Result;
use engine::cancel::Cancel;
use graph::pixels::Metrics;
use image::RgbaImage;
pub(crate) use probe::{Backend, Support};
use ratatui::{Frame, layout::Rect};
use ratatui_image::{Image, protocol::Protocol};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    time::Instant,
};
use worker::{Job, Worker};

const CAPACITY: usize = 256;
const PIXEL_BUDGET: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Source {
    Graph(u64),
    Preview(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Key {
    pub source: Source,
    pub index: usize,
    pub pan: usize,
    pub columns: u16,
    pub rows: u16,
    pub metrics: Metrics,
}
impl Key {
    fn bytes(self) -> u128 {
        u128::from(self.columns)
            * u128::from(self.rows)
            * u128::from(self.metrics.width.get())
            * u128::from(self.metrics.height.get())
            * 4
    }
}

enum Tile {
    Loading(Cancel),
    Ready(Protocol),
    Failed(String),
}
struct Entry {
    id: u32,
    used: u64,
    tile: Tile,
}
pub(crate) enum Draw {
    Ready,
    Loading,
    Failed(String),
}

pub(crate) struct Display {
    support: Support,
    cache: BTreeMap<Key, Entry>,
    pinned: BTreeSet<Key>,
    retired: Vec<u32>,
    worker: Worker,
    active: bool,
    clock: u64,
    next_id: u32,
    spinner: crate::spinner::Spinner,
}
impl Display {
    pub fn new(support: Support) -> Self {
        Self {
            support,
            cache: BTreeMap::new(),
            pinned: BTreeSet::new(),
            retired: Vec::new(),
            worker: Worker::new(),
            active: false,
            clock: 0,
            next_id: std::process::id().wrapping_mul(2_654_435_761).max(1),
            spinner: crate::spinner::Spinner::new(Instant::now()),
        }
    }
    pub fn detect() -> (Self, crate::appearance::Background) {
        let (support, background) = probe::detect();
        (Self::new(support), background)
    }
    pub fn metrics(&self) -> Result<Metrics, String> {
        match &self.support {
            Support::Pixels(backend) => Ok(backend.metrics),
            Support::Unavailable(reason) => Err(reason.clone()),
        }
    }
    pub fn refresh_metrics(&mut self) {
        if let (Support::Pixels(backend), Some(metrics)) =
            (&mut self.support, probe::window_metrics())
            && backend.metrics != metrics
        {
            backend.metrics = metrics;
            self.clear();
        }
    }
    pub fn begin_frame(&mut self) {
        self.active = false;
        self.pinned.clear();
    }
    pub fn pin(&mut self, keys: impl IntoIterator<Item = Key>) {
        self.active = true;
        self.pinned.extend(keys);
    }
    fn remove(&mut self, key: Key) {
        if let Some(entry) = self.cache.remove(&key) {
            if let Tile::Loading(cancel) = entry.tile {
                cancel.cancel();
            }
            self.retired.push(entry.id);
        }
    }
    fn clear(&mut self) {
        for key in self.cache.keys().copied().collect::<Vec<_>>() {
            self.remove(key);
        }
    }
    pub fn request(
        &mut self,
        key: Key,
        prepare: impl FnOnce() -> Result<RgbaImage> + Send + 'static,
    ) {
        self.clock += 1;
        if let Some(entry) = self.cache.get_mut(&key) {
            entry.used = self.clock;
            return;
        }
        let Support::Pixels(backend) = &self.support else {
            return;
        };
        let backend = backend.clone();
        while self.cache.len() >= CAPACITY
            || self.cache.keys().map(|key| key.bytes()).sum::<u128>() + key.bytes()
                > PIXEL_BUDGET as u128
        {
            let oldest = self
                .cache
                .iter()
                .filter(|(key, _)| !self.pinned.contains(key))
                .min_by_key(|(_, entry)| entry.used)
                .map(|(key, _)| *key);
            let Some(oldest) = oldest else {
                return;
            };
            self.remove(oldest);
        }
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        let cancel = Cancel::default();
        self.cache.insert(
            key,
            Entry {
                id,
                used: self.clock,
                tile: Tile::Loading(cancel.clone()),
            },
        );
        self.worker.push(Job {
            key,
            id,
            cancel,
            backend,
            language: i18n::language(),
            prepare: Box::new(prepare),
        });
    }
    pub fn render(&mut self, frame: &mut Frame, key: Key, area: Rect) -> Draw {
        match self.cache.get(&key).map(|entry| &entry.tile) {
            Some(Tile::Ready(image)) => {
                frame.render_widget(Image::new(image), area);
                Draw::Ready
            }
            Some(Tile::Failed(error)) => Draw::Failed(error.clone()),
            Some(Tile::Loading(_)) => Draw::Loading,
            None => Draw::Failed(i18n::tr!(GraphicsTooLarge).into()),
        }
    }
    pub fn symbol(&self) -> &'static str {
        self.spinner.symbol()
    }
    #[cfg(test)]
    pub fn settle(&mut self) {
        let deadline = Instant::now() + std::time::Duration::from_secs(5);
        while self
            .cache
            .values()
            .any(|entry| matches!(entry.tile, Tile::Loading(_)))
        {
            assert!(Instant::now() < deadline, "image worker did not finish");
            self.poll();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
    pub fn poll(&mut self) -> bool {
        let mut dirty = false;
        let responses = match self.worker.responses() {
            Ok(responses) => responses,
            Err(error) => {
                for entry in self.cache.values_mut() {
                    if let Tile::Loading(cancel) = &entry.tile {
                        cancel.cancel();
                        entry.tile = Tile::Failed(error.to_string());
                        dirty = true;
                    }
                }
                Vec::new()
            }
        };
        for response in responses {
            if let Some(entry) = self.cache.get_mut(&response.key)
                && entry.id == response.id
            {
                entry.tile = match response.result {
                    Ok(protocol) => Tile::Ready(protocol),
                    Err(error) => Tile::Failed(error.to_string()),
                };
                dirty = true;
            }
        }
        dirty
            | (self.active
                && self
                    .cache
                    .values()
                    .any(|entry| matches!(entry.tile, Tile::Loading(_)))
                && self.spinner.tick(Instant::now()))
    }
    pub fn finish_frame(&mut self, output: &mut impl Write) -> std::io::Result<()> {
        if !self.active {
            self.clear();
        }
        if let Support::Pixels(backend) = &self.support {
            backend.delete(output, &self.retired)?;
        }
        self.retired.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests;
