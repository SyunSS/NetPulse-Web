use std::path::Path;

use chrono::Utc;
use rust_xlsxwriter::*;
use sqlx::SqlitePool;

use crate::models::task::{DownloadResult, PingResult, VideoResult, WebsiteResult};

/// 导出格式
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Xlsx,
    Csv,
    Json,
}

/// 导出网站测试结果为 Excel
pub fn export_website_xlsx(
    db_data: &[WebsiteResult],
    task_id: &str,
    output_dir: &str,
) -> anyhow::Result<String> {
    let filename = format!("{}/website_{}_{}.xlsx", output_dir, task_id, Utc::now().timestamp());
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();

    // === 定义格式 ===
    let header_fmt = Format::new()
        .set_bold()
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(0x4472C4))
        .set_border(FormatBorder::Thin)
        .set_text_wrap();
    let cell_fmt = Format::new().set_border(FormatBorder::Thin);
    let success_fmt = Format::new()
        .set_border(FormatBorder::Thin)
        .set_font_color(Color::RGB(0x008000));
    let fail_fmt = Format::new()
        .set_border(FormatBorder::Thin)
        .set_font_color(Color::RGB(0xFF0000));
    let url_fmt = Format::new()
        .set_border(FormatBorder::Thin)
        .set_font_color(Color::RGB(0x0563C1))
        .set_underline(FormatUnderline::Single);

    // === 表头 ===
    let headers = [
        "序号", "URL", "DNS解析时延(ms)", "DNS解析成功率(%)", "TCP连接时延(ms)",
        "访问成功率(%)", "首包时延(ms)", "首屏时延(ms)", "首页时延(ms)",
        "总请求", "HTML(KB)", "CSS(KB)", "JS(KB)", "图片(KB)", "字体(KB)",
    ];

    for (col, h) in headers.iter().enumerate() {
        sheet.write_with_format(0, col as u16, *h, &header_fmt)?;
    }

    // === 数据行 ===
    for (i, r) in db_data.iter().enumerate() {
        let row = (i + 1) as u32;
        let has_error = r.error_msg.is_some();
        let row_fmt = if has_error { &fail_fmt } else { &success_fmt };

        sheet.write_with_format(row, 0, (i + 1) as u32, row_fmt)?;
        sheet.write_with_format(row, 1, &r.url, &url_fmt)?;
        write_num(sheet, row, 2, r.dns_time_ms, row_fmt)?;
        write_ok(sheet, row, 3, r.dns_success, row_fmt)?;
        write_num(sheet, row, 4, r.tcp_time_ms, row_fmt)?;
        write_success(sheet, row, 5, r.error_msg.is_none(), row_fmt)?;
        write_num(sheet, row, 6, r.ttfb_ms, row_fmt)?;
        write_num(sheet, row, 7, r.dom_content_loaded_ms, row_fmt)?;
        write_num(sheet, row, 8, r.load_event_ms, row_fmt)?;
        write_num_i32(sheet, row, 9, r.total_requests, row_fmt)?;
        write_kb(sheet, row, 10, r.html_size, row_fmt)?;
        write_kb(sheet, row, 11, r.css_size, row_fmt)?;
        write_kb(sheet, row, 12, r.js_size, row_fmt)?;
        write_kb(sheet, row, 13, r.image_size, row_fmt)?;
        write_kb(sheet, row, 14, r.font_size, row_fmt)?;
    }

    // === 自动列宽 ===
    for col in 0..headers.len() as u16 {
        let max_len = std::cmp::max(
            headers[col as usize].len() as u16,
            db_data.iter().fold(0u16, |acc, r| {
                let s = match col { 1 => r.url.len() as u16, _ => 10 };
                acc.max(s)
            }),
        );
        sheet.set_column_width(col, (max_len + 2).clamp(8, 50) as f64)?;
    }

    // === 冻结首行 ===
    sheet.set_freeze_panes(1, 0)?;

    workbook.save(&filename)?;
    Ok(filename)
}

