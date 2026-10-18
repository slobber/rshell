# RShell 堡垒机会话设计

- 状态：已冻结，待开发
- 冻结日期：2026-10-18
- 适用版本：RShell 0.1.x
- 相关代码：`src/ssh/mod.rs`、`src/app/sessions.rs`、`src/app/ssh_events.rs`、`src/domain/host.rs`、`src/storage/`、`src/ui/dialogs/host.rs`

## 1. 背景

深信服、奇安信、新华三等堡垒机通常不向 SSH 客户端暴露目标机的 shell 或 SFTP
子系统。用户先登录堡垒机，拿到的是一个运行在 PTY 中的菜单程序，然后在菜单里
选择资产、输入目标用户名和目标密码，最后才进入目标机的 shell。

从 SSH 协议角度看：

- 到目标机只有一条 SSH 连接，端点是堡垒机。
- 资产选择、目标用户名、目标密码都是同一个 PTY 里的 TUI 交互。
- 目标机的 SSH 连接由堡垒机内部建立，RShell 看不到目标机的主机密钥。
- 目标机的 SFTP 通常不可用；文件传输要么使用堡垒机自己的菜单，要么使用独立的
  文件网关或 Web 接口。

因此这类堡垒机不是传统意义上的 ProxyJump。只做端口转发或嵌套 SSH 连接无法覆盖
菜单交互，必须把“登录后的 TUI”作为一等会话流程处理。

## 2. 目标与非目标

### 2.1 目标

- 把堡垒机后面的目标资产保存为 RShell 主机树中的一等主机记录。
- 支持半自动登录：自动选择资产、输入目标用户名、输入目标密码。
- 任何一步无法识别时暂停，保留终端并允许用户手工接管。
- 支持堡垒机登录阶段的 keyboard-interactive 二次认证（OTP、短信、令牌）。
- 当目标 shell 真正就绪后再把资产标记为 Online。
- 明确处理 SFTP：经堡垒机的目标资产不提供 SFTP 入口。
- 为多个厂商保留统一的驱动接口；v1 只正式实现深信服。

### 2.2 非目标

- 不做 Web 门户自动化，也不调用堡垒机厂商 API 同步资产。
- 不做堡垒机专用 SFTP 网关或 Web 文件传输协议。
- 不做一条 SSH 连接内复用多个目标会话。
- 不做嵌套堡垒机（堡垒机再经另一台堡垒机）。
- 不在 v1 正式实现奇安信和新华三驱动。
- 不做断线自动重连。

## 3. 用户流程

### 3.1 配置堡垒机

1. 新建一条 SSH 主机，填写堡垒机地址、端口、登录账号和认证方式。
2. 打开“编辑连接 → 高级”，把“堡垒机类型”设为“深信服”。
3. 保存。这条主机既可以单独连接，也可以被目标资产引用。

`通用（手工）` 类型只打开堡垒机终端，不运行自动登录。

### 3.2 配置目标资产

1. 新建一条 SSH 主机，`IP` 填写目标资产地址。
2. 打开“编辑连接 → 高级”，启用“经堡垒机”。
3. 从主机树中选择一条已设置类型的堡垒机。
4. 填写“资产选择值”。留空时默认使用目标资产的 `IP`。
5. 在基本页填写目标用户名和目标密码。目标密码允许留空。
6. 可选填写“登录成功提示”；留空时使用内置 shell 提示符判断。

### 3.3 连接

1. 打开目标资产的终端 Tab。
2. RShell 连接堡垒机并完成 SSH 认证。
3. 如果堡垒机要求 keyboard-interactive，Tab 顶部出现内联认证条。
4. 进入堡垒机菜单后，驱动自动执行：
   - 选择资产；
   - 输入目标用户名；
   - 输入目标密码；
   - 等待目标 shell。
5. 目标 shell 就绪后，资产标记为 Online。
6. 任一步无法识别、超时或出现错误提示时，自动登录暂停，终端仍可输入。
7. 用户处理完 OTP、确认或其他提示后，点“继续自动登录”从下一步恢复。

## 4. 数据模型

### 4.1 Host 字段

在现有 `Host` 上增加：

