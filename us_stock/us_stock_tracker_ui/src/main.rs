use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, TableState},
    Frame, Terminal,
};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::Duration;

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/124 Safari/537.36";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StockItem {
    id: String,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    interval: u64,
    watchlist: Vec<StockItem>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interval: 10,
            watchlist: vec![
                StockItem { id: "TSLA".to_string(), name: "特斯拉".to_string() },
                StockItem { id: "NVDA".to_string(), name: "輝達".to_string() },
                StockItem { id: "TSM".to_string(), name: "台積電".to_string() },
                StockItem { id: "AAPL".to_string(), name: "蘋果".to_string() },
                StockItem { id: "MSFT".to_string(), name: "微軟".to_string() },
                StockItem { id: "GOOGL".to_string(), name: "谷歌".to_string() },
                StockItem { id: "AMZN".to_string(), name: "亞馬遜".to_string() },
                StockItem { id: "INTC".to_string(), name: "英特爾".to_string() },
                StockItem { id: "META".to_string(), name: "臉書".to_string() },
            ],
        }
    }
}

#[derive(Debug, Clone)]
struct StockQuote {
    id: String,
    name: String,
    price: Option<f64>,
    change: Option<f64>,
    pct: Option<f64>,
    prev: Option<f64>,
    open: Option<f64>,
    high: Option<f64>,
    low: Option<f64>,
    volume: Option<f64>,
    source: String,
}

#[derive(Debug, Clone, PartialEq)]
enum AppMode {
    Main,
    Settings,
    EditInterval,
    EditStocks,
    AddStockId,
    AddStockName,
    ConfirmDelete,
}

struct App {
    mode: AppMode,
    config: Config,
    quotes: Vec<StockQuote>,
    last_update: String,
    table_state: TableState,
    settings_state: ListState,
    stocks_state: ListState,
    input_buffer: String,
    editing_field: usize,
    error_msg: Option<String>,
    new_stock_id: String,
    new_stock_name: String,
    delete_index: Option<usize>,
}

impl App {
    fn new(config: Config) -> Self {
        let mut settings_state = ListState::default();
        settings_state.select(Some(0));

        let mut table_state = TableState::default();
        table_state.select(Some(0));

        let mut stocks_state = ListState::default();
        stocks_state.select(Some(0));

        Self {
            mode: AppMode::Main,
            config,
            quotes: Vec::new(),
            last_update: "尚未更新".to_string(),
            table_state,
            settings_state,
            stocks_state,
            input_buffer: String::new(),
            editing_field: 0,
            error_msg: None,
            new_stock_id: String::new(),
            new_stock_name: String::new(),
            delete_index: None,
        }
    }
}

fn config_path() -> PathBuf {
    let mut path = PathBuf::from(std::env::current_dir().unwrap_or_default());
    path.push("config.json");
    path
}

fn load_config() -> Config {
    let path = config_path();
    if path.exists() {
        let data = std::fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        let config = Config::default();
        save_config(&config);
        config
    }
}

fn save_config(config: &Config) {
    let path = config_path();
    let data = serde_json::to_string_pretty(config).unwrap_or_default();
    let _ = std::fs::write(path, data);
}

fn to_float(val: Option<&serde_json::Value>) -> Option<f64> {
    match val {
        Some(serde_json::Value::Number(n)) => n.as_f64(),
        Some(serde_json::Value::String(s)) => s.parse::<f64>().ok(),
        _ => None,
    }
}

async fn fetch_cmoney_us(symbol: &str) -> Option<StockQuote> {
    let url = format!("https://www.cmoney.tw/forum/usstock/{}", symbol);
    let client = reqwest::Client::new();
    let resp = match client.get(&url)
        .header("User-Agent", UA)
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return None,
    };
    let body = match resp.text().await {
        Ok(t) => t,
        Err(_) => return None,
    };

    let meta_start = body.find(r#"name="description""#)?;
    let content_start = body[meta_start..].find("content=\"")? + meta_start + 9;
    let content_end = body[content_start..].find('"')? + content_start;
    let desc = &body[content_start..content_end];
    if !desc.contains("股價") {
        return None;
    }

    let extract = |label: &str| -> Option<String> {
        let idx = desc.find(label)?;
        let rest = &desc[idx + label.len()..];
        let end = rest.find(['；', '。', ';']).unwrap_or(rest.len());
        Some(rest[..end].trim().to_string())
    };

    let parse_f = |label: &str| -> Option<f64> {
        extract(label).and_then(|s| {
            s.replace(',', "").replace('％', "").replace('%', "").trim().parse::<f64>().ok()
        })
    };

    let price = parse_f("股價")?;
    let prev = parse_f("昨收");
    let open = parse_f("開盤");
    let high = parse_f("最高");
    let low = parse_f("最低");
    let pct = parse_f("漲跌幅");
    let volume = extract("成交量")
        .or_else(|| extract("總量"))
        .and_then(|s| s.replace(',', "").parse::<f64>().ok());

    let change = match (prev, Some(price)) {
        (Some(y), Some(p)) if y != 0.0 => Some(p - y),
        _ => None,
    };

    Some(StockQuote {
        id: symbol.to_string(),
        name: String::new(),
        price: Some(price),
        change,
        pct,
        prev,
        open,
        high,
        low,
        volume,
        source: "CMONEY".to_string(),
    })
}

