#!/usr/bin/env bash
# herdr 中文版 - 官方升级时重新汉化的更新脚本
#
# 用法:
#   cd /path/to/herdr
#   bash scripts/update-from-upstream.sh
#
# 本脚本假设你已经在 i18n-zh 分支上(汉化分支)。
# 它会自动: 拉取上游 → 把汉化 rebase 到新版本 → 提示你处理冲突/新增翻译 → 编译验证
#
# 详细原理见 I18N.md

set -e

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_DIR"

# 颜色输出
red()    { echo -e "\033[31m$*\033[0m"; }
green()  { echo -e "\033[32m$*\033[0m"; }
yellow() { echo -e "\033[33m$*\033[0m"; }
blue()   { echo -e "\033[34m$*\033[0m"; }

echo "================================================"
blue "  herdr 中文版 - 官方升级更新脚本"
echo "================================================"
echo ""

# ============ 第0步: 环境检查 ============
blue "【0/6】环境检查..."

# 必须在 i18n-zh 分支
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [ "$CURRENT_BRANCH" != "i18n-zh" ]; then
  red "错误: 当前在 $CURRENT_BRANCH 分支,必须在 i18n-zh 分支运行此脚本"
  echo "请先执行: git checkout i18n-zh"
  exit 1
fi

# 工作区必须干净
if [ -n "$(git status --porcelain)" ]; then
  red "错误: 工作区有未提交改动,请先提交或 stash"
  git status --short
  exit 1
fi

# 检查 zig(编译需要)
if ! command -v zig >/dev/null 2>&1 && [ -z "$ZIG" ]; then
  yellow "警告: 未找到 zig。编译步骤需要 zig 0.15.2"
  echo "  Linux/macOS: export ZIG=/path/to/zig"
  echo "  或把 zig 加入 PATH"
fi

green "环境检查通过"
echo ""

# ============ 第1步: 添加上游远程(仅首次) ============
blue "【1/6】配置上游远程..."

if ! git remote get-url upstream >/dev/null 2>&1; then
  git remote add upstream https://github.com/ogulcancelik/herdr.git
  green "已添加 upstream 远程"
else
  echo "upstream 远程已存在,跳过"
fi
echo ""

# ============ 第2步: 拉取上游最新代码 ============
blue "【2/6】拉取上游最新代码..."
git fetch upstream
echo ""

# 记录更新前的版本
OLD_HEAD=$(git rev-parse HEAD)
UPSTREAM_MASTER=$(git rev-parse upstream/master)

if [ "$OLD_HEAD" = "$UPSTREAM_MASTER" ] || git merge-base --is-ancestor "$UPSTREAM_MASTER" HEAD; then
  green "已经是最新,无需更新"
  exit 0
fi

yellow "检测到上游有新版本:"
echo "  当前基于: $(git log --oneline -1 HEAD~1 | cut -d' ' -f2-)"
echo "  上游最新: $(git log --oneline -1 upstream/master | cut -d' ' -f2-)"
echo ""

# ============ 第3步: 更新本地 master 分支 ============
blue "【3/6】更新本地 master 分支(快进到上游)..."
git checkout master
git merge --ff-only upstream/master
green "master 已更新到上游最新"
git checkout i18n-zh
echo ""

# ============ 第4步: rebase 汉化分支到新 master ============
blue "【4/6】把汉化提交 rebase 到新版本..."
echo ""
echo "这一步可能产生冲突(如果上游改了你汉化过的同一行代码)。"
echo "常见冲突文件: src/cli/spec.rs, src/ui/*.rs, src/main.rs 等"
echo ""

if git rebase master; then
  green "rebase 成功,无冲突!"
else
  red "================================================"
  red "  rebase 遇到冲突,需要你手动解决"
  red "================================================"
  echo ""
  yellow "冲突文件:"
  git diff --name-only --diff-filter=U
  echo ""
  echo "解决步骤:"
  echo "  1. 打开上述文件,找到 <<<<<<< 标记的冲突段"
  echo "  2. 保留你的 t!() 写法(中文翻译),合并上游的结构改动"
  echo "  3. 如果上游新增了英文字符串,改成 t!(\"键名\") 并在 locales/{en,zh}.yml 加翻译"
  echo "  4. 解决完所有冲突后,执行:"
  echo "       git add ."
  echo "       git rebase --continue"
  echo "  5. 如果实在搞不定,放弃 rebase:"
  echo "       git rebase --abort"
  echo "       (回到 rebase 前的状态,汉化不会丢)"
  echo ""
  echo "参考文档: I18N.md 第三节(上游更新维护流程)"
  exit 1
fi
echo ""

# ============ 第5步: 检查新增/变更的英文字符串 ============
blue "【5/6】扫描上游新增的英文字符串..."
echo ""
echo "对比上游本次更新改动了哪些可能需要汉化的文件:"
echo ""

# 找出上游在本次更新中改动的、且属于汉化范围的文件
HANHUA_FILES="src/cli/spec.rs src/cli/status.rs src/main.rs src/app/state.rs src/app/actions.rs src/app/mod.rs src/update.rs src/config/model.rs"
HANHUA_FILES="$HANHUA_FILES src/ui/onboarding.rs src/ui/menus.rs src/ui/sidebar.rs src/ui/navigator.rs src/ui/dialogs.rs src/ui/settings.rs src/ui/status.rs src/ui/keybind_help.rs src/ui/release_notes.rs src/ui/mobile.rs"

echo "本次上游更新涉及的可汉化文件:"
UPDATED=""
for f in $HANHUA_FILES; do
  if git diff --quiet "$OLD_HEAD" HEAD -- "$f" 2>/dev/null; then
    : # 无变化
  else
    if [ -f "$f" ]; then
      echo "  - $f (有变化,检查是否有新增英文)"
      UPDATED="$UPDATED $f"
    fi
  fi
done

if [ -z "$UPDATED" ]; then
  green "上游本次更新未触及汉化文件,你的翻译完全适用,无需补充!"
else
  echo ""
  yellow "请人工检查上述文件:"
  echo "  grep -n '\"[A-Z][a-z]' <文件>   # 找新增的英文字符串"
  echo "  发现新增英文 → 改成 t!(\"键名\") 并补 locales/{en,zh}.yml"
fi
echo ""

# ============ 第6步: 编译验证 ============
blue "【6/6】编译验证..."

export ZIG="${ZIG:-zig}"
if ! command -v cargo >/dev/null 2>&1; then
  yellow "未找到 cargo,跳过编译验证"
  echo "请手动执行: ZIG=/path/to/zig cargo build --release"
else
  echo "运行 cargo check (快速语法检查)..."
  if cargo check 2>&1 | tail -5; then
    green "编译检查通过!"
    echo ""
    echo "如果要构建发布版: cargo build --release"
    echo "如果要部署到服务器: 把 target/release/herdr 传到服务器替换 /root/.local/bin/herdr"
  else
    red "编译失败,请检查上面的错误信息"
    echo "常见原因: t!() 键名拼写错误 / locales yml 格式错误 / 类型不匹配"
    exit 1
  fi
fi
echo ""

echo "================================================"
green "  更新完成!"
echo "================================================"
echo ""
echo "后续步骤:"
echo "  1. 如果改了代码,提交: git add -A && git commit --amend"
echo "  2. 构建: cargo build --release"
echo "  3. 部署服务器: scp target/release/herdr root@192.168.31.4:/root/.local/bin/herdr"
echo "     (或用 paramiko 上传)"
echo ""
echo "详细维护说明见: I18N.md"