```rust
pub enum BastionKind {
    Disabled,
    Manual,
    Sangfor,
    QiAnXin,
    H3c,
}

pub struct Host {
    // 现有字段省略
    pub bastion_kind: BastionKind,
    pub bastion_id: Option<u64>,
    pub asset_selector: String,
    pub ready_pattern: String,
}
```

字段含义：

| 字段 | 含义 |
| --- | --- |
| `bastion_kind` | 这条主机作为堡垒机时使用哪种驱动；`Disabled` 表示不是堡垒机 |
| `bastion_id` | 目标资产引用的堡垒机主机 ID；`None` 表示直连 |
| `asset_selector` | 菜单中输入的资产值，例如 IP、编号、名称关键字 |
| `ready_pattern` | 可选的登录成功正则；空字符串表示使用内置判断 |

### 4.2 目标地址与连接端点

走堡垒机时：

- 目标资产的 `ip` 仍然是目标地址，用于主机树展示、搜索和资产选择值默认值。
- 目标资产的 `port` 仅作记录，不参与连接。UI 在“经堡垒机”模式下隐藏或标注为
  “仅记录”。
- 实际 TCP 端点是 `bastion.ip:bastion.port`。
- SSH 主机密钥 TOFU 使用堡垒机的 `ip:port`，不使用目标资产的 `ip:port`。
- Tab 的 `host_id` 始终是目标资产 ID。
- 会话计数、断开主机、标签标题都按目标资产处理。
- 延迟只表示到堡垒机的 SSH 往返时间，不表示到目标机的网络延迟。

### 4.3 认证语义

- 堡垒机记录的 `user` / `secret` / `auth` 用于登录堡垒机。
- 目标资产的 `user` / `secret` 是驱动要输入到 TUI 的目标账号和目标密码。
- 经堡垒机时目标认证固定为 `AuthKind::Password`，UI 隐藏 Key 和 Agent。
- 目标密码允许为空。为空时驱动在密码提示处暂停，由用户手动输入。
- `credentials_unavailable` 对经堡垒机的目标资产不能因为目标密码为空而置位；
  真正需要检查的是堡垒机记录的凭据是否可用。

### 4.4 不变量

保存和加载时必须满足：

1. `bastion_id` 必须指向存在的主机。
2. 被引用的主机 `bastion_kind != Disabled`。
3. `bastion_id != host.id`。
4. 有 `bastion_id` 的主机不能再设置非 `Disabled` 的 `bastion_kind`，禁止嵌套。
5. 只有 `ConnectionBackend::Ssh` 的主机可以设置 `bastion_id`。
6. 经堡垒机的目标资产必须填写 `ip` 和目标用户名。
7. `ready_pattern` 非空时必须是合法正则。

### 4.5 SQLite 迁移

沿用 `Persistence::ensure_column`，增加：

```sql
ALTER TABLE hosts ADD COLUMN bastion_kind TEXT NOT NULL DEFAULT 'disabled';
ALTER TABLE hosts ADD COLUMN bastion_id INTEGER;
ALTER TABLE hosts ADD COLUMN asset_selector TEXT NOT NULL DEFAULT '';
ALTER TABLE hosts ADD COLUMN ready_pattern TEXT NOT NULL DEFAULT '';
```

建议把 `PRAGMA user_version` 提升到 5。

不为 `bastion_id` 增加数据库外键。当前 `save_host_tree` 会先删除全部主机再重新
插入，自引用外键会让保存顺序和清理逻辑变复杂；引用完整性在应用层统一校验。

`bastion_kind`、`bastion_id`、`asset_selector`、`ready_pattern` 均为非敏感字段，
可以明文保存。目标密码仍使用现有的 ChaCha20-Poly1305 字段加密。

## 5. 连接链路

### 5.1 端点拆分

worker 不能再默认“传入的 Host 就是 TCP 端点”。建议引入明确的连接计划：

```rust
pub struct BastionContext {
    pub asset_selector: String,
    pub target_user: String,
    pub target_password: String,
    pub ready_pattern: Option<String>,
}

pub struct SessionPlan {
    /// 主机树中展示的目标资产。
    pub target: Host,
    /// 实际建立 SSH 连接的端点：直连时等于 target，经堡垒机时是堡垒机。
    pub transport: Host,
    /// 经堡垒机时的 TUI 上下文。
    pub bastion: Option<BastionContext>,
}
```

