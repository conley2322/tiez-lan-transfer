# TieZ 局域网文件传输

自己几台电脑之间，文件拖一下、右键一下就过去了。不上传、不中转、不要账号，只在同一局域网里走。

这是开源剪贴板工具 [TieZ](https://github.com/jimuzhe/tiez-clipboard) 的分支（GPLv3，原项目版权归其作者）。仓库里的代码保留它的 Tauri 2 工程骨架与局域网传输通道，但产品已经改成纯文件互传工具：剪贴板历史的监听和界面都不再启用。`LICENSE` 与原项目版权声明按许可证要求原样保留。

---

## 先把它跑起来（3 台电脑的实际步骤）

1. **每台电脑都装一份。** 安装包都在这个仓库的 Releases 页上：苹果芯片的 Mac 下载 tag `mac-dev` 里的 `TieZ_1.0.0_arm64.dmg`，打开后把图标拖进 Applications；Windows（10/11，x64）下载 tag `win-dev` 里的 `TieZ_1.0.0_x64-setup.exe`，双击安装即可，装在您当前用户下（不需要管理员权限）。这两条是每次推 main 自动重新打的，永远对应仓库最新代码，不需要登录 GitHub 就能下载。
   安装包没有付费签名，第一次会被拦：Mac 上如果提示「无法验证开发者」，在访达里右键这个 app →「打开」→ 再点一次「打开」，只需第一次这么做；Windows 上 SmartScreen 会弹「已保护你的电脑」，点「更多信息」→「仍要运行」。
   装好后的第一次启动，Windows 会弹窗问「是否允许此应用对你的设备进行访问」，**勾上「专用网络」再点允许**，不勾就互相找不到——软件自己不会改防火墙设置，这个同意只能你给。
   想要 Intel 的 Mac 包，或者不想用 CI 出的这一份，照文末「开发」里的打包小节自己构建。
2. **装完不会弹任何窗口。** 这是有意的：它是个后台程序。Mac 看菜单栏图标——发送图标平时不显示，拖起文件时才出现在鼠标旁边（也可以在设置里固定到左上/右上角）；Windows 去看右下角通知区（被藏起来的话，点一下那个向上的小箭头把它拖出来）。
3. **点图标 → 设置… → 设备与用户**，给自己这台电脑起个名字（比如「书房的MacBook」）、挑一个头像颜色。三台电脑分别填，拖拽框和右键菜单都靠这个名字认人。开着自动发现时，附近同一网段的电脑会出现在这一页的「附近的电脑」里，名字会跟着对方那边的修改自动更新。
4. **配对有两种办法。** 在「附近的电脑」里直接点对方的名字，对方屏幕上会弹出「连接请求」，滑一下接受就成了，不用念密码。或者点「生成配对码」，把屏幕上那串码念给对方 / 拍照发过去，另一台电脑 → 设备与用户 → 输入配对码；两边都会显示同一组「核对暗号」，暗号一致才点接受——暗号不一致说明码被换过，直接拒绝。
5. **（用配对码时）另一台电脑 → 设备与用户 → 输入配对码**，两边都会显示同一组「核对暗号」。暗号一致才点接受——暗号不一致说明码被换过，直接拒绝。
6. **发文件：** Mac 上从访达拖一个文件或一个文件夹，鼠标旁边会随拖拽出现一个小图标；把文件拖到图标上，它会展开成一列头像，松手放在要发给的那个人身上。拖起文件后把鼠标左右晃两下，头像列表会直接弹出来（设置 → 角落拖拽框里可以关掉这个手势）。Windows 上选中一个或多个文件（也可以选文件夹）→ 右键 → 二级菜单「发送给 TieZ」→ 点对方的名字；Windows 11 的简化右键里没有它，先点「显示更多选项」，或者按住 `Shift` 再右键。
7. **收文件：** 默认是自动接收，落进「下载」文件夹，右下角弹出进度卡，传完发一条系统通知「已接收文件 · 来自XX」。如果你更想先过目，在设置 → 文件传输里改成「确认才能接收」：右下角会出现滑动接收的卡片，滑一下才收，超时没人理它就自动拒收。

一次最多同时发给 8 台设备，一次最多 2000 个文件。文件夹按原目录结构整批发出，对方收到的是完整的文件夹。Windows 上多选几个文件一起右键时，应用会把它们合成一批发出——文件管理器是每选中一个文件就启动一次应用，这层合并就发生在应用里。

---

## 界面上只有这几个入口

| 入口 | 哪个系统 | 什么时候出现 | 能做什么 |
| :--- | :--- | :--- | :--- |
| 菜单栏 / 通知区图标 | 都有 | 一直在 | 左键开设置；右键：设置… / 传输记录… / 暂停接收 / 退出 |
| 角落拖拽框 | 仅 Mac | 拖起文件时出现在鼠标旁（可改为固定左上/右上），平时隐藏 | 拖文件进来 → 松手在某个头像上 = 发给那台电脑；拖拽中晃两下鼠标直接展开头像列表 |
| 右键「发送给 TieZ」 | 仅 Windows | 挂在文件管理器右键里（可在设置里关掉） | 二级菜单列出已配对的用户名，点一个就发过去 |
| 右下角进度卡 | 都有 | 收发文件时 | 每行一个文件，失败行留「重试」，全部完成后按设定时长自动消失 |
| 配对 / 接收浮层 | 都有 | 有人找你配对、或开了「确认才能接收」 | 核对暗号、滑动接收、拒绝，都带倒计时 |

发文件的方式每个系统只有一个：Mac 拖，Windows 右键。设置里也因此只显示当前系统真能用到的那一页——Windows 上不会出现「角落拖拽框」的设置项，那几项在 Windows 上没有任何东西去读。

没有主窗口。老版本剪贴板工具的那个主界面在这个 fork 里已经被移除：代码里根本没有创建它的地方，所以任何「打开软件看到的内容」都不存在。

---

## 设置里的每一项都真的干活

这个项目对设置项的要求是：**开关必须对应后端一个真实生效的动作**，否则界面就是摆设。下面每一项都标了它写进数据库的键名，以及 Rust 什么时候去读它。

### 常规

| 项目 | 键 | 生效点 |
| :--- | :--- | :--- |
| 开机自动启动 | — | Mac：登录项（`SMAppService`，随安装包内的 `Contents/Library/LaunchAgents/com.tiez.autostart.plist`）。Windows：当前用户的 `HKCU\...\Run` 值，卸载时会清掉。系统里被关掉时这一项会失败并提示，不会假装成功 |
| 语言 | `app.language` | 简体中文 / 繁體中文 / English，立刻生效；Windows 上顺带把右键那级菜单的标题文字换成对应语言 |
| 暂停接收 | — | 内存标志，与菜单栏的「暂停接收」双向同步。开着时入站请求直接被拒，不影响你往外发 |
| 传输服务 | — | 停止 / 启动本机 TCP 监听。停了以后别人发不进来，连接自检会显示原因 |

### 设备与用户

列本机身份、已配对设备、附近发现的设备、待确认的配对请求；能改名、改颜色、标收藏、删除设备、生成/输入配对码。附近的电脑点一下就能发连接请求，对方滑一下接受即可配对，两端都不需要再抄码。名字同步是单向覆盖的：对方改了它自己的名字，你这边会跟着变；但你在自己这台电脑上给它起过名字（「改名」里填了内容），就以你填的为准，清空后又会跟着对方走。底部的**连接自检**六项（传输服务、本机地址、广播发送、广播接收、已配对设备、在线设备）由 Rust 实测得出，每一项连原因一起打印，不是前端猜的。

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

### 角落拖拽框（仅 Mac）

`app.corner_box_enabled`、`app.corner_box_side`（follow / top-right / top-left，默认 follow 跟随拖拽位置）、`app.corner_box_opacity`（40–100，默认 80）、`peer_corner_favorites_only`（只显示标了收藏的人）、`app.corner_shelf_enabled`（拖拽中晃两下鼠标直接展开头像列表，默认开）。这几项走专用命令，因为要挪动和重绘一个按需出现的窗口。Windows 上这一页整个不显示——那个窗口只在 Mac 创建。

### 右键菜单

Mac 上是**状态说明**：访达右键需要 FinderSync 扩展，而扩展要签名后才能打进安装包，没签名的包在系统里装不上，所以现在 Mac 的发送方式只有角落拖拽框。

Windows 上是**真的开关**：`win_context_menu_enabled`（缺省即开）。关掉时 Rust 把 `HKCU\Software\Classes\*\shell\TieZSend` 和 `HKCU\Software\Classes\Directory\shell\TieZSend` 两棵子树删掉并通知资源管理器刷新；打开时按当前已配对设备重建，二级菜单每一项就是一个用户名，命令是 `"<exe>" --win-send "<设备id>" "%1"`。这一页底下显示的**用户名列表不是前端拼的**，是把注册表里现有的子键读回来给你看，改名或配对后跟着变。两个已知情况直接印在页面上：Windows 11 的简化右键菜单里没有它，要点「显示更多选项」；如果这个应用被以管理员身份运行，普通权限的第二进程没法把文件交给已经运行的那个实例，右键会没反应。

### 传输记录

按「一批」分组显示历史，一批只有一个「重试」（重发整批，和后端语义一致）；清空要二次确认。记录表最多留 1000 条。

### 关于

版本号、本机地址、数据目录（Mac 是 `~/Library/Application Support/com.tiez`，Windows 安装包用的是 `com.tiez.app`，即 `%APPDATA%\com.tiez.app`）与打开它、退出程序。

---

## 网络要走哪些端口

| 用途 | 协议 | 端口 / 地址 |
| :--- | :--- | :--- |
| 文件传输与配对 | TCP | 28462（数据库键 `file_server_port`） |
| 设备发现（IPv4 广播 / 组播） | UDP | 28463，广播地址 `255.255.255.255`，组地址 `224.0.0.167` |
| 设备发现（IPv6 组播） | UDP | 28464，组地址 `ff15::7469:657a` |

广播每 3 秒一次，15 秒没收到就判定对方离线。两块网卡的电脑会从每块网卡各发一份；一台机器收到广播后还会用单播回一条（同一个 28463 端口，带 `reply` 标记），这样即使某一路广播被防火墙挡掉，另一台也能知道彼此。macOS 15 起第一次访问局域网会要求「本地网络」权限，安装包已带好说明文字，弹窗时点允许即可。第一步里的两个「允许」没给的话，表现就是谁都发现不了谁；设置 → 设备与用户 底部的连接自检会把每一项连同原因打印出来，先看它。之后的顺序：路由器的 AP 隔离、两台电脑是否同一网段、Windows 防火墙里那条规则还在不在（被拒过一次，只能去「允许应用通过 Windows 防火墙」里手动勾回来）。

---

## 还没做

- Intel Mac 的安装包（当前 Mac 包只适用于 Apple 芯片，Windows 包只有 x64）。
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

**本机构建覆盖不到的那一半**：`#[cfg(windows)]` 里的代码在这台 Mac 不参与编译，交叉编译又过不去（缺 mingw，`libsqlite3-sys` 打不动）。Windows 侧的类型检查唯一落点是 `ci.yml` 里的 `Rust (windows-latest)` job，它每次 push 都会跑。改完 Windows 代码要等这条 job 绿，才能说「Windows 这边没问题」。

### 目录

```
src/features/lan/            设置窗口（7 页）、三个右下浮层、文案（zh/en/tw）、Rust 调用封装
src/features/corner/         角落拖拽框页面与命中判定
src/styles/components/       lan-settings.css、lan-floats.css、corner-box.css
src-tauri/src/services/peer_transfer/   发现、配对、名册、接收、传输记录、连接自检
src-tauri/src/services/peer_transfer/win_menu.rs   Windows 右键：注册表读写 + 多选合并发送
src-tauri/src/app/           corner_box.rs、peer_floats.rs、floating_windows.rs、tray.rs
src-tauri/src/app/commands/win_menu_cmd.rs         右键菜单页读写的那两条命令
src-tauri/capabilities/      窗口权限白名单
src-tauri/nsis/uninstall.nsh Windows 卸载时清右键注册表、Run 值和残留文件
src-tauri/tauri.windows.conf.json                 Windows 的全部打包配置
```

平台相关的代码有两种写法，别混：命令要进 `generate_handler![]`，所以 `win_menu_cmd.rs` 这种文件在每个平台都参与编译，函数体里用 `#[cfg(target_os = "windows")] { …; return X; }` 分岔；不进命令表的（`win_menu.rs`）才整个文件用 `#[cfg(windows)] pub mod` 挡住。

这台 Mac 编译不了 `#[cfg(windows)]` 的代码——没有 mingw，`libsqlite3-sys` 就过不去。Windows 侧的类型检查只由 `ci.yml` 里那个 `Rust (windows-latest)` job 负责，它每次推送都会真的在 Windows 上编译一遍，`#[cfg]` 里的错误在那里暴露。改完 Windows 代码，看这条 job 绿了再说「没问题」。

窗口靠 `index.html?window=<label>` 分流：`corner`（拖拽框）、`settings`（设置）、`progress` / `pair` / `receive`（右下浮层）。**加新窗口必须同时改两处**：`src/main.tsx` 里选组件，以及 `src-tauri/capabilities/default.json` 的 `windows` 列表——漏掉后者，那个窗口的每一次 `invoke` 都会静默失败。

浮层窗口都是 `visible(false)` + 透明 + `focusable(false)` 创建，由 Rust 在事件到来时唤起；待确认记录在唤起之前就写进内存表，所以页面读到空列表时可以放心把自己藏起来。

### 打包 macOS 安装包

安装包由 GitHub Actions 的 `macos-14` 打。推一次 main（或到 Actions 里手动运行「打包 macOS 安装包」）之后，产物出现在两个地方：

- Release 页 tag `mac-dev` 上的 `TieZ_1.0.0_arm64.dmg`（每次构建覆盖上一份）；
- 那次运行的 workflow 产物 `tiez-macos-installer`（下载要登录 GitHub）。

那条构建没有把 DMG 交给 `tauri build`：CI 上没有 Developer ID 证书，Tauri 打完 `.app` 直接交给 `bundle_dmg.sh` 的话，那份 `.app` 只有链接器签名，`codesign --verify` 会报 "code has no resources"。所以它分三步——`--bundles app` 只出 `.app`、`codesign --sign -` 补一次 ad-hoc 签名、`hdiutil create` 自己装成 DMG。挂 Release 那一步失败不会让构建变红，原因打在那次运行的 annotation 上。

想在本地自己打一份，命令是：

```bash
npx tauri build --target aarch64-apple-darwin --bundles app \
  --config '{"bundle":{"createUpdaterArtifacts":false}}'
codesign --force --deep --sign - \
  src-tauri/target/aarch64-apple-darwin/release/bundle/macos/TieZ.app
```

`--bundles dmg` 这一步在本机经常失败，原因有两个：`create-dmg` 的中间镜像被写进了源目录，于是它把自己反复塞进自己的镜像，直到报「设备上无剩余空间」；以及它的 Finder 摆图步骤需要自动化授权。签名后用 `--skip-jenkins` 手动出包（只少掉图标摆位，拖拽安装不受影响）：

```bash
cd src-tauri/target/aarch64-apple-darwin/release/bundle/dmg
rm -f ../macos/rw.*.dmg        # 清掉上一次的中间镜像
bash ./bundle_dmg.sh --skip-jenkins --volname "TieZ 1.0.0" \
  --volicon "$PWD/app.icns" --hide-extension "TieZ.app" \
  --app-drop-link 618 243 --icon "TieZ.app" 192 243 \
  --no-internet-enable "$PWD/TieZ_1.0.0_aarch64.dmg" "../macos"
```

本地自用走 ad-hoc 签名就够；要发给别人的安装包需要正式签名，未签名的安装包在对方机器上需要右键「打开」绕过 Gatekeeper。Intel 芯片的 Mac 包两种方法都还没试过。

### 打包 Windows 安装包

这个不在本机打：Windows 交叉编译在本机过不去，安装包由 GitHub Actions 的 `windows-latest` 打。推一次 main（或到 Actions 里手动运行「打包 Windows 安装包」）之后，产物出现在两个地方：

- Release 页 tag `win-dev` 上的 `TieZ_1.0.0_x64-setup.exe`（每次构建覆盖上一份）；
- 那次运行的 workflow 产物 `tiez-windows-installer`（下载要登录 GitHub）。

挂 Release 那一步失败不会让构建变红——安装包已经在 workflow 产物里了，失败原因会打在那次运行的 annotation 上。

想在本地看命令长什么样，等价的一条是：

```bash
npm ci
npx tauri build --config src-tauri/tauri.windows.conf.json
# → src-tauri/target/release/bundle/nsis/TieZ_1.0.0_x64-setup.exe
```

`--config` 这个文件不能省：identifier（`com.tiez.app`）、`targets: ["nsis"]`、`installMode: currentUser`、卸载钩子都在里面，基础配置只有 dmg/app。安装包不签名，也没有 updater 产物（`createUpdaterArtifacts: false`）——本 fork 的界面里没有检查更新的入口，多打一个 `.sig` 只是让人以为有。

---

## 仓库里的文档

- `docs/DESIGN-SYSTEM.md` — 样式令牌与栅格，`scripts/lint-styles.mjs` 按它校验，改样式前先看。
- `docs/LAN-TRANSFER-ENCRYPTION.md`、`docs/E2E-ENCRYPTION.md` — 局域网传输与配对通道的加密设计，沿用了原项目的通道实现。
- `docs/RELEASES.md`、`docs/THEME-SYSTEM.md`、`docs/THEME-PACKAGE-V1.md`、`docs/SYNC-NOTIFICATIONS.md` — 打包、主题与同步的说明。其中主题在本 fork 的设置界面里不开放；同步（WebDAV / MQTT）属于已停用的剪贴板功能，留着是因为代码没删。

## 许可

GPLv3，见 `LICENSE`。原项目 TieZ 的版权归其作者所有；本仓库的修改部分在同一许可证下发布，允许自由使用与再分发，前提是同样公开源代码并保留本许可。
