//! Background readers apply only matching file and search requests.
use super::*;
impl Session {
    pub fn refresh(&mut self) {
        let navigation = self.navigation.path().map(Path::to_owned);
        self.save_position();
        self.directory_reader.clear();
        self.queued.clear();
        let mut visible = BTreeSet::from([PathBuf::new()]);
        for row in self.rows().into_iter().filter(|row| row.entry.directory()) {
            visible.extend(
                row.entry
                    .path
                    .ancestors()
                    .take_while(|path| path.starts_with(row.head))
                    .map(Path::to_owned),
            );
        }
        for (path, value) in &mut self.directories {
            if visible.contains(path) {
                *value = match std::mem::replace(value, Contents::Unloaded) {
                    Contents::Ready(directory) | Contents::Refreshing(directory) => {
                        Contents::Refreshing(directory)
                    }
                    _ => Contents::Loading,
                };
                self.queued.push_back(path.clone());
            } else {
                *value = Contents::Unloaded;
            }
        }
        self.panes.invalidate();
        if !self.query.is_empty() {
            self.search_reader.clear();
            self.search = Search::Loading(self.query.clone());
        }
        self.sync_preview();
        if let Some(path) = navigation {
            self.navigation = location::Navigation::Pending(path);
        }
    }
    pub fn pending(&self) -> bool {
        matches!(self.navigation, location::Navigation::Pending(_))
            || !self.queued.is_empty()
            || self.directory_reader.pending()
            || self.panes.pending()
            || self.search_reader.pending()
            || matches!(self.search, Search::Loading(_))
    }
    pub fn animate(&mut self) -> bool {
        self.pending() && self.animation.tick(Instant::now())
    }
    pub fn spinner(&self) -> &str {
        self.animation.symbol()
    }
    pub fn notice(&self) -> Option<&str> {
        if let location::Navigation::Failed { error, .. } = &self.navigation {
            return Some(error);
        }
        match &self.search {
            Search::Loading(_) => Some(i18n::tr!(BrowserSearching)),
            Search::Failed(error) => Some(error),
            Search::Ready {
                extent: SearchExtent::Limited,
                ..
            } => Some(i18n::tr!(BrowserSearchLimited)),
            _ => match self.directories.get(Path::new("")) {
                Some(Contents::Loading) => Some(i18n::tr!(BrowserLoading)),
                Some(Contents::Failed(error)) => Some(error),
                _ => None,
            },
        }
    }
    pub fn directory_notice(&self, path: &Path) -> Option<&str> {
        match self.directories.get(path) {
            Some(Contents::Loading | Contents::Refreshing(_)) => Some(self.spinner()),
            Some(Contents::Failed(error)) => Some(error),
            _ => None,
        }
    }
    pub fn directory_loading(&self, path: &Path) -> bool {
        matches!(
            self.directories.get(path),
            Some(Contents::Loading | Contents::Refreshing(_))
        )
    }
    pub fn poll(&mut self, service: &Arc<Service>) -> bool {
        let selected = self.selected_row().map(|row| row.head.to_owned());
        let mut changed = false;
        if let Some((path, result)) = self.directory_reader.poll() {
            changed = true;
            let value = match result {
                Ok(directory) => {
                    for entry in &directory.entries {
                        if entry.directory() {
                            self.directories
                                .entry(entry.path.clone())
                                .or_insert(Contents::Unloaded);
                        }
                    }
                    Contents::Ready(directory)
                }
                Err(error) => Contents::Failed(error),
            };
            self.directories.insert(path, value);
        }
        if changed {
            self.probe_visible();
        }
        if !self.directory_reader.pending()
            && let Some(path) = self.queued.pop_front()
        {
            let service = service.clone();
            let root = self.root.clone();
            let relative = path.clone();
            self.directory_reader.request(path, move |cancel| {
                service.browse_directory(&root, &relative, &cancel)
            });
            changed = true;
        }
        if let Some((query, result)) = self.search_reader.poll()
            && self.query == query
        {
            self.search = match result {
                Ok(found) => Search::Ready {
                    query,
                    entries: found.entries,
                    extent: found.extent,
                },
                Err(error) => Search::Failed(error),
            };
            changed = true;
        }
        if let Search::Loading(query) = &self.search
            && self.search_reader.requested() != Some(query)
        {
            let service = service.clone();
            let root = self.root.clone();
            let text = query.clone();
            self.search_reader.request(query.clone(), move |cancel| {
                service.find_files(&root, &text, &cancel)
            });
            changed = true;
        }
        if changed && matches!(self.navigation, location::Navigation::Idle) {
            let rows = self.rows();
            let index = selected
                .and_then(|path| {
                    rows.iter().position(|row| row.head == path).or_else(|| {
                        rows.iter().position(|row| {
                            path.starts_with(row.head) && row.entry.path.starts_with(&path)
                        })
                    })
                })
                .unwrap_or(self.state.selected().unwrap_or(0));
            self.select(index);
        }
        changed |= self.settle_navigation();
        changed |= self.panes.poll(service, &self.root);
        changed
    }
}
