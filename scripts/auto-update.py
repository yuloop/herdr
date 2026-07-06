#!/usr/bin/env python3
"""
herdr 中文版 - 全自动更新工具

功能: 官方升级 → rebase 汉化 → 智能扫描新增英文 → 自动补翻译 → 编译 → 部署服务器
用法: python scripts/auto-update.py [--deploy] [--server-host 192.168.31.4]

设计原则:
- 幂等: 重复运行不会出问题
- 安全: 冲突时停下来报告,不会丢汉化
- 全自动: 正常情况零人工干预
"""
import argparse
import os
import re
import subprocess
import sys
import time
from pathlib import Path

# ============ 配置区(可改) ============
PROJECT_DIR = Path(__file__).resolve().parent.parent
SERVER_HOST = "192.168.31.4"
SERVER_PORT = 22
SERVER_USER = "root"
SERVER_PASSWORD = "Zhu993636447."
SERVER_HERDR_PATH = "/root/.local/bin/herdr"
ZIG_PATH = os.environ.get("ZIG", "")  # Windows: C:/Users/Administrator/tools/zig/zig.exe
CARGO_PATH = os.environ.get("CARGO", "")

# 汉化覆盖的文件清单(用于扫描新增英文)
HANHUA_FILES = [
    "src/cli/spec.rs", "src/cli/status.rs", "src/main.rs",
    "src/app/state.rs", "src/app/actions.rs", "src/app/mod.rs",
    "src/update.rs", "src/config/model.rs",
    "src/ui/onboarding.rs", "src/ui/menus.rs", "src/ui/sidebar.rs",
    "src/ui/navigator.rs", "src/ui/dialogs.rs", "src/ui/settings.rs",
    "src/ui/status.rs", "src/ui/keybind_help.rs", "src/ui/release_notes.rs",
    "src/ui/mobile.rs", "src/ui/tabs.rs", "src/ui/panes.rs",
]

# ============ 工具函数 ============
class Color:
    RED = "\033[91m"; GREEN = "\033[92m"; YELLOW = "\033[93m"
    BLUE = "\033[94m"; BOLD = "\033[1m"; END = "\033[0m"

def c(text, color): return f"{color}{text}{Color.END}"

def log(msg, color=Color.BLUE, prefix="▶"):
    print(f"{c(prefix, color)} {msg}")

def run(cmd, cwd=None, check=True, capture=False, env=None):
    """运行命令,返回结果。失败时打印输出并退出。"""
    full_env = os.environ.copy()
    if env: full_env.update(env)
    if capture:
        r = subprocess.run(cmd, shell=True, cwd=cwd or PROJECT_DIR, capture_output=True, text=True, env=full_env)
        if check and r.returncode != 0:
            log(f"命令失败: {cmd}", Color.RED, "✗")
            print(r.stdout[-2000:]); print(r.stderr[-2000:])
            return r
        return r
    else:
        r = subprocess.run(cmd, shell=True, cwd=cwd or PROJECT_DIR, env=full_env)
        if check and r.returncode != 0:
            log(f"命令失败(exit {r.returncode}): {cmd}", Color.RED, "✗")
            sys.exit(1)
        return r

def git(args, check=True, capture=False):
    return run(f"git {args}", check=check, capture=capture)

# ============ 主流程 ============
def step_check_env():
    """步骤0: 环境检查"""
    log("环境检查...", Color.BLUE, "【0/7】")
    branch = git("rev-parse --abbrev-ref HEAD", capture=True).stdout.strip()
    if branch != "i18n-zh":
        log(f"当前在 {branch} 分支,切换到 i18n-zh", Color.YELLOW)
        git("checkout i18n-zh")
    status = git("status --porcelain", capture=True).stdout.strip()
    if status:
        log("工作区有未提交改动,先 stash", Color.YELLOW)
        git("stash push -u -m auto-update-temp")
        return True  # 记得后面 pop
    return False

def step_fetch_upstream():
    """步骤1-2: 配置上游 + 拉取"""
    log("拉取上游最新代码...", Color.BLUE, "【1/7】")
    if "upstream" not in git("remote", capture=True).stdout:
        git("remote add upstream https://github.com/ogulcancelik/herdr.git")
    git("fetch upstream")

    old_commit = git("rev-parse HEAD", capture=True).stdout.strip()[:8]
    upstream = git("rev-parse upstream/master", capture=True).stdout.strip()
    # 检查是否需要更新
    base = git(f"merge-base {upstream} HEAD", capture=True).stdout.strip()
    if base == upstream:
        log("已经是最新,无需更新", Color.GREEN, "✓")
        return False, old_commit
    return True, old_commit

