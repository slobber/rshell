#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthKind {
    Key,
    Password,
    Agent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeySource {
    Managed,
    FileSystem,
}

impl KeySource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Managed => "managed",
            Self::FileSystem => "filesystem",
        }
    }

    pub fn parse(value: &str) -> Self {
        if value == "filesystem" {
            Self::FileSystem
        } else {
            Self::Managed
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionBackend {
    Ssh,
    LocalWsl,
    Rdp,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HostOs {
    #[default]
    Unknown,
    Linux,
    Ubuntu,
    Debian,
    Deepin,
    UnionTech,
    Kylin,
    CentOs,
    RedHat,
    Fedora,
    Arch,
    Alpine,
    OpenSuse,
    Rocky,
    AlmaLinux,
    MacOs,
    Windows,
}

impl HostOs {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Linux => "linux",
            Self::Ubuntu => "ubuntu",
            Self::Debian => "debian",
            Self::Deepin => "deepin",
            Self::UnionTech => "uniontech",
            Self::Kylin => "kylin",
            Self::CentOs => "centos",
            Self::RedHat => "redhat",
            Self::Fedora => "fedora",
            Self::Arch => "arch",
            Self::Alpine => "alpine",
            Self::OpenSuse => "opensuse",
            Self::Rocky => "rocky",
            Self::AlmaLinux => "alma",
            Self::MacOs => "macos",
            Self::Windows => "windows",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "linux" => Self::Linux,
            "ubuntu" => Self::Ubuntu,
            "debian" => Self::Debian,
            "deepin" => Self::Deepin,
            "uniontech" => Self::UnionTech,
            "kylin" => Self::Kylin,
            "centos" => Self::CentOs,
            "redhat" => Self::RedHat,
            "fedora" => Self::Fedora,
            "arch" => Self::Arch,
            "alpine" => Self::Alpine,
            "opensuse" => Self::OpenSuse,
            "rocky" => Self::Rocky,
            "alma" => Self::AlmaLinux,
            "macos" => Self::MacOs,
            "windows" => Self::Windows,
            _ => Self::Unknown,
        }
    }

    pub fn from_detection_output(output: &str) -> Self {
        let lower = output.to_ascii_lowercase();
        // Domestic distributions first: their ID_LIKE often says debian/ubuntu,
        // and the generic checks below would otherwise mask them.
        if lower.contains("deepin") {
            Self::Deepin
        } else if lower.contains("kylin") {
            Self::Kylin
        } else if lower.contains("uniontech")
            || lower.contains("union tech")
            || lower.contains("id=uos")
            || lower.contains("id=\"uos\"")
            || lower.contains("name=\"uos")
            || lower.contains("统信")
        {
            Self::UnionTech
        } else if lower.contains("ubuntu") {
            Self::Ubuntu
        } else if lower.contains("centos") {
            Self::CentOs
        } else if lower.contains("rocky") {
            Self::Rocky
        } else if lower.contains("almalinux") || lower.contains("alma linux") {
            Self::AlmaLinux
        } else if lower.contains("red hat") || lower.contains("redhat") || lower.contains("rhel") {
            Self::RedHat
        } else if lower.contains("fedora") {
            Self::Fedora
        } else if lower.contains("debian") {
            Self::Debian
        } else if lower.contains("arch linux") || lower.contains("id=arch") {
            Self::Arch
        } else if lower.contains("alpine") {
            Self::Alpine
        } else if lower.contains("opensuse")
            || lower.contains("open suse")
            || lower.contains("suse")
            || lower.contains("sles")
        {
            Self::OpenSuse
        } else if lower.contains("darwin") || lower.contains("macos") || lower.contains("mac os") {
            Self::MacOs
        } else if lower.contains("windows") {
            Self::Windows
        } else if lower.contains("linux") || lower.contains("id_like") {
            Self::Linux
        } else {
            Self::Unknown
        }
    }
}

impl ConnectionBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ssh => "ssh",
            Self::LocalWsl => "wsl",
            Self::Rdp => "rdp",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "wsl" => Self::LocalWsl,
            "rdp" => Self::Rdp,
            // PSRP was removed; keep old database records loadable as RDP.
            "psrp" => Self::Rdp,
            _ => Self::Ssh,
        }
    }
}