/// 导出视频测试结果为 Excel
pub fn export_video_xlsx(
    db_data: &[VideoResult],
    task_id: &str,
    output_dir: &str,
) -> anyhow::Result<String> {
    let filename = format!("{}/video_{}_{}.xlsx", output_dir, task_id, Utc::now().timestamp());
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();

    let header_fmt = Format::new().set_bold().set_font_color(Color::White)
        .set_background_color(Color::RGB(0x4472C4)).set_border(FormatBorder::Thin);
    let cell_fmt = Format::new().set_border(FormatBorder::Thin);
    let ok_fmt = Format::new().set_border(FormatBorder::Thin).set_font_color(Color::RGB(0x008000));
    let err_fmt = Format::new().set_border(FormatBorder::Thin).set_font_color(Color::RGB(0xFF0000));

    let headers = [
        "序号", "URL", "平台", "DNS解析时延(ms)", "DNS解析成功率(%)", "TCP连接时延(ms)", "HTTP响应时延(ms)",
        "视频首次播放时延(ms)", "缓冲次数", "缓冲时间(ms)", "视频卡顿率(%)",
        "视频播放成功率(%)", "视频下载速率(Mbps)", "视频大小(B)", "视频时长(ms)",
        "丢帧", "解码帧", "页面标题", "截图", "错误",
    ];

    for (col, h) in headers.iter().enumerate() {
        sheet.write_with_format(0, col as u16, *h, &header_fmt)?;
    }

    for (i, r) in db_data.iter().enumerate() {
        let row = (i + 1) as u32;
        let has_err = r.error_msg.is_some();
        let rf = if has_err { &err_fmt } else { &ok_fmt };

        sheet.write_with_format(row, 0, (i + 1) as u32, rf)?;
        sheet.write_with_format(row, 1, &r.url, rf)?;
        sheet.write_with_format(row, 2, r.platform.as_deref().unwrap_or("-"), rf)?;
        write_num(sheet, row, 3, r.first_play_time_ms, rf)?;
        write_num_i32(sheet, row, 4, r.buffer_count, rf)?;
        write_num(sheet, row, 5, r.total_buffer_time_ms, rf)?;
        sheet.write_with_format(row, 6, if r.play_success == Some(1) { "成功" } else { "失败" }, rf)?;
        write_num(sheet, row, 7, r.video_download_speed, rf)?;
        write_num_i32(sheet, row, 8, r.video_size, rf)?;
        write_num(sheet, row, 9, r.video_duration_ms, rf)?;
        write_num_i32(sheet, row, 10, r.dropped_frames, rf)?;
        write_num_i32(sheet, row, 11, r.decoded_frames, rf)?;
        sheet.write_with_format(row, 12, r.page_title.as_deref().unwrap_or("-"), rf)?;
        sheet.write_with_format(row, 13, r.screenshot_path.as_deref().unwrap_or("-"), rf)?;
        sheet.write_with_format(row, 14, r.error_msg.as_deref().unwrap_or(""), rf)?;
    }

    for col in 0..headers.len() as u16 {
        sheet.set_column_width(col, 14.0)?;
    }
    sheet.set_freeze_panes(1, 0)?;
    workbook.save(&filename)?;
    Ok(filename)
}

