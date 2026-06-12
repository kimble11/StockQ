import json
import math
import os
import re
import ssl
import subprocess
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any, Dict, List, Optional

UA = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/124 Safari/537.36"
SSL_CTX = ssl._create_unverified_context()


def fetch_text(url: str, *, headers: Optional[Dict[str, str]] = None, timeout: int = 20) -> str:
    h = {
        "User-Agent": UA,
        "Accept-Language": "zh-TW,zh;q=0.9,en;q=0.8",
        "Accept": "text/html,application/xhtml+xml,application/xml,application/json,*/*;q=0.8",
    }
    if headers:
        h.update(headers)
    req = urllib.request.Request(url, headers=h)
    with urllib.request.urlopen(req, timeout=timeout, context=SSL_CTX) as resp:
        return resp.read().decode("utf-8", "ignore")


def to_num(value: Any) -> Optional[float]:
    if value is None:
        return None
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        if math.isnan(float(value)):
            return None
        return float(value)
    s = str(value).strip().replace(",", "")
    if not s or s in {"-", "—", "N/A"}:
        return None
    try:
        return float(s)
    except ValueError:
        return None


def run_node_eval(js_expr: str) -> Any:
    script = (
        "global.window={};\n"
        "window.__NUXT__=" + js_expr + ";\n"
        "const out = window.__NUXT__ && window.__NUXT__.data && "
        "window.__NUXT__.data[0] && window.__NUXT__.data[0].additionalData;\n"
        "console.log(JSON.stringify(out || null));\n"
    )
    with tempfile.NamedTemporaryFile("w", suffix=".js", delete=False, encoding="utf-8") as f:
        f.write(script)
        path = f.name
    try:
        env = os.environ.copy()
        env["PYTHONIOENCODING"] = "utf-8"
        cp = subprocess.run(
            ["node", path],
            capture_output=True,
            timeout=15,
            encoding="utf-8",
            errors="replace",
            env=env,
        )
        if cp.returncode != 0:
            raise RuntimeError(cp.stderr.strip() or cp.stdout.strip())
        return json.loads(cp.stdout)
    finally:
        try:
            Path(path).unlink(missing_ok=True)
        except Exception:
            pass


def fetch_cmoney(code: str) -> Optional[Dict[str, Any]]:
    url = f"https://www.cmoney.tw/forum/stock/{urllib.parse.quote(code)}"
    html = fetch_text(url, headers={"Referer": "https://www.cmoney.tw/forum/stock"})

    m = re.search(r"window\.__?NUXT__?\s*=(.*?)</script>", html, re.S)
    if m:
        try:
            data = run_node_eval(m.group(1))
            if isinstance(data, dict):
                price = to_num(data.get("stockPrice"))
                if price is not None:
                    prev = to_num(data.get("stockClosePrice"))
                    change = to_num(data.get("stockQuotePrice"))
                    pct = to_num(data.get("stockQuoteRate"))
                    if change is None and prev is not None:
                        change = price - prev
                    if pct is None and change is not None and prev:
                        pct = change / prev * 100
                    volume_lots = to_num(data.get("stockTotalVolume"))
                    volume_shares = volume_lots * 1000 if volume_lots is not None else None
                    return {
                        "price": price,
                        "change": change,
                        "pct": pct,
                        "prev": prev,
                        "open": to_num(data.get("stockOpenPrice")),
                        "high": to_num(data.get("stockHighestPrice")),
                        "low": to_num(data.get("stockLowestPrice")),
                        "volume": volume_shares,
                        "source": "CMoney",
                        "name": data.get("stockName"),
                    }
        except Exception:
            pass

    match = re.search(r'<meta[^>]+name="description"[^>]+content="([^"]+)"', html, re.IGNORECASE)
    if not match:
        return None
    import html as _html
    desc = _html.unescape(match.group(1))
    patterns = [
        r'股價(?P<price>[-0-9.]+)；漲跌幅(?P<pct>[+-]?[0-9.]+)[％%]；開盤(?P<open>[-0-9.]+)；最高(?P<high>[-0-9.]+)；最低(?P<low>[-0-9.]+)；昨收(?P<prev>[-0-9.]+)',
        r'股價(?P<price>[-0-9.]+)；開盤(?P<open>[-0-9.]+)；最高(?P<high>[-0-9.]+)；最低(?P<low>[-0-9.]+)；昨收(?P<prev>[-0-9.]+)；漲跌幅(?P<pct>[+-]?[0-9.]+)[％%]',
    ]
    mm = None
    for pat in patterns:
        mm = re.search(pat, desc)
        if mm:
            break
    if not mm:
        return None
    price = to_num(mm.group('price'))
    prev = to_num(mm.group('prev'))
    pct = to_num(mm.group('pct'))
    if price is None:
        return None
    change = price - prev if prev not in (None, 0) else None

    return {
        "price": price,
        "change": change,
        "pct": pct,
        "prev": prev,
        "open": to_num(mm.group('open')),
        "high": to_num(mm.group('high')),
        "low": to_num(mm.group('low')),
        "volume": None,
        "source": "CMoney",
        "name": None,
    }