impl AuthKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Key => "Public key",
            Self::Password => "Password",
            Self::Agent => "SSH agent",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Key => "key",
            Self::Password => "password",
            Self::Agent => "agent",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "password" => Self::Password,
            "agent" => Self::Agent,
            _ => Self::Key,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostStatus {
    Offline,
    Connecting,
    Online,
    Error,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RdpConnectionMode {
    #[default]
    Auto,
    GraphicsPipeline,
    Compatibility,
}

impl RdpConnectionMode {
    pub fn from_db(value: Option<i64>) -> Self {
        match value {
            Some(1) => Self::GraphicsPipeline,
            Some(0) => Self::Compatibility,
            _ => Self::Auto,
        }
    }

    pub fn to_db(self) -> Option<i64> {
        match self {
            Self::Auto => None,
            Self::GraphicsPipeline => Some(1),
            Self::Compatibility => Some(0),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::GraphicsPipeline => "graphics-pipeline",
            Self::Compatibility => "compatibility",
        }
    }

    pub fn resolve(self, os: HostOs) -> Self {
        match self {
            Self::Auto if os == HostOs::Windows => Self::Compatibility,
            Self::Auto => Self::GraphicsPipeline,
            mode => mode,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Host {
    pub id: u64,
    pub folder_id: u64,
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub user: String,
    pub auth: AuthKind,
    pub key_source: KeySource,
    pub key_id: Option<u64>,
    pub secret: String,
    pub gateway: String,
    pub environment: String,
    pub keep_alive: u32,
    pub backend: ConnectionBackend,
    pub os: HostOs,
    pub wsl_distro: Option<String>,
    pub status: HostStatus,
    pub latency_ms: Option<u32>,
    pub rdp_mode: RdpConnectionMode,
    pub credentials_unavailable: bool,
}

impl Host {
    pub fn address(&self) -> String {
        match self.backend {
            ConnectionBackend::LocalWsl => {
                let distro = self
                    .wsl_distro
                    .as_deref()
                    .filter(|value| !value.is_empty())
                    .unwrap_or("default");
                format!("wsl:{distro}")
            }
            ConnectionBackend::Ssh => {
                if self.user.is_empty() {
                    format!("{}:{}", self.ip, self.port)
                } else {
                    format!("{}@{}:{}", self.user, self.ip, self.port)
                }
            }
            ConnectionBackend::Rdp => {
                if self.user.is_empty() {
                    format!("rdp:{}:{}", self.ip, self.port)
                } else {
                    format!("rdp:{}@{}:{}", self.user, self.ip, self.port)
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct HostFolder {
    pub id: u64,
    pub parent_id: Option<u64>,
    pub title: String,
    pub subtitle: String,
    pub expanded: bool,
    pub hosts: Vec<Host>,
    pub children: Vec<HostFolder>,
    pub sort_order: i64,
}

impl HostFolder {
    pub fn new(id: u64, title: impl Into<String>) -> Self {
        Self {
            id,
            parent_id: None,
            title: title.into(),
            subtitle: String::new(),
            expanded: true,
            hosts: Vec::new(),
            children: Vec::new(),
            sort_order: 0,
        }
    }

    pub fn host_count(&self) -> usize {
        self.hosts.len() + self.children.iter().map(Self::host_count).sum::<usize>()
    }

    pub fn folder_count(&self) -> usize {
        self.children.len() + self.children.iter().map(Self::folder_count).sum::<usize>()
    }
}

pub fn find_host(folders: &[HostFolder], id: u64) -> Option<&Host> {
    for folder in folders {
        if let Some(host) = folder.hosts.iter().find(|host| host.id == id) {
            return Some(host);
        }
        if let Some(host) = find_host(&folder.children, id) {
            return Some(host);
        }
    }
    None
}

pub fn find_host_mut(folders: &mut [HostFolder], id: u64) -> Option<&mut Host> {
    for folder in folders {
        if let Some(host) = folder.hosts.iter_mut().find(|host| host.id == id) {
            return Some(host);
        }
        if let Some(host) = find_host_mut(&mut folder.children, id) {
            return Some(host);
        }
    }
    None
}

pub fn find_folder(folders: &[HostFolder], id: u64) -> Option<&HostFolder> {
    for folder in folders {
        if folder.id == id {
            return Some(folder);
        }
        if let Some(found) = find_folder(&folder.children, id) {
            return Some(found);
        }
    }
    None
}

pub fn find_folder_mut(folders: &mut [HostFolder], id: u64) -> Option<&mut HostFolder> {
    for folder in folders {
        if folder.id == id {
            return Some(folder);
        }
        if let Some(found) = find_folder_mut(&mut folder.children, id) {
            return Some(found);
        }
    }
    None
}

pub fn flatten_folders(folders: &[HostFolder], prefix: &str, out: &mut Vec<(u64, String)>) {
    for folder in folders {
        let label = if prefix.is_empty() {
            folder.title.clone()
        } else {
            format!("{prefix} / {}", folder.title)
        };
        out.push((folder.id, label.clone()));
        flatten_folders(&folder.children, &label, out);
    }
}

pub fn flatten_hosts(folders: &[HostFolder], prefix: &str, out: &mut Vec<(u64, String)>) {
    for folder in folders {
        for host in &folder.hosts {
            out.push((host.id, format!("{prefix}{}", host.name)));
        }
        flatten_hosts(
            &folder.children,
            &format!("{prefix}{} / ", folder.title),
            out,
        );
    }
}

pub fn next_host_id(folders: &[HostFolder]) -> u64 {
    let mut max_id = 1000;
    fn visit(folder: &HostFolder, max_id: &mut u64) {
        *max_id = (*max_id).max(folder.id);
        for host in &folder.hosts {
            *max_id = (*max_id).max(host.id);
        }
        for child in &folder.children {
            visit(child, max_id);
        }
    }
    for folder in folders {
        visit(folder, &mut max_id);
    }
    max_id + 1
}

#[cfg(test)]
mod tests {
    use super::{HostOs, RdpConnectionMode};

    #[test]
    fn detects_common_operating_systems() {
        assert_eq!(
            HostOs::from_detection_output("ID=ubuntu\nNAME=\"Ubuntu\"\nID_LIKE=debian\n"),
            HostOs::Ubuntu
        );
        assert_eq!(
            HostOs::from_detection_output(
                "ID=\"centos\"\nNAME=\"CentOS Stream\"\nID_LIKE=\"rhel fedora\"\n"
            ),
            HostOs::CentOs
        );
        assert_eq!(
            HostOs::from_detection_output("ID=\"rhel\"\nNAME=\"Red Hat Enterprise Linux\"\n"),
            HostOs::RedHat
        );
        assert_eq!(
            HostOs::from_detection_output("Microsoft Windows [Version 10.0.26100.33648]"),
            HostOs::Windows
        );
        assert_eq!(HostOs::from_detection_output("Linux"), HostOs::Linux);
    }

    #[test]
    fn resolves_rdp_mode_from_known_os() {
        assert_eq!(
            RdpConnectionMode::Auto.resolve(HostOs::Windows),
            RdpConnectionMode::Compatibility
        );
        assert_eq!(
            RdpConnectionMode::Auto.resolve(HostOs::Linux),
            RdpConnectionMode::GraphicsPipeline
        );
        assert_eq!(
            RdpConnectionMode::Auto.resolve(HostOs::Unknown),
            RdpConnectionMode::GraphicsPipeline
        );
        assert_eq!(
            RdpConnectionMode::Compatibility.resolve(HostOs::Linux),
            RdpConnectionMode::Compatibility
        );
    }

    #[test]
    fn detects_domestic_linux_distributions_before_their_id_like_parents() {
        assert_eq!(
            HostOs::from_detection_output(
                "ID=deepin\nNAME=\"Deepin\"\nID_LIKE=debian\nPRETTY_NAME=\"Deepin 23\"\n"
            ),
            HostOs::Deepin
        );
        assert_eq!(
            HostOs::from_detection_output(
                "ID=uos\nNAME=\"UnionTech OS Desktop\"\nID_LIKE=debian\n"
            ),
            HostOs::UnionTech
        );
        assert_eq!(
            HostOs::from_detection_output(
                "ID=\"uos\"\nNAME=\"UOS\"\nPRETTY_NAME=\"UOS 20\"\nID_LIKE=debian\n"
            ),
            HostOs::UnionTech
        );
        assert_eq!(
            HostOs::from_detection_output(
                "ID=uniontech\nPRETTY_NAME=\"统信 UOS 专业版\"\nID_LIKE=debian\n"
            ),
            HostOs::UnionTech
        );
        assert_eq!(
            HostOs::from_detection_output(
                "ID=kylin\nNAME=\"Kylin\"\nID_LIKE=debian\nPRETTY_NAME=\"银河麒麟桌面操作系统 V10\"\n"
            ),
            HostOs::Kylin
        );
        assert_eq!(
            HostOs::from_detection_output("ID=openkylin\nNAME=\"OpenKylin\"\nID_LIKE=debian\n"),
            HostOs::Kylin
        );
        assert_eq!(
            HostOs::from_detection_output("ID=ubuntu\nNAME=\"UbuntuKylin\"\nID_LIKE=debian\n"),
            HostOs::Kylin
        );
    }
}