`SshManager::open_terminal` 接受 `SessionPlan`。`open_sftp` 只接受直连主机。

### 5.2 直连

保持现有行为：

1. 解析目标主机认证。
2. 连接 `target.ip:target.port`。
3. 校验主机密钥。
4. 认证。
5. 打开 PTY shell。
6. 立即发送 `Connected`，随后发送 `SessionReady`。

### 5.3 经堡垒机

1. 从 `bastion_id` 解析堡垒机主机。
2. 校验堡垒机存在、类型非 `Disabled`、不是目标自身。
3. 用堡垒机的地址、端口、账号和认证方式解析 SSH 认证材料。
4. 连接 `bastion.ip:bastion.port`，按堡垒机地址做 TOFU。
5. 完成 SSH 认证；需要时进入 keyboard-interactive。
6. 打开 PTY shell。
7. 创建厂商驱动，进入自动登录状态机。
8. 驱动完成后发送 `SessionReady`，应用层才把目标资产标记为 Online。

### 5.4 事件与命令

新增事件：

```rust
pub enum SshEvent {
    // 现有事件省略
    AuthPrompt {
        tab_id: u64,
        request_id: u64,
        name: String,
        instructions: String,
        prompts: Vec<AuthPromptField>,
    },
    BastionState {
        tab_id: u64,
        state: BastionSessionState,
    },
    SessionReady {
        tab_id: u64,
        host_id: u64,
    },
}
```

`AuthPromptField` 包含 prompt 文本、是否回显、是否密码输入。

新增命令：

```rust
pub enum SshCommand {
    // 现有命令省略
    AuthResponse {
        request_id: u64,
        responses: Vec<String>,
    },
    BastionResume,
    BastionCancel,
}
```

`request_id` 用于丢弃过期响应。关闭 Tab 或断开连接时，等待中的认证和驱动都应
尽快退出。

### 5.5 状态语义

- `Connected`：SSH 传输已建立，记录堡垒机延迟；不直接把目标标记为 Online。
- `BastionState`：驱动进度，用于 Tab 顶部横幅。
- `SessionReady`：目标 shell 已就绪；应用层把目标资产标记为 Online。
- 暂停自动登录时目标保持 `Connecting`，由横幅说明停在哪一步。
- 手工模式在堡垒机 shell 打开后发送 `SessionReady`，横幅显示“手工模式”。

## 6. 驱动接口

### 6.1 Trait

驱动是编译期内置的增量状态机，不是动态插件：

```rust
pub trait BastionDriver: Send {
    fn kind(&self) -> BastionKind;

    fn next_action(
        &mut self,
        screen: &ScreenView<'_>,
        ctx: &BastionContext,
    ) -> DriverAction;
}

pub enum DriverAction {
    Send { bytes: Vec<u8>, next: BastionPhase },
    Wait,
    Ready,
    Pause { phase: BastionPhase, reason: String },
    Manual { reason: String },
}
```

`ScreenView` 包装 `vt100::Screen`，至少提供：

- `tail_lines(n)`：取屏幕底部若干行；
- `last_non_empty_line()`；
- `cursor()`；
- `contains_in_tail(pattern)`。

驱动只看可见屏幕，不直接解析原始字节。这样可以避开 ANSI 转义序列和分片问题。

### 6.2 阶段

```rust
pub enum BastionPhase {
    WaitingForAsset,
    WaitingForUsername,
    WaitingForPassword,
    WaitingForShell,
    Ready,
    Paused,
    Manual,
    Cancelled,
}
```

### 6.3 状态机规则

1. 顺序推进：资产选择 → 目标用户名 → 目标密码 → 目标 shell。
2. 每一步只发送一次；发送后记录步骤。
3. 出现错误词、超时、屏幕退回已发送步骤时暂停。
4. 密码错误绝不自动重试，避免堡垒机锁号。
5. 用户手动输入时立即暂停自动发送，避免抢输入。
6. “继续自动登录”只允许推进到下一步，不重发当前步骤。
7. “取消自动登录”关闭驱动，终端继续可用。
8. 空目标密码时，在密码提示处暂停并提示用户手动输入。
9. 如果菜单要求“按回车继续”，驱动可以发送一个回车，但必须把它作为独立步骤
   记录，不能反复发送。

