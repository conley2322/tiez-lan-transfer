# TieZ 局域网文件传输

自己几台电脑之间，把文件拖一下就越过去。不上传、不中转、不要账号，只在同一局域网里走。

这是开源剪贴板工具 [TieZ](https://github.com/jimuzhe/tiez-clipboard) 的分支（GPLv3，原项目版权归其作者）。仓库里的代码保留它的 Tauri 2 工程骨架与局域网传输通道，但产品已经改成纯文件互传工具：剪贴板历史的监听和界面都不再启用。`LICENSE` 与原项目版权声明按许可证要求原样保留。

---

## 先把它跑起来（3 台电脑的实际步骤）

1. **每台电脑都装一份。** 目前只有 Apple 芯片的 macOS 版本：从 Releases 下载 `TieZ_1.0.0_arm64.dmg`，打开后把图标拖进 Applications。二进制文件不入库，没有 Releases 时照文末「打包 macOS 安装包」自己构建一份。
2. **装完不会弹任何窗口。** 这是有意的：它是个后台程序。去屏幕右上角看那个小竖条，或者看菜单栏的图标。
3. **点菜单栏图标 → 设置… → 设备与用户**，给自己这台电脑起个名字（比如「书房的MacBook」）、挑一个头像颜色。三台电脑分别填，拖拽框上靠这个名字认人。
4. **在同一页点「生成配对码」**，把屏幕上那串码念给对方，或者拍照发过去。
5. **另一台电脑 → 设备与用户 → 输入配对码**，两边都会显示同一组「核对暗号」。暗号一致才点接受——暗号不一致说明码被换过，直接拒绝。
6. **发文件：** 从访达拖一个文件或一个文件夹，拖到屏幕右上角的小框上，框会展开成一列头像，松手放在要发给的那个人身上。
7. **收文件：** 默认是自动接收，落进「下载」文件夹，右下角弹出进度卡，传完发一条系统通知「已接收文件 · 来自XX」。如果你更想先过目，在设置 → 文件传输里改成「确认才能接收」：右下角会出现滑动接收的卡片，滑一下才收，超时没人理它就自动拒收。

一次最多同时发给 8 台设备。文件夹按原目录结构整批发出，对方收到的是完整的文件夹。

---

## 界面上只有四个入口

| 入口 | 什么时候出现 | 能做什么 |
| :--- | :--- | :--- |
| 菜单栏图标 | 一直在 | 左键开设置；右键：设置… / 传输记录… / 暂停接收 / 退出 |
| 角落拖拽框 | 常驻屏幕右上角（可换左上） | 拖文件进来 → 松手在某个头像上 = 发给那台电脑 |
| 右下角进度卡 | 收发文件时 | 每行一个文件，失败行留「重试」，全部完成后按设定时长自动消失 |
| 配对 / 接收浮层 | 有人找你配对、或开了「确认才能接收」 | 核对暗号、滑动接收、拒绝，都带倒计时 |

---

## 设置里的每一项都真的干活

这个项目对设置项的要求是：**开关必须对应后端一个真实生效的动作**，否则界面就是摆设。下面每一项都标了它写进数据库的键名，以及 Rust 什么时候去读它。

### 常规

| 项目 | 键 | 生效点 |
| :--- | :--- | :--- |
| 开机自动启动 | — | macOS 登录项（`SMAppService`，随包内的 `Contents/Library/LaunchAgents/com.tiez.autostart.plist`）。系统设置里关掉时这里会失败并提示，不会假装成功 |
| 语言 | `app.language` | 简体中文 / 繁體中文 / English，立刻生效 |
| 暂停接收 | — | 内存标志，与菜单栏的「暂停接收」双向同步。开着时入站请求直接被拒，不影响你往外发 |
| 传输服务 | — | 停止 / 启动本机 TCP 监听。停了以后别人发不进来，连接自检会显示原因 |

### 设备与用户

列本机身份、已配对设备、附近发现的设备、待确认的配对请求；能改名、改颜色、标收藏、删除设备、生成/输入配对码。底部的**连接自检**六项（传输服务、本机地址、广播发送、广播接收、已配对设备、在线设备）由 Rust 实测得出，每一项连原因一起打印，不是前端猜的。

### 文件传输

| 项目 | 键 | 取值范围 / 生效点 |
| :--- | :--- | :--- |
| 保存位置 | `file_transfer_path` | 默认「下载」；上传/落盘时读，改完下一份文件就走新目录 |
| 接收方式 | `peer_confirm_receive` | `false`（默认）自动收；`true` 需要滑动确认。每次收到邀请时读 |
| 超时自动拒收 | `peer_receive_timeout` | 15–120 秒，默认 60。只在「确认才能接收」下有意义 |
| 收完发系统通知 | `peer_notify_received` | 默认开。一批一条，不是一个文件一条 |
| 显示进度浮层 | `peer_progress_float` | 默认开。关掉后收发都在后台跑，只剩通知 |
| 浮层位置 | `peer_progress_position` | 右下 / 左下 / 右上，改完立刻重排已显示的窗口 |
| 完成后停留 | `peer_progress_hold` | 1–30 秒，默认 3。有失败行时不自动消失 |
| 逐文件 SHA-256 校验 | — | **固定开启，不是开关**，所以界面上只写状态、不给选择 |

### 角落拖拽框

`app.corner_box_enabled`、`app.corner_box_side`（top-right / top-left）、`app.corner_box_opacity`（40–100，默认 80）、`peer_corner_favorites_only`（只显示标了收藏的人）。这几项走专用命令，因为要挪动和重绘一个已经在屏幕上活着的窗口。

### 右键菜单

这是一页**状态说明**，不是开关集合：Mac 上的访达右键需要 FinderSync 扩展并签名后才能打进安装包，Windows 上的右键子菜单（HKCU 注册表方案）还没写。现在能用的发送方式就是角落拖拽框。与其摆一排按了没反应的按钮，不如把话说明白。

### 传输记录

按「一批」分组显示历史，一批只有一个「重试」（重发整批，和后端语义一致）；清空要二次确认。记录表最多留 1000 条。

### 关于

版本号、本机地址、数据目录（`~/Library/Application Support/com.tiez`）与打开它、退出程序。

---

## 网络要走哪些端口

| 用途 | 协议 | 端口 / 地址 |
| :--- | :--- | :--- |
| 文件传输与配对 | TCP | 28462（数据库键 `file_server_port`） |
| 设备发现（IPv4 广播） | UDP | 28463 |
| 设备发现（IPv6 组播） | UDP | 28464，组地址 `ff15::7469:657a` |

广播每 3 秒一次，15 秒没收到就判定对方离线。第一次运行时 macOS 会要求放行程序，允许即可；找不到对方电脑时先检查路由器有没有开 AP 隔离，以及两台电脑是否真的在同一网段。

---

## 还没做

- Windows 右键「发送给」子菜单（方案已定：HKCU 注册表 + 二级菜单列用户名，不用 COM）。
- Intel Mac 和 Windows 的安装包。当前 DMG 只适用于 Apple 芯片。
- 访达右键扩展（需要签名，见「右键菜单」页）。
- 剪贴板历史：代码还在仓库里，但监听和界面都不启用。这个 fork 是文件传输工具。

---

## 开发

```bash
npm install
npm run tauri:dev          # 开发运行（带 devtools）
```

改完必须过的门禁，一条都别跳：

```bash
npx tsc --noEmit
npx eslint src
node scripts/lint-styles.mjs                                   # 设计令牌校验，规则见 docs/DESIGN-SYSTEM.md
npx vitest run src/contracts src/locales.test.ts src/features/corner
cd src-tauri && cargo test --bin tiez-app
```

`src/contracts/tauriCommands.test.ts` 会用 AST 扫前端所有 `invoke` 字面量，逐个核对命令是否真的注册在 `generate_handler![]` 里、参数名对不对。加后端命令忘了注册、或者前端拼错参数名，都在这里当场失败。

### 目录

```
src/features/lan/            设置窗口（7 页）、三个右下浮层、文案（zh/en/tw）、Rust 调用封装
src/features/corner/         角落拖拽框页面与命中判定
src/styles/components/       lan-settings.css、lan-floats.css、corner-box.css
src-tauri/src/services/peer_transfer/   发现、配对、名册、接收、传输记录、连接自检
src-tauri/src/app/           corner_box.rs、peer_floats.rs、floating_windows.rs、tray.rs
src-tauri/capabilities/      窗口权限白名单
```

窗口靠 `index.html?window=<label>` 分流：`corner`（拖拽框）、`settings`（设置）、`progress` / `pair` / `receive`（右下浮层）。**加新窗口必须同时改两处**：`src/main.tsx` 里选组件，以及 `src-tauri/capabilities/default.json` 的 `windows` 列表——漏掉后者，那个窗口的每一次 `invoke` 都会静默失败。

浮层窗口都是 `visible(false)` + 透明 + `focusable(false)` 创建，由 Rust 在事件到来时唤起；待确认记录在唤起之前就写进内存表，所以页面读到空列表时可以放心把自己藏起来。

### 打包 macOS 安装包

```bash
npx tauri build --target aarch64-apple-darwin --bundles app \
  --config '{"bundle":{"createUpdaterArtifacts":false}}'
codesign --force --deep --sign - \
  src-tauri/target/aarch64-apple-darwin/release/bundle/macos/TieZ.app
```

`--bundles dmg` 这一步经常失败，原因有两个：`create-dmg` 的中间镜像被写进了源目录，于是它把自己反复塞进自己的镜像，直到报「设备上无剩余空间」；以及它的 Finder 摆图步骤需要自动化授权。签名后用 `--skip-jenkins` 手动出包（只少掉图标摆位，拖拽安装不受影响）：

```bash
cd src-tauri/target/aarch64-apple-darwin/release/bundle/dmg
rm -f ../macos/rw.*.dmg        # 清掉上一次的中间镜像
bash ./bundle_dmg.sh --skip-jenkins --volname "TieZ 1.0.0" \
  --volicon "$PWD/app.icns" --hide-extension "TieZ.app" \
  --app-drop-link 618 243 --icon "TieZ.app" 192 243 \
  --no-internet-enable "$PWD/TieZ_1.0.0_aarch64.dmg" "../macos"
```

本地自用走 ad-hoc 签名就够；要发给别人的安装包需要正式签名，未签名的包在对方机器上需要右键「打开」绕过 Gatekeeper。

---

## 仓库里的文档

- `docs/DESIGN-SYSTEM.md` — 样式令牌与栅格，`scripts/lint-styles.mjs` 按它校验，改样式前先看。
- `docs/LAN-TRANSFER-ENCRYPTION.md`、`docs/E2E-ENCRYPTION.md` — 局域网传输与配对通道的加密设计，沿用了原项目的通道实现。
- `docs/RELEASES.md`、`docs/THEME-SYSTEM.md`、`docs/THEME-PACKAGE-V1.md`、`docs/SYNC-NOTIFICATIONS.md` — 打包、主题与同步的说明。其中主题在本 fork 的设置界面里不开放；同步（WebDAV / MQTT）属于已停用的剪贴板功能，留着是因为代码没删。

## 许可

GPLv3，见 `LICENSE`。原项目 TieZ 的版权归其作者所有；本仓库的修改部分在同一许可证下发布，允许自由使用与再分发，前提是同样公开源代码并保留本许可。
