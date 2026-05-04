## 用 mkline 软连接将 C 盘的文件夹移动到其他盘

### mklink 命令参考

```
MKLINK [[/D] | [/H] | [/J]] <链接名称> <目标路径>
```

- **`<链接名称>`**：要创建的”入口”或”快捷方式”的完整路径（例如 `C:\目标文件夹`）
- **`<目标路径>`**：文件或文件夹实际存放的真实路径（例如 `D:\真实文件夹`）

| 参数 | 链接类型 | 作用对象 | 特点与适用场景 |
|------|----------|----------|----------------|
| **(无参数)** | **文件符号链接** | 单个文件 | 创建一个指向单个文件的链接，效果类似于文件的”快捷方式” |
| **`/D`** | **目录符号链接** | 文件夹 | **最常用的类型**。创建一个指向文件夹的链接，对应用和用户来说就像真实文件夹。支持相对路径，甚至可以链接到网络位置 |
| **`/J`** | **目录联接** | 文件夹 | 另一种文件夹链接，功能上略有不同。要求使用**绝对路径**。对于移动 `C:\Users` 下配置文件夹这类常见需求，`/D` 通常是更稳妥的选择 |
| **`/H`** | **硬链接** | 单个文件 | 为单个文件创建”别名”，与原文件完全平等，删除任意一个不影响另一个。**不能跨硬盘分区使用** |

---

### 使用 mkline 快捷工具

<img width="678" height="525" alt="Pasted image 20260504215908" src="https://github.com/user-attachments/assets/adf7d89f-cd70-44bd-b4a0-2c2fd00e273c" />
该工具提供可视化界面，可以方便快捷地迁移目录。

#### 操作说明

<img width="677" height="525" alt="Pasted image 20260504221125" src="https://github.com/user-attachments/assets/79803ebd-a5e3-4e12-8c48-b9e5b01f56e5" />
1. **选择源目录**：可选择多个源目录进行迁移
2. **迁移过程**：迁移完成后，原位置会生成一个软连接，其他软件访问该软连接时会自动跳转到目标目录
3. **错误处理**：迁移过程中如果出现错误，会立即停止，删除已迁移的文件，同时恢复源文件
4. **备份功能**：支持备份源文件（点击”备份”会生成 `原文件名(1)`）

#### 示例：迁移 `C:\Users\Administrator\test6`

1. **源目录**：`C:\Users\Administrator\test6` <img width="381" height="112" alt="Pasted image 20260504221858" src="https://github.com/user-attachments/assets/cd1aa62c-40b6-4980-b747-ca0bad37312f" />
2. **目标目录**：`E:\User` <img width="347" height="180" alt="Pasted image 20260504222105" src="https://github.com/user-attachments/assets/c5b6edd7-03ec-422f-9323-83b8cea566b9" />
3. **填写示例**： <img width="671" height="403" alt="Pasted image 20260504222253" src="https://github.com/user-attachments/assets/64336434-6b8f-4d25-8759-3161bac3b6e4" />
4. **点击确定**：
   - 成功提示：
      <img width="667" height="409" alt="Pasted image 20260504222318" src="https://github.com/user-attachments/assets/31d0313c-2aba-4b2b-b27f-629138b7f0a8" />

   - 源目录变为软连接，指向 `E:\User\test6`：
      <img width="390" height="103" alt="Pasted image 20260504222500" src="https://github.com/user-attachments/assets/d9d2e5d4-ecf8-453c-972d-ef7ff2af75f4" />

   - 目标目录下出现 `test6`（迁移的目录）：
      <img width="373" height="209" alt="Pasted image 20260504222615" src="https://github.com/user-attachments/assets/0a4ed4b1-878b-4de1-8161-3e83864a906e" />

> **注意**：一般只用来迁移**文件夹**。迁移文件虽然也能创建软连接，但文件的软连接存在问题，轻易不要尝试。