### 6.4 屏幕匹配

- 只匹配屏幕底部 8 行。
- 匹配前压缩连续空白，忽略大小写。
- 输出停止约 150ms 后再判断，避免半屏匹配。
- 每步超时 8 秒；超时进入暂停。
- 错误词优先于步骤匹配。
- 准备就绪时只检查最后一条非空行。

### 6.5 v1 深信服驱动

由于当前没有真实环境样本，v1 的提示词先使用中英文常见词，并在 UI 中标记为
“未验证”。暂停横幅提供“复制当前屏幕”，便于拿到环境后校准。

资产提示候选：

- `请选择`
- `选择资产`
- `资产编号`
- `资产列表`
- `请输入资产`
- `select asset`
- `asset id`
- `asset name`

用户名提示候选：

- `用户名`
- `账号`
- `登录名`
- `username`
- `login as`
- `login:`

密码提示候选：

- `密码`
- `口令`
- `password`

错误提示候选：

- `密码错误`
- `认证失败`
- `用户名或密码`
- `无效`
- `锁定`
- `拒绝`
- `权限不足`
- `denied`
- `invalid`
- `failed`
- `locked`

登录成功判断：

1. 如果目标资产配置了 `ready_pattern`，使用该正则匹配最后一条非空行。
2. 否则匹配最后一条非空行的 `#`、`$` 或 `>` 提示符。
3. 8 秒内没有匹配则暂停，而不是假定成功。

v1 假定资产选择值可以通过“输入文本 + 回车”提交。如果实际菜单只能用方向键，
驱动会停在资产提示；用户可以手动选择，看到用户名提示后点“继续自动登录”。

### 6.6 其他驱动

- `Manual`：不自动发送任何内容；堡垒机 shell 打开后直接进入手工模式。
- `QiAnXin`、`H3c`：类型可以保存，但当前映射到手工模式，并在 UI 提示
  “暂未支持自动登录”。等有测试环境后补充驱动，不需要改数据库。

## 7. Keyboard-interactive 认证

### 7.1 流程

1. 先尝试主机配置的密码、私钥或 Agent 认证。
2. 如果服务器仍要求 keyboard-interactive，启动 KBI。
3. 服务器返回一组 prompt，worker 发送 `AuthPrompt`。
4. UI 在对应 Tab 顶部显示内联认证条。
5. 用户提交后，应用发送 `AuthResponse`。
6. worker 回复服务器；如果还有下一轮，继续发送新的 `AuthPrompt`。
7. 成功则继续打开 PTY；取消或超时则关闭连接。

### 7.2 自动填充规则

- prompt 同时包含 OTP 关键词时，一律让用户输入，不能自动填密码。
- OTP 关键词包括：`otp`、`verification code`、`动态码`、`短信`、`令牌`、
  `验证码`、`二次认证`、`mfa`、`token`。
- prompt 明确包含 `password` 或 `密码`，且没有 OTP 关键词时，可以使用堡垒机
  已保存的密码自动填充。
- 无法识别的 prompt 一律交给用户，不做猜测。
- 多字段 prompt 一次展示多个输入框；密码字段使用遮挡输入。
- 响应内容不写日志。

### 7.3 取消与超时

- 用户关闭 Tab 或点击取消时，worker 放弃认证并断开。
- KBI 等待用户输入的超时建议为 120 秒；超时后连接失败，并给出明确提示。
- 同一个 `request_id` 只能响应一次；过期响应直接丢弃。

## 8. UI

### 8.1 编辑连接

主机编辑对话框从单页改为“基本 / 高级”两个页签。

堡垒机主机的高级页：

- 堡垒机类型下拉：
  - 关闭
  - 通用（手工）
  - 深信服
  - 奇安信（暂未支持）
  - 新华三（暂未支持）
- 一句说明：类型表示“这条主机可以作为堡垒机被目标资产引用”。
- 地址、端口、登录账号、密码仍在基本页，不在高级页重复。

目标资产的高级页：

- “经堡垒机”开关。
- 堡垒机选择器：只列出 `bastion_kind != Disabled` 的 SSH 主机。
- 资产选择值输入框，占位提示“留空使用目标 IP”。
- 登录成功提示输入框，占位提示“留空使用内置 shell 提示符判断”。