/// 导出下载测试结果为 Excel
pub fn export_download_xlsx(
    db_data: &[DownloadResult],
    task_id: &str,
    output_dir: &str,
) -> anyhow::Result<String> {
    let filename = format!("{}/download_{}_{}.xlsx", output_dir, task_id, Utc::now().timestamp());
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();

    let header_fmt = Format::new().set_bold().set_font_color(Color::White)
        .set_background_color(Color::RGB(0x4472C4)).set_border(FormatBorder::Thin);
    let ok_fmt = Format::new().set_border(FormatBorder::Thin).set_font_color(Color::RGB(0x008000));
    let err_fmt = Format::new().set_border(FormatBorder::Thin).set_font_color(Color::RGB(0xFF0000));

    let headers = [
        "序号", "URL", "文件DNS时延(ms)", "DNS解析成功率(%)", "文件TCP连接时延(ms)",
        "文件下载速率(Mbps)", "文件下载成功率(%)",
    ];

    for (col, h) in headers.iter().enumerate() {
        sheet.write_with_format(0, col as u16, *h, &header_fmt)?;
    }

    for (i, r) in db_data.iter().enumerate() {
        let row = (i + 1) as u32;
        let ok = r.success == Some(1);
        let rf = if ok { &ok_fmt } else { &err_fmt };

        sheet.write_with_format(row, 0, (i + 1) as u32, rf)?;
        sheet.write_with_format(row, 1, &r.url, rf)?;
        write_num(sheet, row, 2, r.dns_time_ms, rf)?;
        write_ok(sheet, row, 3, r.dns_success, rf)?;
        write_num(sheet, row, 4, r.tcp_time_ms, rf)?;
        // KB/s → Mbps: ÷125
        let speed_mbps = r.download_speed.map(|s| s / 125.0);
        write_num(sheet, row, 5, speed_mbps, rf)?;
        write_success(sheet, row, 6, ok, rf)?;
    }

    for col in 0..headers.len() as u16 {
        sheet.set_column_width(col, 18.0)?;
    }
    sheet.set_freeze_panes(1, 0)?;
    workbook.save(&filename)?;
    Ok(filename)
}

/// 导出 Ping 测试结果为 Excel
pub fn export_ping_xlsx(
    db_data: &[PingResult],
    task_id: &str,
    output_dir: &str,
) -> anyhow::Result<String> {
    let filename = format!("{}/ping_{}_{}.xlsx", output_dir, task_id, Utc::now().timestamp());
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();

    let header_fmt = Format::new().set_bold().set_font_color(Color::White)
        .set_background_color(Color::RGB(0x4472C4)).set_border(FormatBorder::Thin);
    let ok_fmt = Format::new().set_border(FormatBorder::Thin).set_font_color(Color::RGB(0x008000));
    let err_fmt = Format::new().set_border(FormatBorder::Thin).set_font_color(Color::RGB(0xFF0000));

    let headers = [
        "序号", "目标主机", "平均时延(ms)", "丢包率(%)", "抖动(ms)", "成功率(%)", "错误",
    ];

    for (col, h) in headers.iter().enumerate() {
        sheet.write_with_format(0, col as u16, *h, &header_fmt)?;
    }

    for (i, r) in db_data.iter().enumerate() {
        let row = (i + 1) as u32;
        let ok = r.success == Some(1);
        let rf = if ok { &ok_fmt } else { &err_fmt };

        sheet.write_with_format(row, 0, (i + 1) as u32, rf)?;
        sheet.write_with_format(row, 1, &r.host, rf)?;
        write_num(sheet, row, 2, r.avg_latency_ms, rf)?;
        write_num(sheet, row, 3, r.packet_loss_rate, rf)?;
        write_num(sheet, row, 4, r.jitter_ms, rf)?;
        write_num_i32(sheet, row, 5, r.success, rf)?;
        sheet.write_with_format(row, 6, r.error_msg.as_deref().unwrap_or(""), rf)?;
    }

    for col in 0..headers.len() as u16 {
        sheet.set_column_width(col, 18.0)?;
    }
    sheet.set_freeze_panes(1, 0)?;
    workbook.save(&filename)?;
    Ok(filename)
}

// === 辅助函数 ===

fn write_num(sheet: &mut Worksheet, row: u32, col: u16, val: Option<f64>, fmt: &Format) -> Result<(), XlsxError> {
    match val {
        Some(v) => { sheet.write_with_format(row, col, v, fmt)?; Ok(()) }
        None => { sheet.write_with_format(row, col, "-", fmt)?; Ok(()) }
    }
}

