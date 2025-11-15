#!/bin/bash

# Git工作量统计脚本
# 用法: ./git_stats.sh [起始commit] [结束commit]
# 如果不指定结束commit，默认使用HEAD

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 参数处理
START_COMMIT=${1:-"2e0c6e"}
END_COMMIT=${2:-"HEAD"}

echo -e "${BLUE}Git 工作量统计报告${NC}"
echo -e "${CYAN}========================${NC}"
echo -e "统计范围: ${GREEN}$START_COMMIT${NC} 到 ${GREEN}$END_COMMIT${NC}"
echo ""

# 检查commit是否存在
if ! git rev-parse --verify "$START_COMMIT" >/dev/null 2>&1; then
    echo -e "${RED}错误: 起始commit '$START_COMMIT' 不存在${NC}"
    exit 1
fi

if ! git rev-parse --verify "$END_COMMIT" >/dev/null 2>&1; then
    echo -e "${RED}错误: 结束commit '$END_COMMIT' 不存在${NC}"
    exit 1
fi

# 1. 提交数量统计
echo -e "${YELLOW}1. 提交统计${NC}"
COMMIT_COUNT=$(git rev-list --count "$START_COMMIT..$END_COMMIT")
echo -e "提交数量: ${GREEN}$COMMIT_COUNT${NC} 个 commits"

# 获取时间范围
START_DATE=$(git log -1 --format=%ci "$START_COMMIT")
END_DATE=$(git log -1 --format=%ci "$END_COMMIT")
echo -e "时间范围: $START_DATE 到 $END_DATE"
echo ""

# 2. 文件变更统计
echo -e "${YELLOW}2. 文件变更统计${NC}"
FILE_STATS=$(git diff --numstat "$START_COMMIT..$END_COMMIT")
FILES_CHANGED=$(echo "$FILE_STATS" | wc -l)
echo -e "变更文件数: ${GREEN}$FILES_CHANGED${NC} 个文件"

# 文件类型统计
echo -e "\n文件类型分布:"
echo "$FILE_STATS" | awk '{
    filename = $3
    # 提取文件扩展名
    if (match(filename, /\.([^.]+)$/)) {
        ext = substr(filename, RSTART + 1, RLENGTH - 1)
    } else {
        ext = "无扩展名"
    }
    additions[ext] += $1
    deletions[ext] += $2
    files[ext] += 1
}
END {
    for (ext in files) {
        printf "  %-10s: %3d 个文件 (%+d/%-d 行)\n", ext, files[ext], additions[ext], deletions[ext]
    }
}' | sort -k2 -nr
echo ""

# 3. 代码行数统计
echo -e "${YELLOW}3. 代码行数统计${NC}"
TOTAL_ADDITIONS=$(echo "$FILE_STATS" | awk '{sum += $1} END {print sum+0}')
TOTAL_DELETIONS=$(echo "$FILE_STATS" | awk '{sum += $2} END {print sum+0}')
NET_CHANGE=$((TOTAL_ADDITIONS - TOTAL_DELETIONS))

echo -e "新增行数: ${GREEN}+$TOTAL_ADDITIONS${NC}"
echo -e "删除行数: ${RED}-$TOTAL_DELETIONS${NC}"
if [ $NET_CHANGE -ge 0 ]; then
    echo -e "净变化: ${GREEN}+$NET_CHANGE${NC}"
else
    echo -e "净变化: ${RED}$NET_CHANGE${NC}"
fi

# 4. 作者统计
echo -e "\n${YELLOW}4. 作者贡献统计${NC}"
git log --format="%an" "$START_COMMIT..$END_COMMIT" | sort | uniq -c | sort -nr | head -10 | \
awk '{
    printf "  %-20s: %3d commits\n", $2, $1
}'
echo ""

# 5. 每日提交活动
echo -e "${YELLOW}5. 每日提交活动${NC}"
git log --format="%ad" --date=short "$START_COMMIT..$END_COMMIT" | sort | uniq -c | tail -10 | \
awk '{
    printf "  %-12s: %3d commits\n", $2, $1
}' | tac
echo ""


echo ""
echo -e "${CYAN}========================${NC}"
echo -e "${BLUE}统计完成！${NC}"
echo -e "生成时间: $(date '+%Y-%m-%d %H:%M:%S')"