经堡垒机时：

- 基本页隐藏 Key 和 Agent，只保留 Password。
- 目标端口字段隐藏或标注“仅记录”。
- 目标密码可以留空，旁边提示“留空则在密码提示处暂停”。
- SSH 的旧 `gateway` 字段不再显示；该字段只保留给 RDP domain 使用。

### 8.2 Tab 顶部横幅

自动登录期间在终端上方显示一条窄横幅，包含：

- 当前阶段文案，例如“正在等待资产列表”。
- 暂停原因，例如“未匹配到密码提示，已暂停”。
- 主按钮：继续自动登录。
- 次按钮：取消自动登录。
- 暂停时额外提供：复制当前屏幕。

横幅不能遮挡终端输入，也不能阻止用户直接在终端里打字。

### 8.3 侧栏与主机树

- 经堡垒机的目标资产显示一个小链接图标，或 host 名称后的 `via <bastion>`。
- 属性面板显示“堡垒机”和“资产选择值”，不显示堡垒机密码。
- 堡垒机主机本身仍可像普通 SSH 主机一样打开终端。

### 8.4 SFTP

经堡垒机的目标资产：

- 主机树右键的“打开 SFTP”置灰，tooltip 说明原因。
- 快捷键和命令面板打开 SFTP 时显示 toast：
  “堡垒机会话不支持 SFTP；可在终端中使用堡垒机自带的文件传输菜单。”
- 不允许创建空 SFTP Tab 占位。
- 左右栏选择远端主机时，也不能把经堡垒机的目标作为 SFTP 源。

堡垒机主机自身直接打开的 SFTP 不拦；它只是一条普通 SSH 连接，失败按普通 SFTP
错误处理。

### 8.5 i18n

新增文案必须同时更新 `src/i18n.rs` 的中英文分支，至少包括：

- 堡垒机类型与占位说明；
- 经堡垒机、资产选择值、登录成功提示；
- 自动登录阶段与暂停原因；
- 继续、取消、复制屏幕；
- SFTP 不支持提示；
- 删除被引用堡垒机时的阻止提示。

## 9. 引用完整性

### 9.1 删除堡垒机

- 删除仍被目标资产引用的堡垒机：阻止。
- 提示中列出引用数量，例如“有 3 个资产正在使用此堡垒机”。
- 用户必须先迁移或解除这些资产的引用。

### 9.2 删除文件夹

当前删除文件夹会级联删除内部主机。规则：

- 如果被删子树中的堡垒机被子树外的资产引用，阻止删除。
- 如果堡垒机和所有引用它的目标资产都在同一个被删子树内，允许一起删除。
- 删除前仍要显示现有的主机和子文件夹数量警告。

### 9.3 修改类型

- 把堡垒机类型改为“通用（手工）”：允许，但提示引用它的资产下次连接只会打开
  终端，不再自动登录。
- 把类型改为“关闭”：如果仍被引用，阻止；先解除引用。
- 选择奇安信或新华三：允许保存，但提示当前按手工模式运行。

### 9.4 移动与重命名

- 堡垒机和目标资产可以自由移动、重命名，引用按 ID 保持不变。
- 禁止把目标资产的堡垒机改成它自己。
- 禁止把一条已经有 `bastion_id` 的主机设置为堡垒机类型。

## 10. 安全

- 目标密码和堡垒机密码继续使用现有的字段加密。
- 自动登录驱动发送的密码不进入日志、不进入事件、不写入调试输出。
- “复制当前屏幕”只复制用户可见的终端文本，不复制历史回看和输入缓存。
- KBI 响应不写日志；密码字段使用遮挡输入。
- TOFU 按堡垒机端点记录，避免把目标地址误当成可信主机。
- 主机树、属性面板和 toast 不展示任何密码。

## 11. 测试计划

### 11.1 驱动单元测试

用 `vt100::Parser` 构造屏幕，覆盖：

- 资产提示 → 发送选择值；
- 用户名提示 → 发送目标用户名；
- 密码提示 → 发送目标密码；
- shell 提示 → Ready；
- 空密码 → 在密码步骤暂停；
- 错误提示 → 暂停且不重试；
- 发送后屏幕退回上一阶段 → 暂停；
- 暂停后继续只推进下一步；
- 手动输入后驱动不再发送；
- 超时 → 暂停；
- 未识别的屏幕 → 暂停并保留原因。