fn write_num_i32(sheet: &mut Worksheet, row: u32, col: u16, val: Option<i32>, fmt: &Format) -> Result<(), XlsxError> {
    match val {
        Some(v) => { sheet.write_with_format(row, col, v, fmt)?; Ok(()) }
        None => { sheet.write_with_format(row, col, "-", fmt)?; Ok(()) }
    }
}

fn write_ok(sheet: &mut Worksheet, row: u32, col: u16, val: Option<i32>, fmt: &Format) -> Result<(), XlsxError> {
    match val {
        Some(v) => { sheet.write_with_format(row, col, v, fmt)?; Ok(()) }
        None => { sheet.write_with_format(row, col, "-", fmt)?; Ok(()) }
    }
}

fn write_success(sheet: &mut Worksheet, row: u32, col: u16, ok: bool, fmt: &Format) -> Result<(), XlsxError> {
    sheet.write_with_format(row, col, if ok { "100" } else { "0" }, fmt)?;
    Ok(())
}

fn write_kb(sheet: &mut Worksheet, row: u32, col: u16, val_bytes: Option<i32>, fmt: &Format) -> Result<(), XlsxError> {
    match val_bytes {
        Some(v) => { sheet.write_with_format(row, col, (v as f64 / 1024.0), fmt)?; Ok(()) }
        None => { sheet.write_with_format(row, col, "-", fmt)?; Ok(()) }
    }
}

// === CSV / JSON 导出 ===

pub fn export_csv<T: serde::Serialize>(data: &[T], task_id: &str, output_dir: &str, prefix: &str) -> anyhow::Result<String> {
    let filename = format!("{}/{}_{}_{}.csv", output_dir, prefix, task_id, Utc::now().timestamp());
    let mut wtr = csv::Writer::from_path(&filename)?;
    for row in data {
        wtr.serialize(row)?;
    }
    wtr.flush()?;
    Ok(filename)
}

pub fn export_json(data: &impl serde::Serialize) -> anyhow::Result<Vec<u8>> {
    Ok(serde_json::to_vec_pretty(data)?)
}

// === 计划运行合并导出 ===

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TaskSummary {
    pub task_id: String,
    pub task_type: String,
    pub status: String,
    pub url: String,
}

