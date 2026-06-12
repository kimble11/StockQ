from taiwan_stock import TaiwanStock
import sys

def main():
    stock = TaiwanStock()
    
    if len(sys.argv) < 2:
        print("使用方法:")
        print("  python example.py <股票代號> [功能]")
        print()
        print("功能選項:")
        print("  realtime  - 即時報價")
        print("  history   - 歷史K線")
        print("  multi     - 多月份K線")
        print("  fundamental - 基本面")
        print("  revenue   - 營收")
        print("  dividend  - 除權息")
        print("  search    - 搜尋股票")
        print()
        print("範例:")
        print("  python example.py 2330 realtime")
        print("  python example.py 2330 history")
        print("  python example.py 台積電 search")
        return

    stock_id = sys.argv[1]
    func = sys.argv[2] if len(sys.argv) > 2 else "realtime"

    if func == "realtime":
        print(f"\n=== {stock_id} 即時報價 ===")
        result = stock.get_realtime_quote(stock_id)
        for k, v in result.items():
            print(f"{k}: {v}")

    elif func == "history":
        print(f"\n=== {stock_id} 本月歷史K線 ===")
        df = stock.get_history(stock_id)
        if not df.empty:
            print(df.to_string(index=False))
        else:
            print("無資料")

    elif func == "multi":
        print(f"\n=== {stock_id} 近6個月歷史K線 ===")
        df = stock.get_multi_month_history(stock_id, months=6)
        if not df.empty:
            print(df.to_string(index=False))
            stock.to_csv(df, f"{stock_id}_history.csv")
        else:
            print("無資料")

    elif func == "fundamental":
        print(f"\n=== {stock_id} 基本面資料 ===")
        result = stock.get_fundamentals(stock_id)
        for k, v in result.items():
            print(f"{k}: {v}")

    elif func == "revenue":
        print(f"\n=== {stock_id} 營收資料 ===")
        df = stock.get_revenue(stock_id)
        if not df.empty:
            print(df.to_string(index=False))
        else:
            print("無資料")

    elif func == "dividend":
        print(f"\n=== {stock_id} 除權息資料 ===")
        df = stock.get_dividend_history(stock_id)
        if not df.empty:
            print(df.to_string(index=False))
        else:
            print("無資料")

    elif func == "search":
        print(f"\n=== 搜尋: {stock_id} ===")
        results = stock.search_stock(stock_id)
        if results:
            for r in results:
                print(f"{r['股票代號']}: {r['股票名稱']}")
        else:
            print("找不到相關股票")

    else:
        print(f"未知功能: {func}")

if __name__ == "__main__":
    main()
