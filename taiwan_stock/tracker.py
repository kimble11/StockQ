import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from taiwan_stock import TaiwanStock
from rich.console import Console
from rich.table import Table
from rich.live import Live
from rich.layout import Layout
from rich.panel import Panel
from rich.text import Text
from datetime import datetime
import time
import argparse

DEFAULT_WATCHLIST = [
    {"id": "2330", "name": "台積電"},
    {"id": "2317", "name": "鴻海"},
    {"id": "6770", "name": "力積電"},
    {"id": "2911", "name": "麗嬰房"},
    {"id": "0050", "name": "元大台灣50"},
    {"id": "0056", "name": "元大高股息"},
    {"id": "00878", "name": "國泰永續高股息"},
    {"id": "00919", "name": "群益台灣精選高息"},
    {"id": "00662", "name": "富邦NASDAQ"},
    {"id": "00757", "name": "統一FANG+"},
    {"id": "00981A", "name": "主動統一台股增長"},
]

class StockTracker:
    def __init__(self, watchlist=None, interval=30):
        self.stock = TaiwanStock()
        self.watchlist = watchlist or DEFAULT_WATCHLIST
        self.interval = interval
        self.console = Console()
        self.last_data = {}

    def fetch_quotes(self):
        """Fetch quotes for all stocks in watchlist"""
        self.stock._cache.clear()
        rows = self.stock._fetch_stock_table(force_refresh=True)
        data_map = {row[0]: row for row in rows}

        results = []
        for item in self.watchlist:
            stock_id = item["id"]
            stock_name = item["name"]

            if stock_id in data_map:
                row = data_map[stock_id]
                try:
                    price_str = row[8].replace(",", "")
                    price = float(price_str) if price_str else 0
                    change_str = row[10].replace(",", "").replace("+", "")
                    change = float(change_str) if change_str else 0
                except (ValueError, IndexError):
                    price = 0
                    change = 0

                results.append({
                    "id": stock_id,
                    "name": stock_name,
                    "price": price,
                    "change": change,
                    "open": row[5],
                    "high": row[6],
                    "low": row[7],
                    "volume": row[2],
                    "amount": row[4],
                })
            else:
                results.append({
                    "id": stock_id,
                    "name": stock_name,
                    "price": 0,
                    "change": 0,
                    "open": "-",
                    "high": "-",
                    "low": "-",
                    "volume": "-",
                    "amount": "-",
                })

        return results

    def build_table(self, data):
        """Build rich table from stock data"""
        table = Table(
            title=f"[bold cyan]台股追蹤看板[/bold cyan]",
            show_header=True,
            header_style="bold white on blue",
            border_style="bright_blue",
            expand=True,
        )

        table.add_column("狀態", justify="center", width=6)
        table.add_column("代號", justify="center", width=8)
        table.add_column("名稱", justify="left", width=16)
        table.add_column("成交價", justify="right", width=12)
        table.add_column("漲跌", justify="right", width=10)
        table.add_column("漲跌幅", justify="right", width=8)
        table.add_column("開盤", justify="right", width=10)
        table.add_column("最高", justify="right", width=10)
        table.add_column("最低", justify="right", width=10)
        table.add_column("成交股數", justify="right", width=14)

        for item in data:
            if item["price"] == 0:
                status = "[dim]--[/dim]"
                price = "[dim]--[/dim]"
                change = "[dim]--[/dim]"
                change_pct = "[dim]--[/dim]"
            else:
                prev_price = self.last_data.get(item["id"], item["price"])
                if item["price"] > prev_price:
                    status = "[bold red]▲[/bold red]"
                elif item["price"] < prev_price:
                    status = "[bold green]▼[/bold green]"
                else:
                    status = "[yellow]━[/yellow]"

                price = f"[bold]{item['price']:,.2f}[/bold]"

                if item["change"] > 0:
                    change = f"[bold red]+{item['change']:.2f}[/bold red]"
                    change_pct = f"[red]+{item['change']/prev_price*100:.2f}%[/red]" if prev_price else "[red]--[/red]"
                elif item["change"] < 0:
                    change = f"[bold green]{item['change']:.2f}[/bold green]"
                    change_pct = f"[green]{item['change']/prev_price*100:.2f}%[/green]" if prev_price else "[green]--[/green]"
                else:
                    change = f"[yellow]{item['change']:.2f}[/yellow]"
                    change_pct = "[yellow]0.00%[/yellow]"

            table.add_row(
                status,
                item["id"],
                item["name"],
                price,
                change,
                change_pct,
                item["open"],
                item["high"],
                item["low"],
                item["volume"],
            )

            self.last_data[item["id"]] = item["price"]

        return table

    def run(self):
        """Run the live tracker"""
        self.console.clear()
        self.console.print("[bold cyan]台股追蹤看板啟動中...[/bold cyan]")
        self.console.print(f"[dim]更新頻率: {self.interval} 秒 | 按 Ctrl+C 停止[/dim]")
        self.console.print()

        while True:
            try:
                data = self.fetch_quotes()
                now = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
                twse_date = self.stock._cache.get("stock_table_date_", "未知")

                header = Panel(
                    f"[bold white]台股即時看板[/bold white]\n"
                    f"[dim]更新時間: {now} | TWSE資料日期: {twse_date} | 追蹤: {len(self.watchlist)} 檔股票 | 間隔: {self.interval}秒[/dim]",
                    border_style="cyan",
                )

                table = self.build_table(data)

                footer = Panel(
                    f"[dim]按 Ctrl+C 停止追蹤 | 資料來源: 台灣證券交易所[/dim]",
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
    parser = argparse.ArgumentParser(description="台股即時追蹤看板")
    parser.add_argument(
        "-i", "--interval",
        type=int,
        default=30,
        help="更新間隔秒數 (預設: 30)",
    )
    parser.add_argument(
        "-s", "--stocks",
        type=str,
        help="自訂股票清單，用逗號分隔 (例: 2330,2317,0050)",
    )

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
