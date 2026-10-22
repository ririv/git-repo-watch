//! Incremental lane routing for child-before-parent DAG streams, independent of Git and UI.
pub mod pixels;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Track {
    pub column: usize,
    pub color: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub color: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub node: Track,
    pub incoming: Vec<Track>,
    pub outgoing: Vec<Edge>,
    pub width: usize,
}

#[derive(Debug, Clone)]
struct Lane<Id> {
    target: Id,
    color: usize,
}

#[derive(Debug, Clone)]
pub struct Layout<Id> {
    lanes: Vec<Lane<Id>>,
    next_color: usize,
}

impl<Id> Default for Layout<Id> {
    fn default() -> Self {
        Self {
            lanes: Vec::new(),
            next_color: 0,
        }
    }
}

impl<Id: Clone + Eq> Layout<Id> {
    /// Append one topologically ordered node, preserving lanes across page boundaries.
    pub fn push(&mut self, id: &Id, parents: &[Id]) -> Row {
        let column = match self.lanes.iter().position(|lane| &lane.target == id) {
            Some(column) => column,
            None => {
                let color = self.allocate_color();
                self.lanes.push(Lane {
                    target: id.clone(),
                    color,
                });
                self.lanes.len() - 1
            }
        };
        let before = self.lanes.clone();
        let node = Track {
            column,
            color: self.lanes.remove(column).color,
        };
        let mut insertion = column.min(self.lanes.len());
        for (index, parent) in parents.iter().enumerate() {
            if self.lanes.iter().any(|lane| &lane.target == parent) {
                continue;
            }
            let color = if index == 0 {
                node.color
            } else {
                self.allocate_color()
            };
            self.lanes.insert(
                insertion,
                Lane {
                    target: parent.clone(),
                    color,
                },
            );
            insertion += 1;
        }
        let mut outgoing = Vec::new();
        for (from, lane) in before.iter().enumerate() {
            if from != column {
                let to = self
                    .lanes
                    .iter()
                    .position(|next| next.target == lane.target)
                    .expect("continuing lane remains active");
                outgoing.push(Edge {
                    from,
                    to,
                    color: lane.color,
                });
            }
        }
        for parent in parents {
            let to = self
                .lanes
                .iter()
                .position(|lane| &lane.target == parent)
                .expect("parent lane was inserted or already active");
            let edge = Edge {
                from: column,
                to,
                color: self.lanes[to].color,
            };
            if !outgoing.contains(&edge) {
                outgoing.push(edge);
            }
        }
        Row {
            node,
            incoming: before
                .iter()
                .enumerate()
                .map(|(column, lane)| Track {
                    column,
                    color: lane.color,
                })
                .collect(),
            outgoing,
            width: before.len().max(self.lanes.len()) * 2 - 1,
        }
    }

    fn allocate_color(&mut self) -> usize {
        let color = self.next_color;
        self.next_color += 1;
        color
    }
}
