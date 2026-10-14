<div align="center">

# wx-cli

**从命令行查询本地微信数据**

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey.svg)](#安装)
[![Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg)](https://www.rust-lang.org)

会话 · 聊天记录 · 搜索 · 联系人 · 群成员 · 群昵称 · 收藏 · 统计 · 导出 · 明文缓存

</div>

---

## AI Agent Skill

通过 [skills CLI](https://github.com/vercel-labs/skills) 一键安装到 Claude Code、Cursor、Codex 等 agent：

```bash
npx skills add hooligan520/wx-cli-again
```

或全局安装：

```bash
npx skills add hooligan520/wx-cli-again -g
```

安装后 agent 会自动读取仓库里的 `SKILL.md`，了解如何安装和调用 wx-cli。
（跟随上游的话把仓库名换成 `jackwener/wx-cli-again`。）

---

## 关于本仓库（fork）

本仓库是 **[jackwener/wx-cli-again](https://github.com/jackwener/wx-cli-again)** 的 fork：
`origin` = 本仓库，`upstream` = jackwener/wx-cli-again（仅用于同步上游）。

> 上游 `botiverse/wx-cli` 已不存在（历史 README 里的安装链接已失效），
> 现在可直接 clone **上游或本 fork** 的源码构建，不需要仓库权限。

相对上游，本 fork 只加了 6 个 commit，全部围绕**长跨度/大数据量下的可用性**：

| commit | 内容 |
|---|---|
| `ded34a6` | daemon 启动超时 `15s → 180s`。首次要解密 ~20 个 DB，15s 必然超时 |
| `93e75ee` | 单请求 IPC 读超时 `120s → 600s`。`FullDecrypt` 路径本身就要 ~120s，与 120s 读超时**正面撞车**，症状是命令**恰好卡在 120.0s** 退出、`rc=1`、stdout 为空。可用 `WX_CLI_TIMEOUT_SECS` 覆盖 |
| `b2604d3` | npm 包支持**随包原生二进制**（`bin/native/wx`），并新增 `npm/pack-local.sh` 一键打包，让 `npm install -g` 能真正装上 fork 的二进制 |
| `0983381` | `wx init` 认 `config.keys_file`（不再硬编码 `<配置目录>/all_keys.json`）；新增 **`WX_CLI_CONFIG`** 环境变量钉死配置路径 |
| `345befb` | **新增 `wx cache list` / `wx cache pull`** —— 把任意分片物化成明文，供外部工具直接跑 SQL（见[明文缓存](#明文缓存)） |
| `d0cb9b1` | `wx cache pull --refresh`，并强制「`--refresh` 必须配 `--shard`」护栏 |

```bash
git clone https://github.com/hooligan520/wx-cli-again.git
cd wx-cli-again
git remote add upstream https://github.com/jackwener/wx-cli-again.git   # 同步上游用
git fetch upstream && git merge upstream/main                            # 按需
```

---

## 特性

- **零依赖安装** — 单一 Rust 二进制，一行命令装完
- **毫秒级响应** — 后台 daemon 持久缓存解密数据库，mtime 不变则复用
- **AI 友好** — `history` / `search` / `sessions` / `new-messages` / `stats` / `attachments` 默认返回 `{..., meta}` wrapper，agent 能直接消费 freshness / source 信息
- **完全本地** — 数据不出本机，实时解密，无需全量预解密
- **按需物化明文** — `wx cache list/pull` 把任意分片落成明文，方便用别的语言直接跑跨库 SQL

---

## 安装

> 上游与本 fork **都是公开仓库**，直接 clone 源码构建即可，不需要任何仓库权限。
> npm 上的 `@jackwener/wx-cli` 只发到 **0.3.0**（早于 0.6.x），公开 registry 拿不到当前版本，
> 所以**默认不要**用 `npm i -g @jackwener/wx-cli`。

### 从源码构建（推荐）

```bash
git clone https://github.com/hooligan520/wx-cli-again.git
cd wx-cli-again
cargo build --release

# 安装到用户 PATH（覆盖旧版）
mkdir -p ~/.local/bin
cp target/release/wx ~/.local/bin/wx
wx --version   # 应显示当前 Cargo.toml 版本，如 0.6.3
```

想跟上游而不是本 fork：把 clone 地址换成 `https://github.com/jackwener/wx-cli-again.git` 即可。

Windows：

```powershell
git clone https://github.com/hooligan520/wx-cli-again.git
cd wx-cli-again
cargo build --release
# 将 target\release\wx.exe 放到 PATH 目录
```

### 已有 clone 时升级

```bash
git pull
cargo build --release
cp target/release/wx ~/.local/bin/wx
```

> ⚠️ 如果你有正在运行的 daemon，**先停掉再覆盖二进制**，否则旧进程仍持有旧文件：
> ```bash
> wx daemon stop        # 失败时用 kill -TERM $(jq -r .pid ~/.wx-cli/daemon.pid)
> cp target/release/wx ~/.local/bin/wx
> ```
> 覆盖后用 `sha256sum target/release/wx ~/.local/bin/wx` 复核一致。

### 用 npm 安装（本地打包，可选）

若你的机器上 `wx` 是 npm 装的薄壳（`node_modules/@jackwener/wx-cli`），
可以用本 fork 的脚本把它换成**内置原生二进制**的包：

```bash
cd wx-cli-again
./npm/pack-local.sh              # 把 target/release/wx 拷进 npm/wx-cli/bin/native/
./npm/pack-local.sh --install    # 打包完顺手 npm install -g
```

脚本会打印二进制 sha256、平台键与版本号，并把各 `package.json` 的版本同步成 `Cargo.toml` 的值。
装完后 `node_modules/@jackwener/wx-cli/bin/wx.js` 会优先使用随包的
`bin/native/wx`，而不是去 resolve 尚不存在的 0.6.x 平台包。

若需要指向别处的二进制，可用环境变量兜底：`WX_CLI_BINARY=/path/to/wx wx --version`。

<details>
<summary>其他方式（GitHub Release / 一键脚本）</summary>

**GitHub Release 预编译包**

从 [Releases](https://github.com/jackwener/wx-cli-again/releases) 下载：

| 平台 | 文件 |
|------|------|
| macOS Apple Silicon | `wx-macos-arm64` |
| macOS Intel | `wx-macos-x86_64` |
| Linux x86_64 | `wx-linux-x86_64` |
| Linux arm64 | `wx-linux-arm64` |
| Windows x86_64 | `wx-windows-x86_64.exe` |

```bash
chmod +x wx-macos-arm64 && mv wx-macos-arm64 ~/.local/bin/wx
```

**用 gh 下载 release asset**

```bash
gh release download -R jackwener/wx-cli-again -p 'wx-macos-arm64' -O ~/.local/bin/wx
chmod +x ~/.local/bin/wx
```

**一键脚本**

```bash
curl -fsSL https://raw.githubusercontent.com/jackwener/wx-cli-again/main/install.sh | bash
```

</details>

---

## 快速开始

### 新用户要做什么 / 不要做什么

| | macOS 新用户 |
|--|--|
| **需要** | 微信 4.x 已安装并**登录**；从**本机 GUI Terminal**（Terminal.app / iTerm 等，不要用 SSH）执行 `sudo wx init` |
| **不需要** | **关闭 SIP** |
| **不需要** | 预先 `codesign` / ad-hoc **重签微信**（默认路径不会改 WeChat.app） |
| **init 之后** | 日常 `wx sessions` / `history` 等**无需 sudo**，也**不要求微信一直开着** |

### 初始化（只需一次）

保持微信运行并已登录，然后：

**macOS**

```bash
# 必须：本机 GUI Terminal + sudo（系统可能提示授予「开发者工具」权限，请允许）
sudo wx init

# 若提示缺分片密钥 / meta.unknown_shards 非空：加长 hook，等待期间在微信里点开相关聊天
sudo wx key extract --hook-seconds 90
```

`wx init` 两阶段取密钥（**都不依赖关 SIP**）：

1. 进程内存扫描（`x'key+salt'` + salt 邻接）
2. LLDB hook `CCCryptorCreate`，补齐尚未加载进内存的 per-DB AES key

说明：

- 官方 **Hardened Runtime** 包：本机 Terminal + `sudo` 即可；若失败，到「系统设置 → 隐私与安全性 → 开发者工具」里勾选你的终端。
- 部分官网 4.x 本身已是 **ad-hoc**：用户态 LLDB 往往也能工作。
- **不要**默认对 WeChat 做 ad-hoc 重签：会打乱 TCC 权限、公众号/截图可能异常。仅当 SSH 等无 GUI 场景下 `task_for_pid` 仍失败时，才考虑有副作用的重签；详见 [macOS 权限指南](docs/macos-permission-guide.md)。

**Linux**

```bash
sudo wx init
```

**Windows**（以管理员身份运行 PowerShell）

```powershell
wx init
```

### 验证

```bash
wx --version
wx doctor          # 密钥 / 分片 / SQLCipher 健康检查
wx sessions
```

能看到最近会话且 `wx doctor` 关键项通过即表示正常。daemon 在首次查询时自动启动。

若 `doctor` 提示关键分片缺密钥，或 `meta.unknown_shards` 非空：

```bash
sudo wx key extract --hook-seconds 90
# 等待期间在微信里打开相关聊天（冷分片可能需触发加载）
wx doctor
```

---

## 命令

### 诊断与密钥

```bash
wx doctor                                 # 环境 / 密钥 / 分片检查
wx doctor --fix --json                   # JSON + 修复建议
wx key list                               # 已有密钥与缺失覆盖
wx key extract --hook-seconds 90          # 建议：sudo wx key extract --hook-seconds 90
wx key set message/message_1.db <64hex>   # 手动写入并校验
```

补密钥统一用 **`sudo wx key extract --hook-seconds 90`**（不要用裸的 `wx init --force` 当主路径）。  
取钥**不需要关闭 SIP**；需本机 GUI Terminal +（Hardened Runtime 包）sudo。

### 明文缓存

需要**用别的语言/工具直接对全部消息分片跑 SQL**（跨库聚合、自建索引等）时用它。
daemon 的明文缓存是内部实现：它首选 `CacheMode::Online`（对加密源库在线打开、
**不产生明文副本**），只有它自己认为需要的少部分库才落盘，所以不能指望
`cache/` 里一定有你要的分片。

```bash
wx cache list                     # 每个分片：源大小 / 有无密钥 / 明文是否已落盘
wx cache list --json
wx cache pull                     # 补齐缺失的：聊天分片 + session + contact
wx cache pull --all               # 补齐所有有密钥且缺失的库（含 fts/media/biz）
wx cache pull --shard message/message_3.db
wx cache pull --shard message/message_3.db --refresh   # 强制重解（会覆盖）
```

落盘位置**硬编码**为 `~/.wx-cli/cache` —— `cache_dir()` = `cli_dir()/cache`，
而 `cli_dir()` 永远是 `~/.wx-cli`（**与 cwd 无关**，socket / pid / log / cache 都不会漂移）。
文件名是 `md5(相对路径).db`，与 daemon 的缓存**同一套命名、同一个目录**，
所以同一个分片不会出现两份明文。

两条规矩：

- **只补齐，绝不覆盖。** 已存在的文件可能是 daemon 落的，它额外合并了 `-wal`
  增量，比只解主库文件更全；覆盖它等于静默丢最新消息。要强制刷新必须
  **点名分片**（`--refresh --shard X`）—— 不允许配 `--all` 一把梭，
  否则一条命令就能把热库的明文降级成无 WAL 版本。
- **不写索引文件。** 存在性即状态：幂等、可重复跑，崩溃也不会留下半截状态。

`--json` 会打印实际写入的路径，方便上层工具直接引用。

### 消息

```bash
wx sessions                                      # 最近 20 个会话
wx unread                                        # 有未读消息的会话
wx unread --filter private,group                 # 只看真人未读（过滤公众号/折叠入口）
wx new-messages                                  # 上次检查后的新消息（增量）
wx history "张三"                                # 最近 50 条记录
wx history "张三" -n 2000                        # 拉更多历史消息
wx history "AI群" --since 2026-04-01 --until 2026-04-15
wx search "关键词"                               # 全库搜索
wx search "关键词" -n 500                        # 放宽搜索结果条数
wx search "会议" --in "工作群" --since 2026-01-01
```

`history` / `search` / `export` 都支持 `-n` / `--limit` 指定条数。默认值只是为了避免一次性输出过多消息，不是硬上限。

会话/消息输出里都带 `chat_type` 字段，取值为 `private` / `group` / `official_account` / `folded`。`official_account` 涵盖公众号、订阅号、服务号及 `mphelper` / `qqsafe` 等系统通知；`folded` 对应微信里的"订阅号折叠"和"折叠群聊"两个聚合入口。

群聊里的 `last_sender`、`sender` 和 `stats` 的 `top_senders` 会优先使用群昵称（群名片）。如果本地数据库里没有对应群昵称，则回退到联系人备注、微信昵称或 username。

`history` / `search` / `new-messages` / `attachments` 以及 `stats.top_senders`，在群聊上下文里还会附带稳定身份三件套：

- `sender_username`：稳定 wxid，用来区分两个昵称同名的成员
- `sender_contact_display`：通讯录里的显示名（备注 > 昵称 > wxid 兜底）
- `sender_group_nickname`：群名片本身（同 `sender` 的来源，方便机器读取时不必再解析）

解析不到 wxid 时（id2u 没命中且老格式 `wxid_xxx:\n...` 前缀也不存在）这三字段不会输出，避免伪造空字段污染下游过滤。

`history` / `search` / `sessions` / `unread` / `new-messages` / `stats` / `attachments` 现在都会附带 `meta`：

- `status`: `ok` / `possibly_stale` / `possibly_stale_unknown_shards` / `windowed`
- `unknown_shards`: 磁盘上存在、但 daemon 当前没有 key 的 `message_N.db` 分片；非空时应先跑 `sudo wx key extract --hook-seconds 90`
- `chat_latest_timestamp` / `chat_latest_db`: 当前命中数据里最新一条消息的时间和分片来源
- `session_last_timestamp`: `session.db` 里 WeChat 自己记录的最新时间；如果明显领先于 `chat_latest_timestamp`，说明结果可能漏了消息

默认情况下，人类用户会在 stderr 看到可执行的 warning；agent / 脚本可直接读 stdout 里的 `meta`。传 `--with-meta` 会额外返回 `per_shard_latest` / `cache_mode_per_shard`，传隐藏 flag `--debug-source` 还会带真实 `shard_paths`。

引用消息会在 `history` / `search` / `new-messages` 输出中显示当前回复和被引用原文：

```text
[引用] 当前回复
  ↳ 发送者: 被引用内容
```

`--type link` / `--type file` 会包含微信 appmsg 里的链接、文件、合并聊天记录和引用消息等变体；搜索时也会匹配解压后可见的引用原文。

### 朋友圈（SNS）

三个独立命令，区分"通知"和"帖子"：

```bash
wx sns-notifications                             # 点赞/评论通知（默认仅未读）
wx sns-notifications --include-read -n 100       # 含已读

wx sns-feed                                      # 近 20 条朋友圈（时间线）
wx sns-feed --user "张三"                        # 限定作者
wx sns-feed --since 2026-04-01 -n 100            # 按时间

wx sns-search "关键词"                           # 全文搜索朋友圈正文
wx sns-search "婚礼" --user "李四" --since 2023-01-01
```

- **sns-notifications** 返回互动通知：`type`（`like`/`comment`）、`from_nickname`、`content`（评论正文）、`feed_preview` + `feed_author`（对应原帖）
- **sns-feed** / **sns-search** 返回朋友圈帖子：`author`、`content`（正文）、`media`、`media_count`、`location`、`timestamp`；`media` 字段含每张图的 url/thumb/key/token/md5/enc_idx/size，供下游做图片代理或离线渲染。`media_count = media.len()`，按 DOM 解析的合法 `<media>` 子节点计数（malformed XML 返回 0）

朋友圈数据只覆盖你本地刷到过的帖子（微信 app 按需下载）。

### 公众号文章

公众号文章推送存在独立的 `biz_message_*.db` 分片，用 `biz-articles` 单独查：

```bash
wx biz-articles                                   # 最近 50 篇
wx biz-articles -n 200                            # 更多
wx biz-articles --account "返朴"                  # 限定公众号（名称模糊匹配）
wx biz-articles --since 2026-05-01 --until 2026-05-10
wx biz-articles --unread                          # 仅有未读的公众号，每号取最新 1 篇
wx biz-articles --json | jq '.[].url'             # 下游消费 URL
```

每条返回：`account` / `account_username` / `title` / `url` / `digest` / `cover_url` / `time` / `timestamp` / `recv_time_str`。多图文推送会展开成多行。

### 附件提取（图片）

聊天里的附件本体存在 `xwechat_files/<wxid>/msg/attach/...` 下的 `.dat` 文件，需要按消息所在 `message_resource.db` 的 md5 + 平台相关 image key 解码才能拿到原图。

```bash
# 1) 列出会话里的图片附件，先拿到不透明的 attachment_id
wx attachments "张三"
wx attachments "AI群" --kind image -n 100
wx attachments "AI群" --since 2026-04-01 --until 2026-04-15

# 2) 把单个 attachment_id 解密写出去（扩展名建议保留 .jpg / .mp4 等）
wx extract <attachment_id> -o ~/Desktop/photo.jpg
wx extract <attachment_id> -o /tmp/x.jpg --overwrite
```

`attachments` 输出每条带：`attachment_id` / `kind` / `type` / `local_id` / `timestamp` / `time`，群聊里还有 `sender` 以及稳定身份三件套 `sender_username` / `sender_contact_display` / `sender_group_nickname`（语义同 `history` / `search` / `new-messages`：`sender_username` 是 wxid，用于两个同名成员之间的稳定区分；解析不到 wxid 时这三字段不输出）。当前 `kind` 固定为 `image`；命令名保留成 `attachments` 是为了后续扩到其他附件类型时不 break CLI。

`extract` 输出报告里带：`md5` / `dat_path` / `dat_size` / `output` / `output_size` / `format`（实际识别出的图片格式：jpg / png / gif / webp / hevc 等）/ `decoder`（实际选用的解码器：`legacy_xor` / `v1_aes` / `v2`）。

支持的解码档位：
- **legacy XOR**：早期单字节 XOR，无 magic（按文件首字节探测格式自动反推）
- **V1 fixed-AES**（`07 08 V1 08 07`）：AES-128-ECB + 固定 key `cfcd208495d565ef`
- **V2 AES + XOR**（`07 08 V2 08 07`）：AES-128-ECB + raw + XOR；AES key 平台派生

V2 image key 提取：
- **macOS**：`kvcomm` cache（`key_<uin>_*.statistic` 文件名取 uin → `md5(str(uin) + wxid)[:16]`）+ brute-force fallback（`md5(str(uin))[:4] == wxid_suffix` 枚举 2^24）；xor_key = `uin & 0xff`，**不是硬编码 0x88**
- **Windows**：扫 `Weixin.exe` 内存匹配 `[A-Za-z0-9]{32|16}` 候选，按 V2 template ciphertext-block 反验
- **Linux**：上游空白，遇到 V2 .dat 会报 unsupported

### 联系人 & 群组

```bash
wx contacts                  # 联系人列表
wx contacts --query "李"     # 按名字搜索
wx members "AI交流群"        # 群成员列表
```

`wx members --json` 返回的成员字段包括：

- `username`：微信内部 username
- `display`：用于展示的名称，优先使用群昵称
- `contact_display`：联系人备注或微信昵称
- `group_nickname`：群昵称；本地没有记录时为空字符串
- `is_owner`：是否群主

### 收藏 & 统计

```bash
wx favorites                          # 全部收藏
wx favorites --type image             # 按类型筛选（text/image/article/card/video）
wx favorites --query "关键词"         # 搜索收藏内容
wx stats "AI群"                       # 聊天统计
wx stats "AI群" --since 2026-01-01   # 指定时间范围
```

### 导出

```bash
wx export "张三" --format markdown -o chat.md
wx export "张三" -n 2000 --format markdown -o chat.md
wx export "AI群" --since 2026-01-01 --format json
```

### 输出格式

默认输出 YAML；`--json` 可切换为 JSON。对 agent 而言，`history` / `search` / `sessions` / `new-messages` / `stats` / `attachments` 的 stdout 现在是 wrapper，而不是裸数组：

```bash
wx sessions --json
wx search "关键词" --json | jq '.results[0].content'
wx new-messages --json
wx history "张三" --json | jq '.meta'
wx history "张三" --json --with-meta | jq '.meta.cache_mode_per_shard'
```

### Daemon 管理

```bash
wx daemon status
wx daemon stop
wx daemon logs --follow
```

---

## 配置与路径

### 配置文件的查找顺序

`config.json` 按 **`WX_CLI_CONFIG` → 当前工作目录（cwd）→ 可执行文件所在目录 → `~/.wx-cli`** 查找，
命中即用。**配置一旦找到，里面的相对路径都相对 `config.json` 自己所在的目录解析**。

> ⚠️ **cwd 会抢占配置。**如果你在一个恰好放着 `config.json` 的项目目录里执行 `wx`，
> 它会把这个文件当成自己的配置 —— 典型症状是报
> `读取密钥文件 "/private/tmp/all_keys.json" 失败`，或者 `wx doctor` 显示「0 个密钥」。
> 这**不是密钥提取失败**，只是读错了配置文件。排查时先 `cd ~/.wx-cli` 再跑一遍。
>
> 反过来说：socket / pid / log / 明文缓存这几样**永远固定在 `~/.wx-cli`**，
> 与 cwd 无关，所以不用担心它们被抢占。

要用一个完全确定的配置文件（脚本、多账号、容器里都推荐）：

```bash
WX_CLI_CONFIG=/Users/me/.wx-cli/config.json wx doctor
```

该文件不存在时会直接报错退出，而**不会**静默回落到别的配置。

### 配置字段

```jsonc
{
  "db_dir": ".../xwechat_files/<wxid>/db_storage",  // 加密库根目录
  "keys_file": "all_keys.json",                     // 相对 config.json 所在目录
  "decrypted_dir": "cache",                         // ⚠️ 见下方说明
  "user_nickname": "",
  "password": "", "password_enabled": false
}
```

- **`keys_file`**：密钥文件位置，`wx init` 会写到这里（fork `0983381` 起）。
  早期版本硬编码成 `<config.json 同目录>/all_keys.json`，配置里写了别的路径也不会生效，
  会出现「`init` 把密钥写进了没人读的地方」。
- **`decrypted_dir`**：⚠️ **这个字段目前被解析但没有任何消费方**
  （`grep '\.decrypted_dir' src/` 无命中），只是历史遗留字段。
  明文目录实际上由 `cache_dir() = cli_dir()/cache` **硬编码**决定，永远是 `~/.wx-cli/cache`。
  改了它**不会**改变落盘位置。

### 环境变量

| 变量 | 作用 |
|---|---|
| `WX_CLI_CONFIG` | 钉死 `config.json` 路径，跳过全部位置探测（fork `0983381`） |
| `WX_CLI_TIMEOUT_SECS` | 覆盖单请求 IPC 读超时（默认 600s，fork `93e75ee`） |
| `WX_CLI_BINARY` | 覆盖 npm 薄壳要执行的二进制路径 |

---

## 架构

```
wx (CLI) ──Unix socket──▶ wx-daemon (后台进程)
                              │
                    ┌─────────┴──────────┐
               DBCache               联系人缓存
           (mtime 感知复用)
```

daemon **优先用 `CacheMode::Online`** —— 对加密源库做 SQLCipher 在线打开，
**不产生明文副本**；只有它自己认为需要的那部分库才 `FullDecrypt` 落盘到
`~/.wx-cli/cache/`（所以 `cache/` 里**不一定**有你要的分片，缺的用 `wx cache pull` 补）。
落盘的库按 mtime / WAL 增量更新，重启后 mtime 未变则直接复用。

```
~/.wx-cli/                 # = cli_dir()，永远在这里，与 cwd 无关
├── config.json            # 配置（可用 WX_CLI_CONFIG 指向别处）
├── all_keys.json          # 数据库密钥（路径由 config.keys_file 决定）
├── daemon.sock            # Unix socket
├── daemon.pid / daemon.log
├── shard-meta.json        # 分片跳跃优化用的 per-talker 时间边界
└── cache/                 # 唯一的明文目录（cache_dir() 硬编码 = cli_dir()/cache）
    ├── _mtimes.json       # mtime 索引
    └── <md5(相对路径)>.db  # 解密后的数据库，daemon 与 `wx cache pull` 共写此处
```

同一个分片在 `cache/` 里只有一个副本：daemon 与 `wx cache pull` 用的是
**同一个命名函数、同一个目录**，而 `wx cache pull` 默认只补缺失、不覆盖。

---

## 原理

微信 4.x 使用 SQLCipher 4 加密本地数据库（AES-256-CBC + HMAC-SHA512，页级 raw key）。每个 DB 有独立的 32-byte AES key；密钥在微信进程打开 DB 时出现在内存中。

⚠️ 微信用的是**非标准页布局**：`PAGE_SZ = 4096`、`SALT_SZ = 16`、**`RESERVE_SZ = 80`**
（尾部 = IV 16 + HMAC 64），而**不是标准 SQLCipher 的 16**。自行解析时若按标准取
「页面最后 16 字节」当 IV，**页 1 的头永远对不上**，很容易把结论误导成「密钥错了」。
参数在 `src/crypto/mod.rs`。页 1 的结构是：`iv = page[4016:4032]`、
密文 `page[16:4016]`，解密结果从逻辑偏移 16 起落，`"SQLite format 3\0"` 明文头需要自己合成；
其余页密文是 `page[0:4016]`，尾部 80 字节丢弃。

验钥推荐用**页 1 的 HMAC-SHA512**（`mac_salt = salt ^ 0x3a`，
`mac_key = pbkdf2_hmac('sha512', key, mac_salt, 2, 32)`，覆盖 `密文 || iv || 页码`），
比「解密后看文件头」可靠得多。

- **首次**：`sudo wx init` — 内存扫描（`x'key+salt'` + salt 邻接）+ 可选 LLDB hook，写入 `config.keys_file`
- **补齐 / 新分片**：`sudo wx key extract --hook-seconds 90`（与 `init --force` 等价，是产品推荐命令）

之后 daemon **优先 SQLCipher 在线打开**加密库（无需全量预解密）；必要时才解密到 `~/.wx-cli/cache/` 并按 mtime / WAL 增量更新。

macOS 取钥依赖本机 Terminal 的 `task_for_pid` / 调试能力，**与是否关闭 SIP 无关**（SIP 保护的是系统组件，不是「能不能读微信内存」的开关）。

---

## 常见问题

| 问题 | 处理 |
|------|------|
| `meta.unknown_shards` / doctor 缺关键分片 | `sudo wx key extract --hook-seconds 90`，等待时打开相关聊天 |
| `wx doctor` 报「0 个密钥」/ `读取密钥文件 "..." 失败` | **先 `cd ~/.wx-cli` 再跑**（cwd 抢占了别的 `config.json`，不是提取失败）；脚本里用 `WX_CLI_CONFIG` 钉死 |
| 某命令卡在整分钟数（如恰好 120.0s）退出、`rc=1`、无输出 | 撞上了 IPC 读超时（`FullDecrypt` 本身要 ~120s）。本 fork 已把默认值改成 600s，可再用 `WX_CLI_TIMEOUT_SECS=900` 覆盖 |
| daemon 首次启动超时 | 首次要解密 ~20 个 DB。本 fork 已把 `STARTUP_TIMEOUT_SECS` 从 15s 改成 180s |
| `wx --version` 偏旧 | `git pull && cargo build --release && cp target/release/wx ~/.local/bin/wx`（先 `wx daemon stop`） |
| `wx history` 的条数随分窗粒度变化、偶发整段为 0 | 已知问题：按 `local_id` 去重，而 `local_id` 只在**单库单表内**唯一；需要精确条数时用 `wx cache pull` 落明文后自行统计 |
| 是否必须关 SIP？ | **否** |
| 是否必须 ad-hoc 重签微信？ | **默认否**；仅 SSH/无 GUI 且 attach 失败时考虑 |
| daemon 无响应 | `wx daemon stop` 后任意查询会自动重启。<br>`wx daemon stop` 报「读取 PID 的 command 失败」时，是因为它用 `ps` 校验进程，在受限 shell 里被禁 → 直接 `kill -TERM $(jq -r .pid ~/.wx-cli/daemon.pid)` |
| 覆盖了二进制但行为没变 | 旧 daemon 进程仍持有旧文件。先 `wx daemon stop` 再覆盖，并用 sha256 复核 |

---

## 致谢

本项目受 [ylytdeng/wechat-decrypt](https://github.com/ylytdeng/wechat-decrypt) 启发，在其基础上进行了重新设计与实现。感谢原作者的研究与探索。

---

## 免责声明

本工具仅用于学习和研究目的，用于解密**自己的**微信数据。请遵守相关法律法规，不得用于未经授权的数据访问。
