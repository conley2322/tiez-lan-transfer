# TieZ 客户端发布流程

`versions.json` 是客户端版本号的唯一来源，分别记录：

- `stable.windows` / `stable.macos`：正式版
- `beta.windows` / `beta.macos`：群测试版

构建脚本会把目标版本同步到 `package.json`、`src-tauri/Cargo.toml` 和
`src-tauri/tauri.conf.json`，并校验更新接口、发布渠道与 Git 标签。

## 群测试版

测试版必须使用 SemVer 预发布后缀，例如：

```json
{
  "beta": {
    "windows": "1.0.0-beta.1",
    "macos": "1.0.0-beta.1"
  }
}
```

推荐用 GitHub Actions 构建（两个平台都会签名，macOS 还会公证）：

1. 开一个只改 `versions.json` 的 PR，CI 通过后合并到 `master`。
2. 从 `master` 触发两个测试版工作流：

   ```bash
   gh workflow run release-beta-macos.yml --ref master -f git_ref=master
   gh workflow run release-beta-windows.yml --ref master -f git_ref=master
   ```

3. 产物是 GitHub 上的**草稿**预发布版（`beta-mac-v…`、`beta-v…`），只有维护者能看到。

不再为每个测试版单独建 `beta/…` 分支。

测试分两个阶段：

- **群测**：草稿保持不公开，维护者下载安装包发到测试群。此阶段不建版本后台的
  beta 批次（自动更新要求下载地址公开）。
- **公测**：群里的问题处理完后，把草稿发布为预发布版（不要设为 Latest），
  再在 TieZ Analytics 后台建 beta 批次。

也可以在对应系统上本地构建：

```bash
npm ci
npm run release:beta:win
# 或
npm run release:beta:mac
```

构建结束后脚本会恢复 package、Cargo 和 Tauri 配置，只保留
`versions.json` 中由维护者明确修改的版本号，避免误提交 beta 生成状态。

构建前需要在本机环境中配置 Tauri 签名私钥。签名私钥不能提交到仓库、
上传到版本后台或发送到测试群。

测试版安装包会固定使用 `beta` 更新通道。服务端在 `channel=beta` 时会同时
比较 beta 与 stable 上已发布的更高版本，因此测试用户可以从
`1.0.0-beta.x` 通过「检查更新」升到正式版 `1.0.0` 及之后的 stable 补丁。
正式版（1.0.0 起）也会查询 beta 渠道：有更新的测试版时，只在「设置 → 关于」里
显示一行带「测试版」标记的提示，不弹窗；用户打开「接收测试版更新」后，测试版才会
像普通更新一样提示。0.3.x / macOS 0.1.x 等旧版本不带渠道参数，只收到 stable。

构建完成后：

1. 将安装包发送给测试群。
2. 如需让测试用户自动更新，将 updater 制品和 `.sig` 上传到 HTTPS 地址。
3. 在 TieZ Analytics 后台创建 `beta` 发布批次，填写对应架构的 URL 和签名。
4. 测试通过后，把不带预发布后缀的版本写入 `stable`，再走正式发布。

如果只想单次发包、不提供后续自动更新，可以不在后台发布 beta 批次。

## 正式版

更新 `stable` 版本后，在对应系统构建：

```bash
npm run release:stable:win
# 或
npm run release:stable:mac
```

GitHub Actions 正式发布标签：

- Windows：`v1.0.0`
- macOS：`mac-v1.0.0`

工作流会阻止标签、平台版本、Cargo、Tauri 配置或更新通道不一致的构建。

## 便携版

Windows 便携版不会运行安装器自动更新。用户需要从官网或测试群下载新的
便携包并替换程序文件，原有 `data` 目录会继续保留。