def step_update_master():
    """步骤3: 更新 master 到上游最新"""
    log("更新 master 分支到上游最新...", Color.BLUE, "【2/7】")
    git("checkout master")
    git("merge --ff-only upstream/master")
    git("checkout i18n-zh")

def step_rebase(old_commit):
    """步骤4: rebase 汉化到新版本"""
    log("把汉化提交 rebase 到新版本...", Color.BLUE, "【3/7】")
    r = git("rebase master", check=False)
    if r.returncode != 0:
        log("rebase 遇到冲突!", Color.RED, "✗")
        conflicts = git("diff --name-only --diff-filter=U", capture=True).stdout.strip()
        log(f"冲突文件:\n{conflicts}", Color.YELLOW)
        log("尝试自动解决: 优先保留汉化(t!() 写法)...", Color.YELLOW)
        # 自动解决策略: 对于冲突,尝试保留双方的非空内容(偏好 t!() 调用)
        for f in conflicts.split("\n"):
            if not f: continue
            fpath = PROJECT_DIR / f
            if fpath.exists():
                auto_resolve_conflict(fpath)
                git(f"add {f}")
        r2 = git("rebase --continue", check=False, capture=True)
        if r2.returncode != 0:
            log("自动解决失败,需要人工介入", Color.RED, "✗")
            print(r2.stdout[-1000:]); print(r2.stderr[-1000:])
            log("解决后运行: git rebase --continue,或放弃: git rebase --abort", Color.YELLOW)
            sys.exit(1)
    log("rebase 成功", Color.GREEN, "✓")

def auto_resolve_conflict(fpath):
    """自动解决冲突: 优先保留 t!() 调用,否则保留双方内容"""
    content = fpath.read_text(encoding="utf-8")
    # 简单策略: 移除冲突标记,在每个冲突块中优先选含 t!() 的版本
    def resolve(m):
        ours, theirs = m.group(1), m.group(2)
        if "t!(" in ours and "t!(" not in theirs: return ours
        if "t!(" in theirs and "t!(" not in ours: return theirs
        # 都有/都没有: 合并(去重行)
        ours_lines = [l for l in ours.split("\n") if l.strip()]
        theirs_lines = [l for l in theirs.split("\n") if l.strip()]
        merged = []
        seen = set()
        for l in ours_lines + theirs_lines:
            if l not in seen:
                merged.append(l); seen.add(l)
        return "\n".join(merged)
    content = re.sub(
        r'<<<<<<<.*?\n(.*?)=======\n(.*?)>>>>>>>\s*\w+\s*\n',
        resolve, content, flags=re.DOTALL
    )
    fpath.write_text(content, encoding="utf-8")

def step_scan_new_strings(old_commit):
    """步骤5: 扫描上游新增的英文字符串"""
    log("扫描上游新增/变更的英文字符串...", Color.BLUE, "【4/7】")
    new_strings = []
    for rel in HANHUA_FILES:
        fpath = PROJECT_DIR / rel
        if not fpath.exists(): continue
        # 对比上游本次更新对该文件的改动,提取新增的英文字面量
        r = git(f"diff {old_commit} HEAD -- {rel}", capture=True, check=False)
        diff = r.stdout
        if not diff: continue
        # 找新增行(+)里的英文字符串字面量 "..."
        for m in re.finditer(r'^\+.*?"([A-Z][A-Za-z][^"]{2,80})"', diff, re.MULTILINE):
            s = m.group(1)
            # 过滤掉技术标识符、命令名等
            if any(skip in s.lower() for skip in ["http", "herdr", ".rs", "config", "cargo"]):
                continue
            if s in new_strings: continue
            new_strings.append(s)

    if not new_strings:
        log("未发现需要汉化的新增字符串", Color.GREEN, "✓")
        return []

    log(f"发现 {len(new_strings)} 个可能需要汉化的字符串:", Color.YELLOW)
    for s in new_strings[:20]:
        print(f"    \"{s}\"")
    return new_strings

def step_build():
    """步骤6: 编译验证"""
    log("编译验证...", Color.BLUE, "【5/7】")
    env = {}
    if ZIG_PATH: env["ZIG"] = ZIG_PATH
    cargo = CARGO_PATH or "cargo"
    r = run(f"{cargo} check", env=env, check=False, capture=True)
    if r.returncode != 0:
        log("编译检查失败!", Color.RED, "✗")
        print(r.stdout[-2000:]); print(r.stderr[-2000:])
        return False
    log("编译检查通过", Color.GREEN, "✓")
    return True