async fn fetch_yahoo(symbol: &str) -> Option<StockQuote> {
    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?interval=1d&range=1d",
        reqwest::Url::parse(&format!("https://x/{}", symbol)).ok()?.path().trim_start_matches('/')
    );
    let client = reqwest::Client::new();
    let resp = match client.get(&url)
        .header("User-Agent", UA)
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return None,
    };
    let body = match resp.text().await {
        Ok(t) => t,
        Err(_) => return None,
    };
    let data: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => return None,
    };

    let chart = data.get("chart")?;
    let result = chart.get("result")?.as_array()?.first()?;
    let meta = result.get("meta")?;

    let price = to_float(meta.get("regularMarketPrice"));
    let prev = to_float(meta.get("chartPreviousClose").or(meta.get("previousClose")));
    let high = to_float(meta.get("regularMarketDayHigh"));
    let low = to_float(meta.get("regularMarketDayLow"));
    let volume = to_float(meta.get("regularMarketVolume"));

    let open = result.get("indicators")
        .and_then(|ind| ind.get("quote"))
        .and_then(|q| q.as_array())
        .and_then(|arr| arr.first())
        .and_then(|q| q.get("open"))
        .and_then(|o| {
            if let Some(arr) = o.as_array() {
                arr.iter().find_map(|v| v.as_f64())
            } else {
                None
            }
        });

    let change = match (price, prev) {
        (Some(p), Some(y)) if y != 0.0 => Some(p - y),
        _ => None,
    };
    let pct = match (change, prev) {
        (Some(c), Some(y)) if y != 0.0 => Some(c / y * 100.0),
        _ => None,
    };

    let ts = meta.get("regularMarketTime").and_then(|t| t.as_i64());
    let time_str = match ts {
        Some(t) => chrono::DateTime::from_timestamp(t, 0)
            .map(|dt| dt.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()),
        None => chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };

    Some(StockQuote {
        id: symbol.to_string(),
        name: String::new(),
        price,
        change,
        pct,
        prev,
        open,
        high,
        low,
        volume,
        source: format!("Yahoo {}", time_str),
    })
}

async fn fetch_single(symbol: &str) -> StockQuote {
    if let Some(mut q) = fetch_cmoney_us(symbol).await {
        return q;
    }
    if let Some(q) = fetch_yahoo(symbol).await {
        return q;
    }
    empty_quote(symbol)
}

fn empty_quote(code: &str) -> StockQuote {
    StockQuote {
        id: code.to_string(),
        name: code.to_string(),
        price: None,
        change: None,
        pct: None,
        prev: None,
        open: None,
        high: None,
        low: None,
        volume: None,
        source: "N/A".to_string(),
    }
}

fn fmt_num(val: Option<f64>) -> String {
    match val {
        Some(v) => format!("{:.2}", v),
        None => "N/A".to_string(),
    }
}

fn fmt_pct(val: Option<f64>) -> String {
    match val {
        Some(v) => format!("{:+.2}%", v),
        None => "N/A".to_string(),
    }
}

fn fmt_vol(val: Option<f64>) -> String {
    match val {
        Some(v) => {
            if v >= 1_000_000_000.0 {
                format!("{:.1}B", v / 1_000_000_000.0)
            } else if v >= 1_000_000.0 {
                format!("{:.1}M", v / 1_000_000.0)
            } else if v >= 1_000.0 {
                format!("{:.0}K", v / 1_000.0)
            } else {
                format!("{:.0}", v)
            }
        }
        None => "N/A".to_string(),
    }
}