### 11.2 端到端测试

用现有 `russh::server` 测试设施构造一个假堡垒机：

1. 接受密码认证。
2. 打开 PTY shell。
3. 打印资产列表提示，读取选择值。
4. 打印用户名提示，读取用户名。
5. 打印密码提示，读取密码。
6. 打印一个 shell 提示符。

断言：

- 驱动按顺序发送正确的值；
- 收到 `SessionReady` 后目标才 Online；
- 密码错误时不重试；
- 关闭 Tab 会终止 worker。

### 11.3 KBI 测试

假服务器要求两轮 keyboard-interactive：

- 第一轮 `Password:`，验证自动填充；
- 第二轮 `Verification code:`，验证 UI 收到 `AuthPrompt` 并回传响应；
- 取消和超时路径。

### 11.4 存储与完整性测试

- 旧数据库自动增加新列，旧主机按直连加载。
- 保存/加载 `bastion_kind`、`bastion_id`、`asset_selector`、`ready_pattern`。
- 自引用、嵌套、悬空引用、非法正则被拒绝。
- 删除被引用的堡垒机被阻止。
- 删除包含完整引用闭包的文件夹被允许。

### 11.5 SFTP 守卫测试

- 经堡垒机的主机不能创建 SFTP Tab。
- 不能作为 SFTP 左栏或右栏的远端主机。
- 堡垒机主机自身仍可尝试 SFTP。

## 12. 实现顺序

1. 领域模型、SQLite 迁移、引用校验与测试。
2. `BastionDriver` trait、屏幕视图、深信服状态机与单元测试。
3. worker 端点拆分、`SessionPlan`、事件/命令接线。
4. keyboard-interactive 认证与内联认证条。
5. 编辑连接“基本 / 高级”页签与目标资产字段。
6. Tab 顶部横幅、继续/取消/复制屏幕。
7. SFTP 守卫、删除/类型修改完整性规则。
8. 假堡垒机端到端测试。
9. 有真实环境后校准深信服提示词，移除“未验证”标记。

## 13. 风险与未决事项

- **深信服提示词未验证**：当前没有真实菜单样本，驱动可能无法一次命中。
  这是 v1 最大的风险；暂停横幅和“复制屏幕”是主要的校准手段。
- **菜单可能不支持文本选择**：如果资产列表只能用方向键，v1 会停在资产步骤，
  需要用户手工选择。后续可以增加按键序列类型。
- **目标 shell 提示符不固定**：网络设备可能使用 `]`、`<` 或自定义提示符。
  用户可以配置“登录成功提示”正则。
- **KBI 文案差异**：OTP 关键词表可能需要按厂商补充。
- **每个 Tab 独立认证**：多开资产会重复 OTP；v1 明确接受这个成本。
- **SFTP 不可用**：只能使用堡垒机自带菜单或后续独立文件网关。
- **奇安信/新华三未实现**：类型占位，当前按手工模式运行。

## 14. 决策记录

| # | 决策 | 选择 |
| --- | --- | --- |
| 1 | 第一版支持层级 | 半自动资产条目（B） |
| 2 | 堡垒机配置归属 | 复用主机树中的 SSH 主机记录 |
| 3 | 驱动接口粒度 | 增量状态机（B） |
| 4 | v1 正式实现厂商 | 只实现深信服；奇安信、新华三占位 |
| 5 | 打断处理 | 暂停 + 显式继续（B） |
| 6 | 目标地址语义 | `ip` 保持目标资产地址，端点用堡垒机 |
| 7 | 真实菜单样本 | 暂无，先按常见提示写并标记未验证（C） |
| 8 | 二次认证 UI | Tab 顶部内联认证条 |
| 9 | 连接复用 | 每个目标 Tab 一条独立 SSH 连接 |
| 10 | SFTP | 经堡垒机的目标入口置灰 |
| 11 | Online 时机 | 目标 shell 就绪后 |
| 12 | 目标认证 | 固定 Password，密码可空 |
| 13 | 引用完整性 | 阻止删除被引用的堡垒机，禁止嵌套 |
