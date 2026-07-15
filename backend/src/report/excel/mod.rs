use std::path::Path;

use chrono::Utc;
use rust_xlsxwriter::*;
use sqlx::SqlitePool;

use crate::models::task::{DownloadResult, VideoResult, WebsiteResult};

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
        "序号", "URL", "DNS(ms)", "DNS成功", "TCP(ms)", "TLS(ms)", "HTTP状态码",
        "TTFB(ms)", "FP(ms)", "FCP(ms)", "DOM加载(ms)", "Load事件(ms)",
        "页面打开(ms)", "首页绘制(ms)", "资源数量", "资源大小(B)", "最终URL",
        "页面标题", "截图路径", "错误信息",
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
        write_num(sheet, row, 5, r.tls_time_ms, row_fmt)?;
        write_num_i32(sheet, row, 6, r.http_status, row_fmt)?;
        write_num(sheet, row, 7, r.ttfb_ms, row_fmt)?;
        write_num(sheet, row, 8, r.fp_ms, row_fmt)?;
        write_num(sheet, row, 9, r.fcp_ms, row_fmt)?;
        write_num(sheet, row, 10, r.dom_content_loaded_ms, row_fmt)?;
        write_num(sheet, row, 11, r.load_event_ms, row_fmt)?;
        write_num(sheet, row, 12, r.page_open_time_ms, row_fmt)?;
        write_num(sheet, row, 13, r.first_paint_ms, row_fmt)?;
        write_num_i32(sheet, row, 14, r.resource_count, row_fmt)?;
        write_num_i32(sheet, row, 15, r.resource_total_size, row_fmt)?;
        sheet.write_with_format(row, 16, r.final_url.as_deref().unwrap_or("-"), row_fmt)?;
        sheet.write_with_format(row, 17, r.page_title.as_deref().unwrap_or("-"), row_fmt)?;
        sheet.write_with_format(row, 18, r.screenshot_path.as_deref().unwrap_or("-"), row_fmt)?;
        sheet.write_with_format(row, 19, r.error_msg.as_deref().unwrap_or(""), if has_error { &fail_fmt } else { row_fmt })?;
    }

    // === 自动列宽 ===
    for col in 0..headers.len() as u16 {
        let max_len = std::cmp::max(
            headers[col as usize].len() as u16,
            db_data.iter().fold(0u16, |acc, r| {
                let s = match col {
                    1 => r.url.len() as u16,
                    17 => r.page_title.as_ref().map(|t| t.len() as u16).unwrap_or(0),
                    19 => r.error_msg.as_ref().map(|e| e.len() as u16).unwrap_or(0),
                    _ => 10,
                };
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
        "序号", "URL", "平台", "首次播放(ms)", "缓冲次数", "缓冲时间(ms)",
        "播放成功", "下载速度(KB/s)", "视频大小(B)", "视频时长(ms)",
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
        "序号", "URL", "下载速度(KB/s)", "平均速度(KB/s)", "峰值速度(KB/s)",
        "下载时间(ms)", "文件大小(B)", "状态", "错误",
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
        write_num(sheet, row, 2, r.download_speed, rf)?;
        write_num(sheet, row, 3, r.avg_speed, rf)?;
        write_num(sheet, row, 4, r.peak_speed, rf)?;
        write_num(sheet, row, 5, r.download_time_ms, rf)?;
        write_num_i32(sheet, row, 6, r.file_size, rf)?;
        sheet.write_with_format(row, 7, if ok { "成功" } else { "失败" }, rf)?;
        sheet.write_with_format(row, 8, r.error_msg.as_deref().unwrap_or(""), rf)?;
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
        Some(1) => { sheet.write_with_format(row, col, "成功", fmt)?; Ok(()) }
        Some(0) => { sheet.write_with_format(row, col, "失败", fmt)?; Ok(()) }
        _ => { sheet.write_with_format(row, col, "-", fmt)?; Ok(()) }
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