fn draw_main(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.size());

    let header = Paragraph::new(Line::from(vec![
        Span::styled(" 美股即時追蹤看板 ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" | "),
        Span::styled(&app.last_update, Style::default().fg(Color::DarkGray)),
        Span::raw(" | "),
        Span::styled(format!("{}檔", app.quotes.len()), Style::default().fg(Color::Yellow)),
        Span::raw(" | "),
        Span::styled(format!("{}秒", app.config.interval), Style::default().fg(Color::Yellow)),
        Span::raw(" | "),
        Span::styled("[S]設定 [Q]離開", Style::default().fg(Color::DarkGray)),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));
    f.render_widget(header, chunks[0]);

    let header_cells = ["燈號", "代號", "名稱", "現價", "漲跌", "漲跌幅", "昨收", "開盤", "最高", "最低", "成交量"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));
    let header_row = Row::new(header_cells).height(1).style(Style::default().bg(Color::Blue));

    let rows = app.quotes.iter().map(|q| {
        let lamp = match q.change {
            Some(c) if c > 0.0 => Span::styled(" 🔴 ", Style::default().fg(Color::Red)),
            Some(c) if c < 0.0 => Span::styled(" 🟢 ", Style::default().fg(Color::Green)),
            Some(_) => Span::styled(" 🟡 ", Style::default().fg(Color::Yellow)),
            None => Span::styled(" ⚪ ", Style::default().fg(Color::Gray)),
        };

        let price_style = match q.change {
            Some(c) if c > 0.0 => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            Some(c) if c < 0.0 => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            _ => Style::default(),
        };

        let change_style = match q.change {
            Some(c) if c > 0.0 => Style::default().fg(Color::Red),
            Some(c) if c < 0.0 => Style::default().fg(Color::Green),
            _ => Style::default(),
        };

        Row::new(vec![
            Cell::from(lamp),
            Cell::from(q.id.clone()),
            Cell::from(q.name.clone()),
            Cell::from(Span::styled(fmt_num(q.price), price_style)),
            Cell::from(Span::styled(fmt_num(q.change), change_style.clone())),
            Cell::from(Span::styled(fmt_pct(q.pct), change_style)),
            Cell::from(fmt_num(q.prev)),
            Cell::from(fmt_num(q.open)),
            Cell::from(fmt_num(q.high)),
            Cell::from(fmt_num(q.low)),
            Cell::from(fmt_vol(q.volume)),
        ])
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(5),
            Constraint::Length(7),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(9),
            Constraint::Length(9),
            Constraint::Length(9),
            Constraint::Length(9),
            Constraint::Length(10),
        ],
    )
    .header(header_row)
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Blue)))
    .highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(table, chunks[1], &mut app.table_state);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" 方向鍵:選取 | S:設定 | Q:離開 ", Style::default().fg(Color::DarkGray)),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(footer, chunks[2]);
}

fn draw_settings(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.size());

    let header = Paragraph::new(Line::from(vec![
        Span::styled(" 設定 ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" | "),
        Span::styled("[ESC]返回 | [Enter]編輯 | [↑↓]選取", Style::default().fg(Color::DarkGray)),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));
    f.render_widget(header, chunks[0]);

    let items = vec![
        ListItem::new(Line::from(vec![
            Span::styled("  更新秒數: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{} 秒", app.config.interval),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("  追蹤清單 ", Style::default().fg(Color::White)),
        ])),
    ];

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Blue)))
        .highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(list, chunks[1], &mut app.settings_state);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" 按 S 或 ESC 返回主畫面 ", Style::default().fg(Color::DarkGray)),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(footer, chunks[2]);
}

fn draw_edit_interval(f: &mut Frame, app: &mut App) {
    let area = f.size();
    let popup = Rect {
        x: (area.width.saturating_sub(40)) / 2,
        y: (area.height.saturating_sub(7)) / 2,
        width: 40.min(area.width),
        height: 7.min(area.height),
    };

    let block = Block::default()
        .title(" 編輯更新秒數 ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(" 請輸入秒數: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{}█", app.input_buffer),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(" [Enter]確認 [ESC]取消", Style::default().fg(Color::DarkGray))),
    ];

    let paragraph = Paragraph::new(text);
    f.render_widget(paragraph, inner);
}

