# StockQ - 即時股票追蹤看板

Rust + ratatui 打造的終端機股票追蹤器，支援台股與美股即時報價。

## 功能

- 即時更新報價（可自訂秒數）
- 紅綠燈漲跌視覺化
- 設定選單：調整更新頻率、新增/刪除追蹤股票
- 雙資料源：CMoney + Yahoo Finance 自動切換

## 檔案結構

```
StockQ/
├── launch.bat                  ← 雙擊啟動選單
├── taiwan_stock/
│   ├── stock_tracker.exe       ← 台股追蹤器
│   └── config.json             ← 台股設定（改這裡）
└── us_stock/
    ├── us_stock_tracker.exe    ← 美股追蹤器
    └── config.json             ← 美股設定（改這裡）
```

## 使用方式

### 直接執行（免安裝）

1. 下載 zip 或 clone 此 repo
2. 雙擊 `launch.bat`
3. 按 `1` 台股、`2` 美股

### 從原始碼編譯

需要安裝 [Rust](https://rustup.rs/)

```bash
# 台股
cd taiwan_stock/stock_tracker_ui
cargo build --release

# 美股
cd us_stock/us_stock_tracker_ui
cargo build --release
```

## 操作方式

| 按鍵 | 功能 |
|------|------|
| `↑` `↓` | 選取股票 |
| `S` | 進入設定 |
| `Q` | 離開 |
| `A` | 新增股票 |
| `D` | 刪除股票 |
| `ESC` | 返回 |

## 設定檔

修改 `config.json` 自訂追蹤清單與更新頻率：

```json
{
  "interval": 10,
  "watchlist": [
    { "id": "2330", "name": "台積電" },
    { "id": "TSLA", "name": "特斯拉" }
  ]
}
```

- `interval`：更新秒數（1-300）
- `watchlist`：追蹤股票清單
  - 台股：輸入股票代號（如 `2330`）
  - 美股：輸入 ticker symbol（如 `TSLA`）

## 資料來源

| 市場 | 主要來源 | 備用來源 |
|------|---------|---------|
| 台股 | CMoney | TWSE MIS |
| 美股 | CMoney US | Yahoo Finance |

## 技術棧

- **語言**：Rust
- **UI**：ratatui + crossterm
- **非同步**：tokio
- **HTTP**：reqwest
- **序列化**：serde + serde_json

## License

MIT
