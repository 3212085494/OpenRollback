# OpenRollback — GitHub 首发指南（逐步操作）

> **版本说明**：项目当前版本号为 **0.1.0**（与安装包文件名一致）。  
> 若你希望对外叫「v1.0.0」，可在 Release 时使用 `v1.0.0`，但建议与 `package.json` 保持一致，使用 **`v0.1.0`**。

---

## 第 0 步：确认本机已安装 Git

在 PowerShell 中输入：

```powershell
git --version
```

- **若显示版本号**（如 `git version 2.x`）→ 继续第 1 步。
- **若提示找不到命令** → 请先安装：
  1. 打开 https://git-scm.com/download/win
  2. 下载并安装（一路「Next」即可）
  3. **关闭并重新打开** Cursor 终端
  4. 再次运行 `git --version`

---

## 第 1 步：在 GitHub 网站创建空仓库（需手动）

1. 浏览器打开 https://github.com 并登录
2. 右上角 **+** → **New repository**
3. 填写：
   - **Repository name**：`OpenRollback`（或 `openrollback`，建议与文件夹一致）
   - **Description**：`AI 代码时光机 — 本地文件快照与一键回滚`
   - **Visibility**：**Public**
4. **重要**：不要勾选以下任何一项：
   - ❌ Add a README file
   - ❌ Add .gitignore
   - ❌ Choose a license
5. 点击 **Create repository**
6. **手动复制** 页面上的 HTTPS 地址，形如：

   ```
   https://github.com/3212085494/OpenRollback.git
   ```

   记下你的用户名，后面要替换 `你的用户名`。

---

## 第 2 步：本地 Git 初始化并上传

在 Cursor 终端中**逐行**执行（可复制粘贴）。

### 2.1 进入项目目录

```powershell
cd <你的项目路径>   # 例如：cd C:\Users\YourName\Desktop\OpenRollback
```

### 2.2 确认不会误上传大文件夹

```powershell
git status
```

若提示 `not a git repository`，说明还没初始化，继续下面步骤。

若 `git status` 里出现 `node_modules` 或 `target`，先执行：

```powershell
git rm -r --cached node_modules dist src-tauri/target 2>$null
```

### 2.3 初始化并提交

```powershell
git init
git add .
git status
```

**检查 `git status`**：应只看到源代码、README、LICENSE 等；**不应**出现 `node_modules/`、`dist/`、`src-tauri/target/`。

```powershell
git commit -m "Initial commit: OpenRollback v0.1.0 ready for launch"
git branch -M main
```

### 2.4 关联远程并推送

**把下面命令里的地址换成你在第 1 步复制的地址：**

```powershell
git remote add origin https://github.com/3212085494/OpenRollback.git
git push -u origin main
```

### 2.5 推送时常见问题

| 报错 | 解决办法 |
|------|----------|
| 要求登录 / Authentication failed | 浏览器弹出 GitHub 登录窗口 → 点授权；或使用 **Personal Access Token** 作为密码（见下方附录 A） |
| `failed to push some refs` / `rejected` | 说明你远程仓库不是空的。若你误勾了 README，请删掉远程仓库重建，或执行 `git pull --rebase origin main` 后再 push |
| `remote origin already exists` | 执行 `git remote set-url origin https://github.com/你的用户名/OpenRollback.git` |

推送成功后，刷新 GitHub 仓库页面，应能看到所有代码文件。

---

## 第 3 步：确认安装包已打好

在推送 Release 之前，确认这两个文件存在：

```powershell
dir src-tauri\target\release\bundle\nsis\OpenRollback_0.1.0_x64-setup.exe
dir src-tauri\target\release\bundle\msi\OpenRollback_0.1.0_x64_en-US.msi
```

若不存在，先打包：

```powershell
npm run tauri build
```

预计等待 1～3 分钟。

---

## 第 4 步：发布 Release（需手动在浏览器）

1. 打开 `https://github.com/3212085494/OpenRollback`
2. 右侧 **Releases** → **Create a new release**
3. 填写：

   | 字段 | 填写内容 |
   |------|----------|
   | **Tag** | 选择 **Create new tag**：`v0.1.0` |
   | **Target** | `main` |
   | **Release title** | `OpenRollback v0.1.0 — 首发版` |

4. **Description**（可复制下方内容）：

   ```markdown
   ## OpenRollback v0.1.0 — 首发版

   AI Agent 的本地文件时光机：监控目录变更、增量快照、时间轴一键回滚。

   ### 下载（Windows x64）

   | 文件 | 说明 |
   |------|------|
   | `OpenRollback_0.1.0_x64-setup.exe` | 推荐：NSIS 安装程序 |
   | `OpenRollback_0.1.0_x64_en-US.msi` | MSI 安装包 |

   ### 核心功能

   - 目录监控（支持中文路径）
   - 增量快照 + SQLite 记录
   - 删除文件可回滚（预缓存）
   - MCP 集成（Cursor / Claude）

   ### 系统要求

   - Windows 10/11 x64
   - WebView2（Win10 通常已自带）

   ### 快速上手

   1. 安装后打开 OpenRollback
   2. 点击 **WATCH** 选择项目子文件夹
   3. 修改文件 → **创建快照**
   4. 出问题 → 时间轴 **回滚**
   ```

5. 在 **Attach binaries** 区域，拖入这两个文件：

   ```
   src-tauri\target\release\bundle\nsis\OpenRollback_0.1.0_x64-setup.exe
   src-tauri\target\release\bundle\msi\OpenRollback_0.1.0_x64_en-US.msi
   ```

6. 点击 **Publish release**

---

## 第 5 步：最终检查清单

- [ ] GitHub 仓库能看到 `README.md`、`LICENSE`、`src/`、`src-tauri/`
- [ ] 仓库里**没有** `node_modules`（若有一说明 `.gitignore` 未生效）
- [ ] Releases 页面有 `v0.1.0`
- [ ] 能点击下载 `.exe` 和 `.msi`
- [ ] 本机双击安装包能正常安装并打开

---

## 附录 A：Personal Access Token（推送失败时用）

1. GitHub → 头像 → **Settings**
2. **Developer settings** → **Personal access tokens** → **Tokens (classic)**
3. **Generate new token (classic)**
4. 勾选 **repo** 权限
5. 生成后**复制 token**（只显示一次）
6. `git push` 时：
   - Username：你的 GitHub 用户名
   - Password：**粘贴 token**（不是登录密码）

---

## 附录 B：Release 说明文案

发布 v0.1.0 时，直接复制 [`docs/RELEASE_NOTES_v0.1.0.md`](../RELEASE_NOTES_v0.1.0.md) 中的内容到 GitHub Release 描述框。

---

## 附录 C：一键验证脚本（可选）

上传前在本地跑一遍发布验证：

```powershell
.\scripts\release_verify.ps1
```

全部 PASS 后再执行第 2～4 步。