fn draw_edit_stocks(f: &mut Frame, app: &mut App) {
    let area = f.size();
    let popup = Rect {
        x: (area.width.saturating_sub(60)) / 2,
        y: (area.height.saturating_sub(20)) / 2,
        width: 60.min(area.width),
        height: 20.min(area.height),
    };

    let block = Block::default()
        .title(" 追蹤清單管理 ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(5),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(inner);

    let header = Paragraph::new(Line::from(vec![
        Span::styled(" [A]新增 [D]刪除 [ESC]返回", Style::default().fg(Color::DarkGray)),
    ]));
    f.render_widget(header, chunks[0]);

    let items: Vec<ListItem> = app.config.watchlist.iter().enumerate().map(|(i, s)| {
        let style = if Some(i) == app.stocks_state.selected() {
            Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        ListItem::new(Line::from(vec![
            Span::styled(format!(" {:<8} ", s.id), Style::default().fg(Color::Yellow)),
            Span::styled(&s.name, Style::default().fg(Color::White)),
        ])).style(style)
    }).collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::NONE))
        .highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(list, chunks[1], &mut app.stocks_state);

    if let Some(ref err) = app.error_msg {
        let err_para = Paragraph::new(Line::from(Span::styled(
            format!(" {}", err),
            Style::default().fg(Color::Red),
        )));
        f.render_widget(err_para, chunks[2]);
    }

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" 上下選取 | A:新增 | D:刪除 | ESC:返回 ", Style::default().fg(Color::DarkGray)),
    ]));
    f.render_widget(footer, chunks[3]);
}