def fetch_twse_mis(codes: List[str]) -> Dict[str, Dict[str, Any]]:
    exchs = []
    for code in codes:
        exchs.extend([f"tse_{code}.tw", f"otc_{code}.tw"])
    url = "https://mis.twse.com.tw/stock/api/getStockInfo.jsp?" + urllib.parse.urlencode(
        {
            "ex_ch": "|".join(exchs),
            "json": "1",
            "delay": "0",
            "_": str(int(time.time() * 1000)),
        }
    )
    txt = fetch_text(
        url,
        headers={
            "Referer": "https://mis.twse.com.tw/stock/fibest.jsp?lang=zh_tw",
            "Accept": "application/json,*/*",
        },
        timeout=15,
    ).strip()
    data = json.loads(txt)
    out: Dict[str, Dict[str, Any]] = {}
    for item in data.get("msgArray", []):
        code = item.get("c")
        if code and item.get("n"):
            out[str(code)] = item
    return out


def parse_level_one(levels: Any) -> Optional[float]:
    if not levels:
        return None
    first = str(levels).split("_")[0]
    return to_num(first)


def quote_from_twse_item(item: Dict[str, Any]) -> Optional[Dict[str, Any]]:
    if not item:
        return None
    price = to_num(item.get("z")) or to_num(item.get("pz"))
    source = "TWSE MIS"
    if price is None:
        bid1 = parse_level_one(item.get("b"))
        ask1 = parse_level_one(item.get("a"))
        if bid1 is not None and ask1 is not None:
            price = (bid1 + ask1) / 2
            source = "TWSE MIS估價"
        elif bid1 is not None:
            price = bid1
        elif ask1 is not None:
            price = ask1
    prev = to_num(item.get("y"))
    if price is None and prev is None:
        return None
    change = price - prev if price is not None and prev is not None else None
    pct = change / prev * 100 if change is not None and prev else None
    vol_lots = to_num(item.get("v"))
    volume = vol_lots * 1000 if vol_lots is not None else None
    return {
        "price": price,
        "change": change,
        "pct": pct,
        "prev": prev,
        "open": to_num(item.get("o")),
        "high": to_num(item.get("h")),
        "low": to_num(item.get("l")),
        "volume": volume,
        "source": source,
        "name": item.get("n"),
    }


class TaiwanStockRealtime:
    def fetch_one(self, code: str) -> Optional[Dict[str, Any]]:
        q = None
        try:
            q = fetch_cmoney(code)
        except Exception:
            q = None

        if q and q.get("price") is not None:
            return q

        try:
            twse_data = fetch_twse_mis([code])
            if code in twse_data:
                q2 = quote_from_twse_item(twse_data[code])
                if q2 and q2.get("price") is not None:
                    return q2
        except Exception:
            pass

        return None

    def fetch_all(self, codes: List[str]) -> Dict[str, Dict[str, Any]]:
        results = {}
        for code in codes:
            q = self.fetch_one(code)
            if q:
                results[code] = q
            time.sleep(0.3)
        return results
