use super::terminal::{SessionTab, TabKind};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug)]
pub struct Pane {
    pub id: u64,
    pub tabs: Vec<SessionTab>,
    pub active: usize,
}

impl Pane {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            tabs: Vec::new(),
            active: 0,
        }
    }

    pub fn active_tab(&self) -> Option<&SessionTab> {
        self.tabs.get(self.active)
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut SessionTab> {
        self.tabs.get_mut(self.active)
    }

    pub fn add_tab(&mut self, tab: SessionTab) {
        self.tabs.push(tab);
        self.active = self.tabs.len().saturating_sub(1);
    }
}

#[derive(Clone, Debug)]
pub enum SplitNode {
    Leaf(Pane),
    Split {
        axis: Axis,
        ratio: f32,
        first: Box<SplitNode>,
        second: Box<SplitNode>,
    },
}

impl SplitNode {
    pub fn tab_count(&self) -> usize {
        match self {
            Self::Leaf(pane) => pane.tabs.len(),
            Self::Split { first, second, .. } => first.tab_count() + second.tab_count(),
        }
    }

    pub fn terminal_tab_ids(&self) -> Vec<u64> {
        match self {
            Self::Leaf(pane) => pane
                .tabs
                .iter()
                .filter(|tab| tab.kind == TabKind::Terminal)
                .map(|tab| tab.id)
                .collect(),
            Self::Split { first, second, .. } => {
                let mut ids = first.terminal_tab_ids();
                ids.extend(second.terminal_tab_ids());
                ids
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropZone {
    Center,
    Left,
    Right,
    Top,
    Bottom,
}

impl DropZone {
    pub fn axis(self) -> Option<Axis> {
        match self {
            Self::Left | Self::Right => Some(Axis::Horizontal),
            Self::Top | Self::Bottom => Some(Axis::Vertical),
            Self::Center => None,
        }
    }

    pub fn new_first(self) -> bool {
        matches!(self, Self::Left | Self::Top)
    }
}

#[derive(Clone, Debug)]
pub struct Workspace {
    pub root: SplitNode,
    pub focused_pane: u64,
    pub next_pane_id: u64,
}

impl Workspace {
    pub fn new() -> Self {
        Self {
            root: SplitNode::Leaf(Pane::new(1)),
            focused_pane: 1,
            next_pane_id: 2,
        }
    }

    pub fn pane(&self, id: u64) -> Option<&Pane> {
        find_pane(&self.root, id)
    }

    pub fn pane_mut(&mut self, id: u64) -> Option<&mut Pane> {
        find_pane_mut(&mut self.root, id)
    }

    pub fn focused(&self) -> Option<&Pane> {
        self.pane(self.focused_pane)
    }

    pub fn focused_mut(&mut self) -> Option<&mut Pane> {
        self.pane_mut(self.focused_pane)
    }

    pub fn active_tab(&self) -> Option<&SessionTab> {
        self.focused().and_then(Pane::active_tab)
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut SessionTab> {
        self.focused_mut().and_then(Pane::active_tab_mut)
    }

    pub fn allocate_pane_id(&mut self) -> u64 {
        let id = self.next_pane_id;
        self.next_pane_id += 1;
        id
    }

    pub fn split_with_tab(&mut self, target_pane: u64, zone: DropZone, tab: SessionTab) -> u64 {
        let axis = zone.axis().unwrap_or(Axis::Horizontal);
        let new_pane_id = self.allocate_pane_id();
        let mut new_pane = Pane::new(new_pane_id);
        new_pane.add_tab(tab);
        if split_leaf_node(&mut self.root, target_pane, axis, zone, &mut Some(new_pane)) {
            new_pane_id
        } else {
            target_pane
        }
    }

    pub fn add_tab_to_pane(&mut self, pane_id: u64, tab: SessionTab) -> bool {
        if let Some(pane) = self.pane_mut(pane_id) {
            pane.add_tab(tab);
            self.focused_pane = pane_id;
            true
        } else {
            false
        }
    }

    pub fn remove_tab(&mut self, tab_id: u64) -> Option<SessionTab> {
        fn remove(node: &mut SplitNode, tab_id: u64, removed: &mut Option<SessionTab>) {
            match node {
                SplitNode::Leaf(pane) => {
                    if let Some(index) = pane.tabs.iter().position(|tab| tab.id == tab_id) {
                        *removed = Some(pane.tabs.remove(index));
                        if pane.active >= pane.tabs.len() {
                            pane.active = pane.tabs.len().saturating_sub(1);
                        }
                    }
                }
                SplitNode::Split { first, second, .. } => {
                    remove(first, tab_id, removed);
                    if removed.is_none() {
                        remove(second, tab_id, removed);
                    }
                }
            }
        }
        let mut removed = None;
        remove(&mut self.root, tab_id, &mut removed);
        self.prune_empty_leaves();
        removed
    }

    pub fn move_tab(&mut self, tab_id: u64, target_pane: u64, zone: DropZone) {
        let Some(source_pane) = self.pane_for_tab(tab_id) else {
            return;
        };
        let Some(tab) = self.find_tab(tab_id).cloned() else {
            return;
        };
        if zone == DropZone::Center {
            if source_pane == target_pane {
                if let Some(pane) = self.pane_mut(target_pane) {
                    pane.active = pane
                        .tabs
                        .iter()
                        .position(|existing| existing.id == tab_id)
                        .unwrap_or(0);
                }
                self.focused_pane = target_pane;
                return;
            }
            self.remove_tab_from_pane(source_pane, tab_id);
            if self.add_tab_to_pane(target_pane, tab) {
                self.focused_pane = target_pane;
            }
            return;
        }

        let new_pane = self.split_with_tab(target_pane, zone, tab);
        self.remove_tab_from_pane(source_pane, tab_id);
        self.focused_pane = new_pane;
    }

    fn remove_tab_from_pane(&mut self, pane_id: u64, tab_id: u64) -> Option<SessionTab> {
        let removed = self.pane_mut(pane_id).and_then(|pane| {
            let index = pane.tabs.iter().position(|tab| tab.id == tab_id)?;
            let removed = pane.tabs.remove(index);
            if pane.active >= pane.tabs.len() {
                pane.active = pane.tabs.len().saturating_sub(1);
            }
            Some(removed)
        });
        self.prune_empty_leaves();
        removed
    }

    pub fn find_tab(&self, tab_id: u64) -> Option<&SessionTab> {
        fn visit(node: &SplitNode, tab_id: u64) -> Option<&SessionTab> {
            match node {
                SplitNode::Leaf(pane) => pane.tabs.iter().find(|tab| tab.id == tab_id),
                SplitNode::Split { first, second, .. } => {
                    visit(first, tab_id).or_else(|| visit(second, tab_id))
                }
            }
        }
        visit(&self.root, tab_id)
    }

    pub fn find_tab_mut(&mut self, tab_id: u64) -> Option<&mut SessionTab> {
        fn visit(node: &mut SplitNode, tab_id: u64) -> Option<&mut SessionTab> {
            match node {
                SplitNode::Leaf(pane) => pane.tabs.iter_mut().find(|tab| tab.id == tab_id),
                SplitNode::Split { first, second, .. } => {
                    visit(first, tab_id).or_else(|| visit(second, tab_id))
                }
            }
        }
        visit(&mut self.root, tab_id)
    }

    pub fn find_tab_for_sftp_session_mut(
        &mut self,
        session_id: u64,
    ) -> Option<(&mut SessionTab, bool)> {
        fn visit(node: &mut SplitNode, session_id: u64) -> Option<(&mut SessionTab, bool)> {
            match node {
                SplitNode::Leaf(pane) => {
                    for tab in &mut pane.tabs {
                        if tab.id == session_id {
                            return Some((tab, false));
                        }
                        if tab.left_session_id == Some(session_id) {
                            return Some((tab, true));
                        }
                    }
                    None
                }
                SplitNode::Split { first, second, .. } => {
                    visit(first, session_id).or_else(|| visit(second, session_id))
                }
            }
        }
        visit(&mut self.root, session_id)
    }

    pub fn pane_for_tab(&self, tab_id: u64) -> Option<u64> {
        fn visit(node: &SplitNode, tab_id: u64) -> Option<u64> {
            match node {
                SplitNode::Leaf(pane) => pane
                    .tabs
                    .iter()
                    .any(|tab| tab.id == tab_id)
                    .then_some(pane.id),
                SplitNode::Split { first, second, .. } => {
                    visit(first, tab_id).or_else(|| visit(second, tab_id))
                }
            }
        }
        visit(&self.root, tab_id)
    }

    pub fn split_active(&mut self, axis: Axis, duplicate: Option<SessionTab>) -> u64 {
        let pane_id = self.focused_pane;
        let zone = match axis {
            Axis::Horizontal => DropZone::Right,
            Axis::Vertical => DropZone::Bottom,
        };
        if let Some(tab) = duplicate {
            self.split_with_tab(pane_id, zone, tab)
        } else {
            let new_pane_id = self.allocate_pane_id();
            let _ = split_leaf_node(
                &mut self.root,
                pane_id,
                axis,
                match axis {
                    Axis::Horizontal => DropZone::Right,
                    Axis::Vertical => DropZone::Bottom,
                },
                &mut Some(Pane::new(new_pane_id)),
            );
            self.focused_pane = new_pane_id;
            new_pane_id
        }
    }

    pub fn prune_empty_leaves(&mut self) {
        fn prune(node: SplitNode) -> SplitNode {
            match node {
                SplitNode::Leaf(pane) => SplitNode::Leaf(pane),
                SplitNode::Split {
                    axis,
                    ratio,
                    first,
                    second,
                } => {
                    let first = prune(*first);
                    let second = prune(*second);
                    match (&first, &second) {
                        (SplitNode::Leaf(pane), _) if pane.tabs.is_empty() => second,
                        (_, SplitNode::Leaf(pane)) if pane.tabs.is_empty() => first,
                        _ => SplitNode::Split {
                            axis,
                            ratio,
                            first: Box::new(first),
                            second: Box::new(second),
                        },
                    }
                }
            }
        }
        let old = std::mem::replace(&mut self.root, SplitNode::Leaf(Pane::new(0)));
        self.root = prune(old);
        if self.pane(self.focused_pane).is_none() {
            self.focused_pane = find_first_pane_id(&self.root).unwrap_or(1);
        }
    }

    pub fn tab_count(&self) -> usize {
        self.root.tab_count()
    }

    pub fn tab_count_by_kind(&self, kind: TabKind) -> usize {
        fn count(node: &SplitNode, kind: TabKind) -> usize {
            match node {
                SplitNode::Leaf(pane) => pane.tabs.iter().filter(|tab| tab.kind == kind).count(),
                SplitNode::Split { first, second, .. } => count(first, kind) + count(second, kind),
            }
        }
        count(&self.root, kind)
    }

    pub fn all_tabs(&self) -> Vec<&SessionTab> {
        fn visit<'a>(node: &'a SplitNode, output: &mut Vec<&'a SessionTab>) {
            match node {
                SplitNode::Leaf(pane) => output.extend(pane.tabs.iter()),
                SplitNode::Split { first, second, .. } => {
                    visit(first, output);
                    visit(second, output);
                }
            }
        }
        let mut tabs = Vec::new();
        visit(&self.root, &mut tabs);
        tabs
    }
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}

pub fn find_pane(node: &SplitNode, id: u64) -> Option<&Pane> {
    match node {
        SplitNode::Leaf(pane) => (pane.id == id).then_some(pane),
        SplitNode::Split { first, second, .. } => {
            find_pane(first, id).or_else(|| find_pane(second, id))
        }
    }
}

pub fn find_pane_mut(node: &mut SplitNode, id: u64) -> Option<&mut Pane> {
    match node {
        SplitNode::Leaf(pane) => (pane.id == id).then_some(pane),
        SplitNode::Split { first, second, .. } => {
            if find_pane(first, id).is_some() {
                find_pane_mut(first, id)
            } else {
                find_pane_mut(second, id)
            }
        }
    }
}

fn find_first_pane_id(node: &SplitNode) -> Option<u64> {
    match node {
        SplitNode::Leaf(pane) => Some(pane.id),
        SplitNode::Split { first, .. } => find_first_pane_id(first),
    }
}

fn split_leaf_node(
    node: &mut SplitNode,
    pane_id: u64,
    axis: Axis,
    zone: DropZone,
    new_pane: &mut Option<Pane>,
) -> bool {
    match node {
        SplitNode::Leaf(pane) if pane.id == pane_id => {
            let old = SplitNode::Leaf(pane.clone());
            let Some(new_pane) = new_pane.take() else {
                return false;
            };
            let new_leaf = SplitNode::Leaf(new_pane);
            let (first, second) = if zone.new_first() {
                (new_leaf, old)
            } else {
                (old, new_leaf)
            };
            *node = SplitNode::Split {
                axis,
                ratio: 0.5,
                first: Box::new(first),
                second: Box::new(second),
            };
            true
        }
        SplitNode::Leaf(_) => false,
        SplitNode::Split { first, second, .. } => {
            if split_leaf_node(first, pane_id, axis, zone, new_pane) {
                true
            } else {
                split_leaf_node(second, pane_id, axis, zone, new_pane)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        find_host, AuthKind, ConnectionBackend, Host, HostFolder, HostStatus, KeySource,
    };

    fn host() -> Host {
        Host {
            id: 1,
            folder_id: 1,
            name: "test".to_owned(),
            ip: "127.0.0.1".to_owned(),
            port: 22,
            user: "root".to_owned(),
            auth: AuthKind::Key,
            key_source: KeySource::FileSystem,
            key_id: None,
            secret: "~/.ssh/id".to_owned(),
            gateway: "direct".to_owned(),
            environment: "TEST".to_owned(),
            keep_alive: 30,
            backend: ConnectionBackend::Ssh,
            os: crate::domain::HostOs::Unknown,
            wsl_distro: None,
            status: HostStatus::Offline,
            latency_ms: None,
            rdp_mode: Default::default(),
            credentials_unavailable: false,
        }
    }

    #[test]
    fn dropping_tab_on_right_edge_creates_sibling_pane() {
        let mut workspace = Workspace::new();
        let host = host();
        workspace
            .focused_mut()
            .unwrap()
            .add_tab(SessionTab::terminal(1, &host));
        workspace
            .focused_mut()
            .unwrap()
            .add_tab(SessionTab::terminal(2, &host));

        workspace.move_tab(1, 1, DropZone::Right);

        match &workspace.root {
            SplitNode::Split {
                axis,
                first,
                second,
                ..
            } => {
                assert_eq!(*axis, Axis::Horizontal);
                assert_eq!(first.tab_count(), 1);
                assert_eq!(second.tab_count(), 1);
            }
            _ => panic!("expected split workspace"),
        }
        assert_eq!(workspace.tab_count(), 2);
    }

    #[test]
    fn removing_last_tab_collapses_empty_leaf() {
        let mut workspace = Workspace::new();
        let host = host();
        workspace
            .focused_mut()
            .unwrap()
            .add_tab(SessionTab::terminal(1, &host));
        workspace.move_tab(1, 1, DropZone::Bottom);
        workspace.remove_tab(1);
        assert!(matches!(workspace.root, SplitNode::Leaf(_)));
        assert_eq!(workspace.tab_count(), 0);
    }

    #[test]
    fn moving_tab_from_later_pane_to_earlier_pane_succeeds() {
        let mut workspace = Workspace::new();
        let host = host();
        workspace
            .focused_mut()
            .unwrap()
            .add_tab(SessionTab::terminal(1, &host));
        workspace
            .focused_mut()
            .unwrap()
            .add_tab(SessionTab::terminal(2, &host));
        workspace.move_tab(2, 1, DropZone::Bottom);

        workspace.move_tab(2, 1, DropZone::Right);

        assert!(workspace.find_tab(1).is_some());
        assert!(workspace.find_tab(2).is_some());
        assert_eq!(workspace.tab_count(), 2);
        assert_ne!(
            workspace.pane_for_tab(1),
            workspace.pane_for_tab(2),
            "the dragged tab must end up in a different pane"
        );
    }

    #[test]
    fn finds_sftp_tabs_by_destination_or_left_source_session() {
        let mut workspace = Workspace::new();
        let host = host();
        let mut tab = SessionTab::sftp(7, &host);
        tab.left_session_id = Some(1_000_001);
        workspace.focused_mut().unwrap().add_tab(tab);

        let (tab, left) = workspace
            .find_tab_for_sftp_session_mut(7)
            .expect("destination session");
        assert!(!left);
        assert_eq!(tab.id, 7);

        let (tab, left) = workspace
            .find_tab_for_sftp_session_mut(1_000_001)
            .expect("left source session");
        assert!(left);
        assert_eq!(tab.id, 7);
    }

    #[test]
    fn folder_tree_is_recursive() {
        let mut root = HostFolder::new(1, "root");
        let mut child = HostFolder::new(2, "child");
        child.hosts.push(host());
        root.children.push(child);
        assert_eq!(root.host_count(), 1);
        assert_eq!(root.folder_count(), 1);
        assert!(find_host(&[root], 1).is_some());
    }
}