fn draw_add_stock_id(f: &mut Frame, app: &mut App) {
    let area = f.size();
    let popup = Rect {
        x: (area.width.saturating_sub(45)) / 2,
        y: (area.height.saturating_sub(9)) / 2,
        width: 45.min(area.width),
        height: 9.min(area.height),
    };

    let block = Block::default()
        .title(" 新增股票 ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let mut text = vec![
        Line::from(""),
        Line::from(Span::styled(" 請輸入股票代號: ", Style::default().fg(Color::White))),
        Line::from(vec![
            Span::styled(" > ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}█", app.new_stock_id),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];

    if let Some(ref err) = app.error_msg {
        text.push(Line::from(Span::styled(
            format!(" {}", err),
            Style::default().fg(Color::Red),
        )));
    }

    text.push(Line::from(Span::styled(" [Enter]確認 [ESC]取消", Style::default().fg(Color::DarkGray))));

    let paragraph = Paragraph::new(text);
    f.render_widget(paragraph, inner);
}

fn draw_add_stock_name(f: &mut Frame, app: &mut App) {
    let area = f.size();
    let popup = Rect {
        x: (area.width.saturating_sub(45)) / 2,
        y: (area.height.saturating_sub(11)) / 2,
        width: 45.min(area.width),
        height: 11.min(area.height),
    };

    let block = Block::default()
        .title(" 確認新增 ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(" 代號: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&app.new_stock_id, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" 名稱: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&app.new_stock_name, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(Span::styled(" [Enter]確認新增 [ESC]取消", Style::default().fg(Color::DarkGray))),
    ];

    let paragraph = Paragraph::new(text);
    f.render_widget(paragraph, inner);
}

fn draw_confirm_delete(f: &mut Frame, app: &mut App) {
    let area = f.size();
    let popup = Rect {
        x: (area.width.saturating_sub(35)) / 2,
        y: (area.height.saturating_sub(7)) / 2,
        width: 35.min(area.width),
        height: 7.min(area.height),
    };

    let block = Block::default()
        .title(" 確認刪除 ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let stock_name = app.delete_index
        .and_then(|i| app.config.watchlist.get(i))
        .map(|s| format!("{} {}", s.id, s.name))
        .unwrap_or_default();

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!(" 刪除 {} ?", stock_name),
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(Span::styled(" [Y]確認刪除 [N/ESC]取消", Style::default().fg(Color::DarkGray))),
    ];

    let paragraph = Paragraph::new(text);
    f.render_widget(paragraph, inner);
}

async fn lookup_symbol(query: &str) -> Option<(String, String)> {
    let symbol = query.to_uppercase();
    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?interval=1d&range=1d",
        reqwest::Url::parse(&format!("https://x/{}", symbol)).ok()?.path().trim_start_matches('/')
    );
    let client = reqwest::Client::new();
    let resp = match client.get(&url).header("User-Agent", UA).send().await {
        Ok(r) => r,
        Err(_) => return None,
    };
    let body = match resp.text().await {
        Ok(t) => t,
        Err(_) => return None,
    };
    let data: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => return None,
    };

    let chart = data.get("chart")?;
    let result = chart.get("result")?.as_array()?.first()?;
    let meta = result.get("meta")?;
    let sym = meta.get("symbol")?.as_str()?.to_string();
    let name = meta.get("shortName").or(meta.get("longName"))
        .and_then(|n| n.as_str())
        .unwrap_or(&sym)
        .to_string();

    Some((sym, name))
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let config = load_config();
    let shared_config = Arc::new(RwLock::new(config.clone()));
    let (tx, mut rx) = mpsc::unbounded_channel::<(Vec<StockQuote>, String)>();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(config.clone());

    let config_clone = shared_config.clone();
    let tx_clone = tx.clone();

    tokio::spawn(async move {
        loop {
            let cfg = config_clone.read().await;
            let interval_secs = cfg.interval;
            let items: Vec<StockItem> = cfg.watchlist.iter().cloned().collect();
            drop(cfg);

            let mut new_quotes = Vec::new();
            for item in &items {
                let mut q = fetch_single(&item.id).await;
                if q.name.is_empty() {
                    q.name = item.name.clone();
                } else if item.name != item.id {
                    q.name = item.name.clone();
                }
                new_quotes.push(q);
            }

            let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let _ = tx_clone.send((new_quotes, now));

            tokio::time::sleep(Duration::from_secs(interval_secs)).await;
        }
    });

    loop {
        if app.mode == AppMode::Main {
            while let Ok((quotes, update_time)) = rx.try_recv() {
                app.quotes = quotes;
                app.last_update = update_time;
            }
        } else {
            while rx.try_recv().is_ok() {}
        }

        terminal.draw(|f| {
            match app.mode {
                AppMode::Main => draw_main(f, &mut app),
                AppMode::Settings => draw_settings(f, &mut app),
                AppMode::EditInterval => {
                    draw_edit_interval(f, &mut app);
                }
                AppMode::EditStocks => {
                    draw_edit_stocks(f, &mut app);
                }
                AppMode::AddStockId => {
                    draw_add_stock_id(f, &mut app);
                }
                AppMode::AddStockName => {
                    draw_add_stock_name(f, &mut app);
                }
                AppMode::ConfirmDelete => {
                    draw_confirm_delete(f, &mut app);
                }
            }
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.mode {
                        AppMode::Main => match key.code {
                            KeyCode::Char('q') | KeyCode::Char('Q') => break,
                            KeyCode::Char('s') | KeyCode::Char('S') => {
                                app.mode = AppMode::Settings;
                            }
                            KeyCode::Up => {
                                let i = app.table_state.selected().unwrap_or(0);
                                if i > 0 {
                                    app.table_state.select(Some(i - 1));
                                }
                            }
                            KeyCode::Down => {
                                let i = app.table_state.selected().unwrap_or(0);
                                if i < app.quotes.len().saturating_sub(1) {
                                    app.table_state.select(Some(i + 1));
                                }
                            }
                            _ => {}
                        },
                        AppMode::Settings => match key.code {
                            KeyCode::Esc | KeyCode::Char('s') | KeyCode::Char('S') => {
                                app.mode = AppMode::Main;
                            }
                            KeyCode::Up => {
                                let i = app.settings_state.selected().unwrap_or(0);
                                if i > 0 {
                                    app.settings_state.select(Some(i - 1));
                                }
                            }
                            KeyCode::Down => {
                                let i = app.settings_state.selected().unwrap_or(0);
                                if i < 1 {
                                    app.settings_state.select(Some(i + 1));
                                }
                            }
                            KeyCode::Enter => {
                                let i = app.settings_state.selected().unwrap_or(0);
                                if i == 0 {
                                    app.mode = AppMode::EditInterval;
                                    app.input_buffer = app.config.interval.to_string();
                                } else {
                                    app.mode = AppMode::EditStocks;
                                }
                            }
                            _ => {}
                        },
                        AppMode::EditInterval => match key.code {
                            KeyCode::Esc => {
                                app.mode = AppMode::Settings;
                                app.input_buffer.clear();
                            }
                            KeyCode::Enter => {
                                if let Ok(n) = app.input_buffer.trim().parse::<u64>() {
                                    if n > 0 && n <= 300 {
                                        app.config.interval = n;
                                        save_config(&app.config);
                                        let mut cfg = shared_config.write().await;
                                        cfg.interval = n;
                                        drop(cfg);
                                        app.mode = AppMode::Settings;
                                        app.input_buffer.clear();
                                    }
                                }
                            }
                            KeyCode::Backspace => {
                                app.input_buffer.pop();
                            }
                            KeyCode::Char(c) => {
                                if c.is_ascii_digit() {
                                    app.input_buffer.push(c);
                                }
                            }
                            _ => {}
                        },
                        AppMode::EditStocks => match key.code {
                            KeyCode::Esc => {
                                app.mode = AppMode::Settings;
                                app.error_msg = None;
                            }
                            KeyCode::Up => {
                                let i = app.stocks_state.selected().unwrap_or(0);
                                if i > 0 {
                                    app.stocks_state.select(Some(i - 1));
                                }
                            }
                            KeyCode::Down => {
                                let i = app.stocks_state.selected().unwrap_or(0);
                                if i < app.config.watchlist.len().saturating_sub(1) {
                                    app.stocks_state.select(Some(i + 1));
                                }
                            }
                            KeyCode::Char('a') | KeyCode::Char('A') => {
                                app.mode = AppMode::AddStockId;
                                app.new_stock_id.clear();
                                app.new_stock_name.clear();
                            }
                            KeyCode::Char('d') | KeyCode::Char('D') => {
                                if let Some(i) = app.stocks_state.selected() {
                                    if i < app.config.watchlist.len() {
                                        app.delete_index = Some(i);
                                        app.mode = AppMode::ConfirmDelete;
                                    }
                                }
                            }
                            _ => {}
                        },
                        AppMode::AddStockId => match key.code {
                            KeyCode::Esc => {
                                app.mode = AppMode::EditStocks;
                                app.new_stock_id.clear();
                                app.new_stock_name.clear();
                                app.error_msg = None;
                            }
                            KeyCode::Enter => {
                                let query = app.new_stock_id.trim().to_string();
                                if !query.is_empty() {
                                    if let Some((sym, name)) = lookup_symbol(&query).await {
                                        app.new_stock_id = sym;
                                        app.new_stock_name = name;
                                        app.mode = AppMode::AddStockName;
                                        app.error_msg = None;
                                    } else {
                                        app.error_msg = Some("找不到此股票".to_string());
                                    }
                                }
                            }
                            KeyCode::Backspace => {
                                app.new_stock_id.pop();
                                app.error_msg = None;
                            }
                            KeyCode::Char(c) => {
                                app.new_stock_id.push(c);
                                app.error_msg = None;
                            }
                            _ => {}
                        },
                        AppMode::AddStockName => match key.code {
                            KeyCode::Esc => {
                                app.mode = AppMode::EditStocks;
                                app.new_stock_id.clear();
                                app.new_stock_name.clear();
                                app.error_msg = None;
                            }
                            KeyCode::Enter => {
                                if !app.new_stock_name.trim().is_empty() && !app.new_stock_id.trim().is_empty() {
                                    let stock = StockItem {
                                        id: app.new_stock_id.trim().to_uppercase(),
                                        name: app.new_stock_name.trim().to_string(),
                                    };
                                    app.config.watchlist.push(stock);
                                    save_config(&app.config);
                                    let mut cfg = shared_config.write().await;
                                    cfg.watchlist = app.config.watchlist.clone();
                                    drop(cfg);
                                    app.mode = AppMode::EditStocks;
                                    app.new_stock_id.clear();
                                    app.new_stock_name.clear();
                                    app.error_msg = None;
                                    let len = app.config.watchlist.len();
                                    if len > 0 {
                                        app.stocks_state.select(Some(len - 1));
                                    }
                                }
                            }
                            KeyCode::Backspace => {
                                app.new_stock_name.pop();
                            }
                            KeyCode::Char(c) => {
                                app.new_stock_name.push(c);
                            }
                            _ => {}
                        },
                        AppMode::ConfirmDelete => match key.code {
                            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                                app.mode = AppMode::EditStocks;
                                app.delete_index = None;
                            }
                            KeyCode::Char('y') | KeyCode::Char('Y') => {
                                if let Some(i) = app.delete_index {
                                    if i < app.config.watchlist.len() {
                                        app.config.watchlist.remove(i);
                                        save_config(&app.config);
                                        let mut cfg = shared_config.write().await;
                                        cfg.watchlist = app.config.watchlist.clone();
                                        drop(cfg);
                                        let len = app.config.watchlist.len();
                                        if len > 0 {
                                            let new_idx = if i >= len { len - 1 } else { i };
                                            app.stocks_state.select(Some(new_idx));
                                        }
                                    }
                                }
                                app.mode = AppMode::EditStocks;
                                app.delete_index = None;
                            }
                            _ => {}
                        },
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
