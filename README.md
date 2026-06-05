# OrcaSlicer SC12060续接 G-code 后处理插件
![sc12060_continue](https://ts1.tc.mm.bing.net/th/id/R-C.1224123370b7b0d499cd9367bd37caa6?rik=o0F76fFKkmV75Q&riu=http%3a%2f%2forcaslicer.cn%2fpublic%2fdefault%2fdefault%2fimg%2flogo.png&ehk=mUIVv6vjZKO1RqiOz7IR%2fQUChtbO664YPmhUWAiRSuc%3d&risl=&pid=ImgRaw&r=0)
![sc12060](image.png)
![sc12060_continue](https://www.techug.com/wordpress/wp-content/uploads/2024/09/2-1024x576.png-1000x563.webp)

## 项目介绍

这是一个用于 OrcaSlicer 的 G-code 后处理插件，专门为 SC12060 打印机设计的续接打印功能。

### 主要功能

1. **COM8端口错误续接**：出现com8端口错误时，记录打印高度并在OrcaSlicer中使用插件生成后续gcode
2. **断料检测失效出现空打时续接**：量出已打印高度，使用插件
3. **出现停电等意外情况时续接**：量出已打印高度，使用插件
4. **智能排除已打印部分**：支持按 TYPE 块类型排除已完成的打印内容

## 使用方法

### 命令行参数

插件支持以下命令行参数（两两互斥，只能指定一个）：

| 中文参数 | 英文参数 | 对应 G-code TYPE | 说明 |
|----------|----------|------------------|------|
| `--支撑` | `--Support` | `;TYPE:Support` | 排除支撑部分 |
| `--外墙` | `--OuterWall` | `;TYPE:Outer wall` | 排除外墙部分 |
| `--内墙` | `--InnerWall` | `;TYPE:Inner wall` | 排除内墙部分 |
| `--实心填充` | `--SolidInfill` | `;TYPE:Internal solid infill` | 排除实心填充部分 |
| `--稀疏填充` | `--SparseInfill` | `;TYPE:Sparse infill` | 排除稀疏填充部分 |

### 支持的暂停指令

插件能识别以下暂停指令：
- `PAUSE` - 标准暂停指令
- `M601` - Marlin 换料暂停指令
- `CONTINUE` - 继续指令
- `continue` - 小写形式
- `接续` - 中文暂停标识
- `继续` - 中文继续标识

### 方法一：在 OrcaSlicer 中配置（推荐）

1. 打开 OrcaSlicer
2. 进入 "Others" 选项卡
3. 在 "Post-processing Scripts" 中输入可执行文件的完整路径：
   ```
   eg: d:\Users\shucai\Desktop\sc12060_continue\target\release\sc12060_continue.exe
   ```
4. 设置暂停点（在需要暂停的位置添加 PAUSE 指令）
5. 点击导出 G-code，插件会自动处理

**带参数配置示例**（排除已打印的外墙）：
```
eg: d:\Users\shucai\Desktop\sc12060_continue\target\release\sc12060_continue.exe --外墙
```

### 方法二：单独运行

在命令行中执行：

```powershell
# 基础用法：移除 PAUSE 指令，保留完整层结构
target\release\sc12060_continue.exe your_file.gcode

# 排除已打印的外墙部分
target\release\sc12060_continue.exe your_file.gcode --外墙

# 使用英文参数
target\release\sc12060_continue.exe your_file.gcode --OuterWall

# 排除已打印的支撑
target\release\sc12060_continue.exe your_file.gcode --支撑
```

### 实际应用场景

**场景一：换料后继续打印**
```powershell
# 打印外墙时耗材耗尽，更换耗材后排除外墙部分
sc12060_continue.exe test.gcode --外墙
```

**场景二：排除多个已打印类型**
如果打印顺序是 `外墙 → 内墙 → 实心填充 → 稀疏填充 → PAUSE`，且外墙和内墙已经打印完成：
```powershell
# 指定最后一个已完成的类型，会删除从第一个 TYPE 到指定 TYPE 的所有内容
sc12060_continue.exe test.gcode --内墙
```

**场景三：仅移除暂停指令**
```powershell
# 不指定任何类型参数，仅移除 PAUSE 指令
sc12060_continue.exe test.gcode
```

## 项目结构

```
sc12060_continue/
├── src/
│   └── main.rs              # 主程序源码
├── target/
│   └── release/
│       └── sc12060_continue.exe  # 编译后的可执行文件
├── Cargo.toml               # Rust 项目配置
├── test.gcode               # 测试用 G-code 文件
└── README.md                # 本文件
```

## 编译项目

如果你需要自己编译项目：

```powershell
# 编译发布版本
cargo build --release

# 编译后的文件位于 target/release/sc12060_continue.exe
```

## 要求

- Windows、Mac、Linux 操作系统均可
- OrcaSlicer 2.x（如果用于后处理）
- Rust 编译工具链（官网下载安装即可）

## 工作原理

1. 读取 G-code 文件
2. 定位关键标记：
   - `EXECUTABLE_BLOCK_START`：执行块开始
   - `LAYER_CHANGE`：层切换标记
   - `PAUSE/M601/CONTINUE/continue/接续/继续`：暂停指令
3. 定位 PAUSE 所在层（前一个 LAYER_CHANGE 到后一个 LAYER_CHANGE 之间）
4. 如果指定了类型参数，扫描该层内所有 TYPE 块，删除从第一个 TYPE 到目标 TYPE 的内容
5. 移除 PAUSE 指令
6. 输出处理后的文件

## 许可证

本项目使用 MIT 许可证。