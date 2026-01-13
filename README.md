# WinClean Rules Repository

WinClean 清理工具的规则仓库，用于集中管理清理规则，方便社区贡献。

```
winclean_rules/
├── README.md                    # 本文件
├── LICENSE                      # 开源协议
├── rules/                       # 规则目录
│   └── 高危软件/                # 规则分类目录
│       └── *.yaml               # 规则文件
├── src/packer/                  # 规则打包工具源码
├── .github/workflows/           # CI/CD 配置文件
│   └── ci.yml                   # GitHub Actions 工作流
├── dist/                        # 构建输出目录
│   ├── rules.bin                # 二进制规则包
│   └── winclean-rules-packer    # 打包工具
└── Cargo.toml                   # Rust 项目配置
```

## 规则格式

每个规则文件采用 YAML 格式，字段说明：

```yaml
# 规则唯一标识符（建议使用软件拼音或英文名）
id: software_name

# 显示名称
name: 软件名称

# 风险等级: high/default
risk: high

# 支持的系统版本
systeminfo:
  - win10_x64
  - win11_x64

# 最后更新日期
update: 2025-12-29

# 匹配规则
match:
  path:
    - "C:\\Path\\To\\<Clean.*>"  # 正则表达式需用 <> 包裹
    - "%APPDATA%\\<Software.*>"
  registry:
    - path: "HKEY_CURRENT_USER\\Software\\Vendor\\"
      key: "ProductName"
      action: delete_key
```

### 字段详解

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | 是 | 规则唯一标识，小写字母、数字、下划线 |
| `name` | string | 是 | 规则显示名称 |
| `risk` | string | 是 | 风险等级：high/default |
| `systeminfo` | list | 否 | 支持的系统版本列表 |
| `update` | string | 是 | 最后更新日期 (YYYY-MM-DD) |
| `match.path` | list | 否 | 要清理的文件/目录路径列表 |
| `match.registry` | list | 否 | 要清理的注册表项列表 |

### registry 字段详解

| 字段 | 类型 | 说明 |
|------|------|------|
| `path` | string | 注册表路径（不含键名），支持<正则表达式> |
| `key` | string | 要匹配的键名，支持通配符 `*` |
| `value` | string | 要匹配的值名，支持通配符 `*` |
| `value_data` | string | 要匹配的值数据，支持通配符 `*` |
| `action` | string | 操作：delete_key / delete_value / delete_value_data |

## 贡献规则

1. 在 `rules/` 目录下找到对应分类，或创建新分类目录
2. 创建新的 `.yaml` 规则文件
3. 填写规则内容（参考上方格式）
4. 提交 Pull Request

## 构建二进制规则包

### 本地构建

```bash
# 构建打包工具
cargo build --release

# 打包规则
mkdir -p dist
./target/release/winclean-rules-packer pack --input ./rules --output ./dist/rules.bin --compress zstd
```

生成的二进制规则包位于 `dist/` 目录。

### CI/CD 自动构建

项目配置了 GitHub Actions 自动化流水线，具有以下功能：

#### 触发条件

| 事件 | 说明 |
|------|------|
| push 到 `main` 分支且更新 `rules/**/*.yaml` | 当规则文件更新时自动构建 |
| workflow_dispatch | 手动触发构建 |

#### 版本管理

- **日期版本号**: 格式为 `YYYYMMDD`（例如：20260113）
- 每次推送规则更新自动创建新 Release

#### 发布产物

构建完成后会自动：
1. 生成 `dist/rules.bin` 二进制规则包
2. 上传构建产物到 GitHub Artifacts（保留 30 天）
3. 自动创建 GitHub Release（tag: `vYYYYMMDD`）

## 打包工具使用

```bash
# 打包规则
./dist/winclean-rules-packer pack --input ./rules --output ./dist/rules.bin --compress zstd

# 查看规则包信息
./dist/winclean-rules-packer info --input ./dist/rules.bin

# 解压规则包
./dist/winclean-rules-packer unpack --input ./dist/rules.bin --output ./rules_unpacked
```
