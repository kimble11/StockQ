from taiwan_stock import TaiwanStock

def test_basic():
    stock = TaiwanStock()
    
    print("=== 測試即時報價 ===")
    result = stock.get_realtime_quote("2330")
    print(result)
    print()
    
    print("=== 測試歷史K線 ===")
    df = stock.get_history("2330")
    if not df.empty:
        print(f"取得 {len(df)} 筆資料")
        print(df.head())
    else:
        print("無資料 (可能非交易時間)")
    print()
    
    print("=== 測試搜尋功能 ===")
    results = stock.search_stock("台積")
    print(f"找到 {len(results)} 個結果")
    for r in results[:5]:
        print(f"  {r['股票代號']}: {r['股票名稱']}")
    
    print("\n測試完成!")

if __name__ == "__main__":
    test_basic()
