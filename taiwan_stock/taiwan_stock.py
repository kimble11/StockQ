import requests
from bs4 import BeautifulSoup
import pandas as pd
from datetime import datetime, timedelta
import json
import time
import re

class TaiwanStock:
    TWSE_URL = "https://www.twse.com.tw"
    TWSE_API_URL = "https://www.twse.com.tw/exchangeReport"
    GOODINFO_URL = "https://goodinfo.tw"

    HEADERS = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        "Accept": "application/json, text/plain, */*",
        "Accept-Language": "zh-TW,zh;q=0.9,en;q=0.8",
    }

    def __init__(self):
        self.session = requests.Session()
        self.session.headers.update(self.HEADERS)
        self._cache = {}

    def _fetch_stock_table(self, date_str: str = None, force_refresh: bool = False) -> list:
        """取得個股交易資料表 (含快取與自動回溯)"""
        if date_str is None:
            date_str = datetime.now().strftime("%Y%m%d")

        cache_key = f"stock_table_{date_str}"
        if not force_refresh and cache_key in self._cache:
            return self._cache[cache_key]

        url = f"{self.TWSE_URL}/rwd/zh/afterTrading/MI_INDEX"

        for offset in range(7):
            try_date = datetime.strptime(date_str, "%Y%m%d") - timedelta(days=offset)
            try_date_str = try_date.strftime("%Y%m%d")

            try:
                resp = self.session.get(url, params={
                    "response": "json",
                    "date": try_date_str,
                    "type": "ALLBUT0999",
                }, timeout=10)
                resp.raise_for_status()
                data = resp.json()

                tables = data.get("tables", [])
                if len(tables) > 8:
                    rows = tables[8].get("data", [])
                    if rows:
                        self._cache[cache_key] = rows
                        self._cache[f"stock_table_date_{date_str}"] = try_date_str
                        return rows
            except Exception:
                continue

        return []

    def get_realtime_quote(self, stock_id: str) -> dict:
        """取得個股即時報價"""
        try:
            rows = self._fetch_stock_table()
            actual_date = self._cache.get("stock_table_date_", "未知")

            for row in rows:
                if row[0] == stock_id:
                    return {
                        "股票代號": row[0],
                        "股票名稱": row[1],
                        "成交股數": row[2],
                        "成交筆數": row[3],
                        "成交金額": row[4],
                        "開盤價": row[5],
                        "最高價": row[6],
                        "最低價": row[7],
                        "收盤價": row[8],
                        "漲跌": row[9],
                        "漲跌價差": row[10],
                        "最後揭示買價": row[11],
                        "最後揭示買量": row[12],
                        "最後揭示賣價": row[13],
                        "最後揭示賣量": row[14],
                        "資料日期": actual_date,
                        "時間": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
                    }
            return {"error": f"找不到 {stock_id} 的即時報價"}
        except Exception as e:
            return {"error": str(e)}

    def get_all_realtime_quotes(self) -> pd.DataFrame:
        """取得所有上市股票即時報價"""
        try:
            rows = self._fetch_stock_table()
            records = []
            for row in rows:
                records.append({
                    "股票代號": row[0],
                    "股票名稱": row[1],
                    "成交股數": row[2],
                    "收盤價": row[8],
                    "漲跌": row[9],
                    "漲跌價差": row[10],
                })
            return pd.DataFrame(records)
        except Exception as e:
            print(f"取得即時報價失敗: {e}")
            return pd.DataFrame()

    def get_history(self, stock_id: str, year: int = None, month: int = None) -> pd.DataFrame:
        """取得個股歷史K線資料"""
        if year is None or month is None:
            now = datetime.now()
            year = now.year
            month = now.month

        tw_year = year - 1911
        url = f"{self.TWSE_API_URL}/STOCK_DAY"
        params = {
            "response": "json",
            "date": f"{year}{month:02d}01",
            "stockNo": stock_id,
        }

        try:
            resp = self.session.get(url, params=params, timeout=10)
            resp.raise_for_status()
            data = resp.json()

            if "data" in data and data["data"]:
                records = []
                for row in data["data"]:
                    date_str = row[0].replace(f"{tw_year}/", f"{year}/")
                    records.append({
                        "日期": date_str,
                        "成交股數": row[1],
                        "成交金額": row[2],
                        "開盤價": row[3],
                        "最高價": row[4],
                        "最低價": row[5],
                        "收盤價": row[6],
                        "漲跌價差": row[7],
                        "成交筆數": row[8],
                    })
                return pd.DataFrame(records)
            return pd.DataFrame()
        except Exception as e:
            print(f"取得歷史資料失敗: {e}")
            return pd.DataFrame()

    def get_multi_month_history(self, stock_id: str, months: int = 6) -> pd.DataFrame:
        """取得多月份歷史K線資料"""
        all_data = []
        now = datetime.now()

        for i in range(months):
            target = now - timedelta(days=30 * i)
            df = self.get_history(stock_id, target.year, target.month)
            if not df.empty:
                all_data.append(df)
            time.sleep(0.5)

        if all_data:
            return pd.concat(all_data, ignore_index=True)
        return pd.DataFrame()

    def get_fundamentals(self, stock_id: str) -> dict:
        """取得個股基本面資料"""
        url = f"{self.GOODINFO_URL}/StockInfo/StockDetail.asp"
        params = {"STOCK_ID": stock_id}

        try:
            resp = self.session.get(url, params=params, timeout=10)
            resp.raise_for_status()
            resp.encoding = "utf-8"
            soup = BeautifulSoup(resp.text, "lxml")

            result = {"股票代號": stock_id}

            tables = soup.find_all("table")
            for table in tables:
                rows = table.find_all("tr")
                for row in rows:
                    cells = row.find_all(["td", "th"])
                    for i in range(len(cells) - 1):
                        key = cells[i].get_text(strip=True)
                        value = cells[i + 1].get_text(strip=True)
                        if key and value:
                            result[key] = value

            return result
        except Exception as e:
            return {"error": str(e)}

    def get_financial_statements(self, stock_id: str) -> pd.DataFrame:
        """取得個股財報資料"""
        url = f"{self.GOODINFO_URL}/StockInfo/StockFinReport.asp"
        params = {"STOCK_ID": stock_id}

        try:
            resp = self.session.get(url, params=params, timeout=10)
            resp.raise_for_status()
            resp.encoding = "utf-8"
            soup = BeautifulSoup(resp.text, "lxml")

            tables = soup.find_all("table", class_="table_4")
            if tables:
                df = pd.read_html(str(tables[0]))[0]
                return df
            return pd.DataFrame()
        except Exception as e:
            print(f"取得財報失敗: {e}")
            return pd.DataFrame()

    def get_revenue(self, stock_id: str) -> pd.DataFrame:
        """取得個股營收資料"""
        url = f"{self.GOODINFO_URL}/StockInfo/StockRevenue.asp"
        params = {"STOCK_ID": stock_id}

        try:
            resp = self.session.get(url, params=params, timeout=10)
            resp.raise_for_status()
            resp.encoding = "utf-8"
            soup = BeautifulSoup(resp.text, "lxml")

            tables = soup.find_all("table", class_="table_4")
            if tables:
                df = pd.read_html(str(tables[0]))[0]
                return df
            return pd.DataFrame()
        except Exception as e:
            print(f"取得營收失敗: {e}")
            return pd.DataFrame()

    def get_dividend_history(self, stock_id: str) -> pd.DataFrame:
        """取得個股除權息資料"""
        url = f"{self.GOODINFO_URL}/StockInfo/StockDividend.asp"
        params = {"STOCK_ID": stock_id}

        try:
            resp = self.session.get(url, params=params, timeout=10)
            resp.raise_for_status()
            resp.encoding = "utf-8"
            soup = BeautifulSoup(resp.text, "lxml")

            tables = soup.find_all("table", class_="table_4")
            if tables:
                df = pd.read_html(str(tables[0]))[0]
                return df
            return pd.DataFrame()
        except Exception as e:
            print(f"取得除權息資料失敗: {e}")
            return pd.DataFrame()

    def search_stock(self, keyword: str) -> list:
        """搜尋股票"""
        try:
            rows = self._fetch_stock_table()
            results = []
            for row in rows:
                if keyword.lower() in row[0].lower() or keyword.lower() in row[1].lower():
                    results.append({
                        "股票代號": row[0],
                        "股票名稱": row[1],
                    })
            return results
        except Exception as e:
            return []

    def to_csv(self, df: pd.DataFrame, filename: str):
        """將 DataFrame 匯出為 CSV"""
        df.to_csv(filename, index=False, encoding="utf-8-sig")
        print(f"已匯出至 {filename}")

    def to_excel(self, df: pd.DataFrame, filename: str):
        """將 DataFrame 匯出為 Excel"""
        df.to_excel(filename, index=False, engine="openpyxl")
        print(f"已匯出至 {filename}")
