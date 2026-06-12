import sys
import os
import json
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from taiwan_stock_realtime import TaiwanStockRealtime
from rich.console import Console
from rich.table import Table
from rich.panel import Panel
from datetime import datetime
import time
import argparse

CONFIG_FILE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "config.json")

def load_config():
    if os.path.exists(CONFIG_FILE):
        with open(CONFIG_FILE, "r", encoding="utf-8") as f:
            return json.load(f)
    return {"interval": 10, "watchlist": []}

DEFAULT_WATCHLIST = load_config().get("watchlist", [])

class StockTracker:
    def __init__(self, watchlist=None, interval=10):
        self.stock = TaiwanStockRealtime()
        self.watchlist = watchlist or DEFAULT_WATCHLIST
        self.interval = interval
        self.console = Console()
        self.last_prices = {}

    def fetch_quotes(self):
        codes = [item["id"] for item in self.watchlist]
        quotes = self.stock.fetch_all(codes)

        results = []
        for item in self.watchlist:
            stock_id = item["id"]
            stock_name = item["name"]
            q = quotes.get(stock_id)

            if q:
                results.append({
                    "id": stock_id,
                    "name": stock_name,
                    "price": q.get("price"),
                    "change": q.get("change"),
                    "pct": q.get("pct"),
                    "prev": q.get("prev"),
                    "open": q.get("open"),
                    "high": q.get("high"),
                    "low": q.get("low"),
                    "volume": q.get("volume"),
                    "time": q.get("time"),
                    "source": q.get("source"),
                })
            else:
                results.append({
                    "id": stock_id,
                    "name": stock_name,
                    "price": None,
                    "change": None,
                    "pct": None,
                    "prev": None,
                    "open": None,
                    "high": None,
                    "low": None,
                    "volume": None,
                    "time": None,
                    "source": None,
                })

        return results

    def build_table(self, data):
        table = Table(
            title="[bold cyan]台股即時追蹤看板[/bold cyan]",
            show_header=True,
            header_style="bold white on blue",
            border_style="bright_blue",
            expand=True,
        )

        table.add_column("燈號", justify="center", width=5)
        table.add_column("代號", justify="center", width=7)
        table.add_column("名稱", justify="left", width=14)
        table.add_column("現價", justify="right", width=10)
        table.add_column("漲跌", justify="right", width=8)
        table.add_column("漲跌幅", justify="right", width=8)
        table.add_column("昨收", justify="right", width=9)
        table.add_column("開盤", justify="right", width=9)
        table.add_column("最高", justify="right", width=9)
        table.add_column("最低", justify="right", width=9)
        table.add_column("成交量(張)", justify="right", width=10)
        table.add_column("資料來源", justify="center", width=12)

        for item in data:
            price = item["price"]
            change = item["change"]
            pct = item["pct"]

            if price is None:
                lamp = "[white]⚪[/white]"
                price_str = "[dim]N/A[/dim]"
                change_str = "[dim]N/A[/dim]"
                pct_str = "[dim]N/A[/dim]"
            else:
                if change is not None and change > 0:
                    lamp = "[bold red]🔴[/bold red]"
                    change_str = f"[bold red]{change:+.2f}[/bold red]"
                    pct_str = f"[red]{pct:+.2f}%[/red]" if pct else "[dim]N/A[/dim]"
                elif change is not None and change < 0:
                    lamp = "[bold green]🟢[/bold green]"
                    change_str = f"[bold green]{change:+.2f}[/bold green]"
                    pct_str = f"[green]{pct:+.2f}%[/green]" if pct else "[dim]N/A[/dim]"
                else:
                    lamp = "[yellow]🟡[/yellow]"
                    change_str = f"[yellow]{change:+.2f}[/yellow]" if change is not None else "[dim]N/A[/dim]"
                    pct_str = f"[yellow]{pct:+.2f}%[/yellow]" if pct else "[dim]N/A[/dim]"

                price_str = f"[bold]{price:,.2f}[/bold]"

            prev_str = f"{item['prev']:,.2f}" if item['prev'] else "N/A"
            open_str = f"{item['open']:,.2f}" if item['open'] else "N/A"
            high_str = f"{item['high']:,.2f}" if item['high'] else "N/A"
            low_str = f"{item['low']:,.2f}" if item['low'] else "N/A"

            if item['volume']:
                vol_lots = int(item['volume'] // 1000) if item['volume'] >= 1000 else item['volume']
                vol_str = f"{vol_lots:,}"
            else:
                vol_str = "N/A"

            source_str = item['source'][:8] if item['source'] else "N/A"

            table.add_row(
                lamp,
                item["id"],
                item["name"],
                price_str,
                change_str,
                pct_str,
                prev_str,
                open_str,
                high_str,
                low_str,
                vol_str,
                source_str,
            )

        return table

    def run(self):
        self.console.clear()
        self.console.print("[bold cyan]台股追蹤看板啟動中...[/bold cyan]")
        self.console.print(f"[dim]更新頻率: {self.interval} 秒 | 按 Ctrl+C 停止[/dim]")
        self.console.print()

        while True:
            try:
                data = self.fetch_quotes()
                now = datetime.now().strftime("%Y-%m-%d %H:%M:%S")

                header = Panel(
                    f"[bold white]台股即時看板[/bold white]\n"
                    f"[dim]更新時間: {now} | 追蹤: {len(self.watchlist)} 檔股票 | 間隔: {self.interval}秒 | 資料來源: CMoney/TWSE MIS[/dim]",
                    border_style="cyan",
                )

                table = self.build_table(data)

                footer = Panel(
                    f"[dim]按 Ctrl+C 停止追蹤[/dim]",
                    border_style="dim",
                )

                self.console.clear()
                self.console.print(header)
                self.console.print(table)
                self.console.print(footer)

                time.sleep(self.interval)

            except KeyboardInterrupt:
                self.console.print("\n[bold yellow]追蹤已停止[/bold yellow]")
                break
            except Exception as e:
                self.console.print(f"[bold red]錯誤: {e}[/bold red]")
                time.sleep(5)


def main():
    config = load_config()
    default_interval = config.get("interval", 10)

    parser = argparse.ArgumentParser(description="台股即時追蹤看板")
    parser.add_argument("-i", "--interval", type=int, default=default_interval, help="更新間隔秒數")
    parser.add_argument("-s", "--stocks", type=str, help="自訂股票清單，用逗號分隔 (例: 2330,2317,0050)")
    args = parser.parse_args()

    if args.stocks:
        stock_ids = [s.strip() for s in args.stocks.split(",")]
        watchlist = [{"id": sid, "name": sid} for sid in stock_ids]
    else:
        watchlist = DEFAULT_WATCHLIST

    tracker = StockTracker(watchlist=watchlist, interval=args.interval)
    tracker.run()


if __name__ == "__main__":
    main()