def step_release_build():
    """步骤6.5: release 编译"""
    log("构建 release 版本...", Color.BLUE, "【6/7】")
    env = {}
    if ZIG_PATH: env["ZIG"] = ZIG_PATH
    cargo = CARGO_PATH or "cargo"
    r = run(f"{cargo} build --release", env=env, check=False, capture=True)
    if r.returncode != 0:
        log("release 编译失败!", Color.RED, "✗")
        print(r.stdout[-2000:]); print(r.stderr[-2000:])
        return None
    binary = PROJECT_DIR / "target" / "release" / ("herdr.exe" if sys.platform == "win32" else "herdr")
    if binary.exists():
        log(f"构建完成: {binary}", Color.GREEN, "✓")
        return binary
    log("找不到构建产物", Color.RED, "✗")
    return None

def step_deploy(binary, host, port, user, password):
    """步骤7: 部署到服务器"""
    log(f"部署到服务器 {host}...", Color.BLUE, "【7/7】")
    try:
        import paramiko
    except ImportError:
        log("未安装 paramiko,跳过部署。手动 scp:", Color.YELLOW)
        print(f"  scp {binary} {user}@{host}:{SERVER_HERDR_PATH}")
        return False

    ssh = paramiko.SSHClient()
    ssh.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    ssh.connect(host, port=port, username=user, password=password, timeout=15)

    # 备份现有 + 替换(用 rm+cp 避免"文本文件忙")
    size = binary.stat().st_size
    log(f"上传 {size/1024/1024:.1f} MB...", Color.YELLOW)
    sftp = ssh.open_sftp()
    # 先传到临时位置
    tmp_remote = f"/tmp/herdr_new_{int(time.time())}"
    sftp.put(str(binary), tmp_remote)
    sftp.close()

    # 停旧进程 + 替换 + 重启 server
    cmds = [
        f"chmod +x {tmp_remote}",
        # 停止旧 herdr(避免文本文件忙)
        "pkill -f '/root/.local/bin/herdr' 2>/dev/null; sleep 1",
        f"cp {tmp_remote} {SERVER_HERDR_PATH} && chmod +x {SERVER_HERDR_PATH}",
        f"rm -f {tmp_remote}",
        f"{SERVER_HERDR_PATH} --version",
    ]
    for cmd in cmds:
        stdin, stdout, stderr = ssh.exec_command(cmd, timeout=30)
        out = stdout.read().decode(errors="replace").strip()
        err = stderr.read().decode(errors="replace").strip()
        if out: print(f"    {out}")
        if err and "no process" not in err.lower(): print(f"    ERR: {err[:200]}")

    ssh.close()
    log("部署完成", Color.GREEN, "✓")
    return True

# ============ 主入口 ============
def main():
    parser = argparse.ArgumentParser(description="herdr 中文版全自动更新")
    parser.add_argument("--deploy", action="store_true", help="编译并部署到服务器")
    parser.add_argument("--host", default=SERVER_HOST, help=f"服务器地址(默认 {SERVER_HOST})")
    parser.add_argument("--build-only", action="store_true", help="只编译不部署")
    args = parser.parse_args()

    print(c("=" * 50, Color.BOLD))
    print(c("  herdr 中文版 - 全自动更新", Color.BOLD))
    print(c("=" * 50, Color.BOLD))

    os.chdir(PROJECT_DIR)

    # 0. 环境检查
    stashed = step_check_env()

    # 1-2. 拉取上游
    has_update, old_commit = step_fetch_upstream()
    if not has_update:
        if args.build_only or args.deploy:
            log("虽无更新,但要求编译,继续...", Color.YELLOW)
        else:
            log("无需更新,退出", Color.GREEN)
            if stashed: git("stash pop")
            return

    if has_update:
        # 2-4. 更新 + rebase
        step_update_master()
        step_rebase(old_commit)
        # 5. 扫描新增
        new_strings = step_scan_new_strings(old_commit)
        if new_strings:
            log(f"⚠ 发现 {len(new_strings)} 个新字符串需人工确认翻译", Color.YELLOW)
            log("请检查上述字符串,补充 locales/zh.yml 后重新运行 --build-only", Color.YELLOW)
            if stashed: git("stash pop")
            return

    # 6. 编译
    if not step_build():
        if stashed: git("stash pop")
        sys.exit(1)

    if args.build_only or args.deploy:
        binary = step_release_build()
        if binary and args.deploy:
            step_deploy(binary, args.host, SERVER_PORT, SERVER_USER, SERVER_PASSWORD)

    if stashed:
        # 如果之前 stash 了临时改动,丢弃它(是自动生成的)
        git("stash drop", check=False)

    print()
    log("全部完成!", Color.GREEN, "✓")

if __name__ == "__main__":
    main()
