# 台股資料抓取工具

用於抓取台灣股市資料的 Python 套件，支援即時報價、歷史K線、基本面、營收、除權息等資料。

## 功能

- **即時報價**: 取得個股或全部上市股票的即時價格、漲跌幅、成交量
- **歷史K線**: 取得日K線資料，支援多月份查詢
- **基本面**: 取得本益比、股價淨值比等基本面指標
- **營收資料**: 取得月營收、年增率等資料
- **除權息**: 取得歷史除權息資料

## 安裝

```bash
pip install -r requirements.txt
```

## 使用方法

### 作為套件使用

```python
from taiwan_stock import TaiwanStock

stock = TaiwanStock()

# 即時報價
quote = stock.get_realtime_quote("2330")
print(quote)

# 歷史K線
df = stock.get_history("2330", 2024, 1)
print(df)

# 多月份K線
df = stock.get_multi_month_history("2330", months=6)

# 基本面
fundamentals = stock.get_fundamentals("2330")

# 匯出CSV
stock.to_csv(df, "2330_history.csv")
```

### 命令列使用

```bash
# 即時報價
python example.py 2330 realtime

# 歷史K線
python example.py 2330 history

# 多月份K線 (自動匯出CSV)
python example.py 2330 multi

# 基本面
python example.py 2330 fundamental

# 營收
python example.py 2330 revenue

# 除權息
python example.py 2330 dividend

# 搜尋股票
python example.py 台積電 search
```

## 資料來源

- 台灣證券交易所 (TWSE)
- Goodinfo 台灣股市資訊網

## 注意事項

- 資料僅供參考，請以官方公告為準
- 請遵守網站使用條款，避免過度頻繁請求
- 建議請求間隔 0.5 秒以上