pub fn export_plan_run_xlsx(
    summaries: &[TaskSummary],
    website_data: &[WebsiteResult],
    video_data: &[VideoResult],
    download_data: &[DownloadResult],
    ping_data: &[PingResult],
    plan_id: &str,
    run_id: &str,
    output_dir: &str,
) -> anyhow::Result<String> {
    let filename = format!("{}/planrun_{}_{}_{}.xlsx", output_dir,
        plan_id.chars().take(8).collect::<String>(),
        run_id.chars().take(8).collect::<String>(),
        Utc::now().timestamp());
    let mut workbook = Workbook::new();

    let header_fmt = Format::new().set_bold().set_font_color(Color::White)
        .set_background_color(Color::RGB(0x4472C4)).set_border(FormatBorder::Thin);
    let ok_fmt = Format::new().set_border(FormatBorder::Thin).set_font_color(Color::RGB(0x008000));
    let err_fmt = Format::new().set_border(FormatBorder::Thin).set_font_color(Color::RGB(0xFF0000));
    let url_fmt = Format::new().set_border(FormatBorder::Thin).set_font_color(Color::RGB(0x0563C1));

    // 任务概览表
    let summary = workbook.add_worksheet().set_name("任务概览")?;
    let headers = ["任务ID", "任务类型", "状态", "URL"];
    for (col, h) in headers.iter().enumerate() {
        summary.write_with_format(0, col as u16, *h, &header_fmt)?;
    }
    for (i, s) in summaries.iter().enumerate() {
        let row = (i + 1) as u32;
        let fmt: &Format = if s.status == "completed" { &ok_fmt } else if s.status == "failed" { &err_fmt } else { &ok_fmt };
        let type_label = match s.task_type.as_str() { "website" => "网站测试", "video" => "视频测试", "download" => "下载测试", "ping" => "Ping测试", _ => &s.task_type };
        summary.write_with_format(row, 0, &s.task_id, fmt)?;
        summary.write_with_format(row, 1, type_label, fmt)?;
        summary.write_with_format(row, 2, &s.status, fmt)?;
        summary.write_with_format(row, 3, &s.url, &url_fmt)?;
    }
    for col in 0..4u16 {
        summary.set_column_width(col, 22.0)?;
    }
    summary.set_freeze_panes(1, 0)?;

    // 网站测试结果
    if !website_data.is_empty() {
        let sheet = workbook.add_worksheet().set_name("网站测试")?;
        let headers = [
            "URL", "DNS解析时延(ms)", "DNS解析成功率(%)", "TCP连接时延(ms)",
            "访问成功率(%)", "首包时延(ms)", "首屏时延(ms)", "首页时延(ms)",
            "总请求", "HTML(KB)", "CSS(KB)", "JS(KB)", "图片(KB)", "字体(KB)",
        ];
        for (col, h) in headers.iter().enumerate() {
            sheet.write_with_format(0, col as u16, *h, &header_fmt)?;
        }
        for (i, r) in website_data.iter().enumerate() {
            let row = (i + 1) as u32;
            let has_err = r.error_msg.is_some();
            let fmt = if has_err { &err_fmt } else { &ok_fmt };
            sheet.write_with_format(row, 0, &r.url, &url_fmt)?;
            write_num(sheet, row, 1, r.dns_time_ms, fmt)?;
            write_ok(sheet, row, 2, r.dns_success, fmt)?;
            write_num(sheet, row, 3, r.tcp_time_ms, fmt)?;
            write_success(sheet, row, 4, !has_err, fmt)?;
            write_num(sheet, row, 5, r.ttfb_ms, fmt)?;
            write_num(sheet, row, 6, r.dom_content_loaded_ms, fmt)?;
            write_num(sheet, row, 7, r.load_event_ms, fmt)?;
            write_num_i32(sheet, row, 8, r.total_requests, fmt)?;
            write_kb(sheet, row, 9, r.html_size, fmt)?;
            write_kb(sheet, row, 10, r.css_size, fmt)?;
            write_kb(sheet, row, 11, r.js_size, fmt)?;
            write_kb(sheet, row, 12, r.image_size, fmt)?;
            write_kb(sheet, row, 13, r.font_size, fmt)?;
        }
        for col in 0..headers.len() as u16 { sheet.set_column_width(col, 14.0)?; }
        sheet.set_freeze_panes(1, 0)?;
    }

    // 视频测试结果
    if !video_data.is_empty() {
        let sheet = workbook.add_worksheet().set_name("视频测试")?;
        let headers = [
            "URL", "平台", "DNS解析时延(ms)", "DNS成功率(%)", "TCP连接时延(ms)", "HTTP响应时延(ms)",
            "首次播放时延(ms)", "缓冲次数", "缓冲时间(ms)", "卡顿率(%)",
            "下载速率(KB/s)", "大小(B)", "时长(ms)",
            "丢帧", "解码帧", "页面标题", "错误",
        ];
        for (col, h) in headers.iter().enumerate() {
            sheet.write_with_format(0, col as u16, *h, &header_fmt)?;
        }
        for (i, r) in video_data.iter().enumerate() {
            let row = (i + 1) as u32;
            let ok = r.play_success == Some(1);
            let fmt = if ok { &ok_fmt } else { &err_fmt };
            sheet.write_with_format(row, 0, &r.url, &url_fmt)?;
            sheet.write_with_format(row, 1, r.platform.as_deref().unwrap_or("-"), fmt)?;
            write_num(sheet, row, 2, r.dns_time_ms, fmt)?;
            write_ok(sheet, row, 3, r.dns_success, fmt)?;
            write_num(sheet, row, 4, r.tcp_time_ms, fmt)?;
            write_num(sheet, row, 5, r.http_response_ms, fmt)?;
            write_num(sheet, row, 6, r.first_play_time_ms, fmt)?;
            write_num_i32(sheet, row, 7, r.buffer_count, fmt)?;
            write_num(sheet, row, 8, r.total_buffer_time_ms, fmt)?;
            write_num(sheet, row, 9, r.buffer_rate, fmt)?;
            write_num(sheet, row, 10, r.video_download_speed, fmt)?;
            write_num_i32(sheet, row, 11, r.video_size, fmt)?;
            write_num(sheet, row, 12, r.video_duration_ms, fmt)?;
            write_num_i32(sheet, row, 13, r.dropped_frames, fmt)?;
            write_num_i32(sheet, row, 14, r.decoded_frames, fmt)?;
            sheet.write_with_format(row, 15, r.page_title.as_deref().unwrap_or("-"), fmt)?;
            sheet.write_with_format(row, 16, r.error_msg.as_deref().unwrap_or(""), fmt)?;
        }
        for col in 0..headers.len() as u16 { sheet.set_column_width(col, 16.0)?; }
        sheet.set_freeze_panes(1, 0)?;
    }

    // 下载测试结果
    if !download_data.is_empty() {
        let sheet = workbook.add_worksheet().set_name("下载测试")?;
        let headers = [
            "URL", "文件DNS时延(ms)", "DNS解析成功率(%)", "文件TCP连接时延(ms)",
            "文件下载速率(Mbps)", "文件下载成功率(%)",
        ];
        for (col, h) in headers.iter().enumerate() {
            sheet.write_with_format(0, col as u16, *h, &header_fmt)?;
        }
        for (i, r) in download_data.iter().enumerate() {
            let row = (i + 1) as u32;
            let ok = r.success == Some(1);
            let fmt = if ok { &ok_fmt } else { &err_fmt };
            sheet.write_with_format(row, 0, &r.url, &url_fmt)?;
            write_num(sheet, row, 1, r.dns_time_ms, fmt)?;
            write_ok(sheet, row, 2, r.dns_success, fmt)?;
            write_num(sheet, row, 3, r.tcp_time_ms, fmt)?;
            let speed_mbps = r.download_speed.map(|s| s / 125.0);
            write_num(sheet, row, 4, speed_mbps, fmt)?;
            write_success(sheet, row, 5, ok, fmt)?;
        }
        for col in 0..headers.len() as u16 { sheet.set_column_width(col, 18.0)?; }
        sheet.set_freeze_panes(1, 0)?;
    }

    // Ping 测试结果
    if !ping_data.is_empty() {
        let sheet = workbook.add_worksheet().set_name("Ping测试")?;
        let headers = ["目标主机", "平均时延(ms)", "丢包率(%)", "抖动(ms)", "结果", "错误"];
        for (col, h) in headers.iter().enumerate() {
            sheet.write_with_format(0, col as u16, *h, &header_fmt)?;
        }
        for (i, r) in ping_data.iter().enumerate() {
            let row = (i + 1) as u32;
            let ok = r.success == Some(1);
            let fmt = if ok { &ok_fmt } else { &err_fmt };
            sheet.write_with_format(row, 0, &r.host, &url_fmt)?;
            write_num(sheet, row, 1, r.avg_latency_ms, fmt)?;
            write_num(sheet, row, 2, r.packet_loss_rate, fmt)?;
            write_num(sheet, row, 3, r.jitter_ms, fmt)?;
            sheet.write_with_format(row, 4, if ok { "成功" } else { "失败" }, fmt)?;
            sheet.write_with_format(row, 5, r.error_msg.as_deref().unwrap_or(""), fmt)?;
        }
        for col in 0..headers.len() as u16 { sheet.set_column_width(col, 18.0)?; }
        sheet.set_freeze_panes(1, 0)?;
    }

    workbook.save(&filename)?;
    Ok(filename)
}
