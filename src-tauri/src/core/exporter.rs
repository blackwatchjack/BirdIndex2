use crate::core::types::{ExportRequest, ExportResponse};
use anyhow::{anyhow, Context, Result};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

const MAX_DATA_ROWS: usize = 1_048_575;

pub fn export_manifest(request: &ExportRequest) -> Result<ExportResponse> {
    let species_count = species_count(request);
    if species_count > MAX_DATA_ROWS {
        return Err(anyhow!(
            "清单包含 {species_count} 个物种，超过 Excel 单工作表上限 {MAX_DATA_ROWS}"
        ));
    }

    let destination = normalized_destination(&request.destination)?;
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    if !parent.exists() {
        return Err(anyhow!("导出目录不存在：{}", parent.display()));
    }

    let temporary = temporary_path(&destination);
    if let Err(error) = write_workbook(&temporary, request, species_count) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error);
    }

    if let Err(error) = install_workbook(&temporary, &destination) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error);
    }

    Ok(ExportResponse {
        path: destination.to_string_lossy().to_string(),
        species_count,
    })
}

fn normalized_destination(value: &str) -> Result<PathBuf> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("导出路径不能为空"));
    }

    let mut path = PathBuf::from(trimmed);
    let has_xlsx_extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("xlsx"));
    if !has_xlsx_extension {
        path.set_extension("xlsx");
    }
    Ok(path)
}

fn temporary_path(destination: &Path) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("BirdIndex2-export.xlsx");
    destination.with_file_name(format!(".{file_name}.{timestamp}.tmp"))
}

fn backup_path(destination: &Path) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("BirdIndex2-export.xlsx");
    destination.with_file_name(format!(".{file_name}.{timestamp}.backup"))
}

fn install_workbook(temporary: &Path, destination: &Path) -> Result<()> {
    if !destination.exists() {
        return std::fs::rename(temporary, destination)
            .with_context(|| format!("无法保存导出文件：{}", destination.display()));
    }

    let backup = backup_path(destination);
    std::fs::rename(destination, &backup)
        .with_context(|| format!("无法准备覆盖已有文件：{}", destination.display()))?;

    if let Err(error) = std::fs::rename(temporary, destination) {
        let restore_result = std::fs::rename(&backup, destination);
        return match restore_result {
            Ok(()) => Err(error)
                .with_context(|| format!("无法覆盖已有导出文件：{}", destination.display())),
            Err(restore_error) => Err(anyhow!(
                "无法覆盖已有导出文件，且原文件恢复失败。备份位于 {}：写入错误：{}；恢复错误：{}",
                backup.display(),
                error,
                restore_error
            )),
        };
    }

    std::fs::remove_file(&backup)
        .with_context(|| format!("导出成功，但无法清理旧文件备份：{}", backup.display()))?;
    Ok(())
}

fn species_count(request: &ExportRequest) -> usize {
    request
        .scan
        .tree
        .orders
        .iter()
        .flat_map(|order| &order.families)
        .flat_map(|family| &family.genera)
        .flat_map(|genus| &genus.species)
        .count()
}

fn write_workbook(path: &Path, request: &ExportRequest, species_count: usize) -> Result<()> {
    let file =
        File::create(path).with_context(|| format!("无法创建临时导出文件：{}", path.display()))?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    write_static_file(&mut zip, "[Content_Types].xml", CONTENT_TYPES, options)?;
    write_static_file(&mut zip, "_rels/.rels", PACKAGE_RELS, options)?;
    write_static_file(&mut zip, "xl/workbook.xml", WORKBOOK, options)?;
    write_static_file(
        &mut zip,
        "xl/_rels/workbook.xml.rels",
        WORKBOOK_RELS,
        options,
    )?;
    write_static_file(&mut zip, "xl/styles.xml", STYLES, options)?;
    write_summary_sheet(&mut zip, request, options)?;
    write_species_sheet(&mut zip, request, species_count, options)?;

    zip.finish().context("无法完成 XLSX 文件写入")?;
    Ok(())
}

fn write_static_file(
    zip: &mut ZipWriter<File>,
    name: &str,
    content: &str,
    options: FileOptions,
) -> Result<()> {
    zip.start_file(name, options)
        .with_context(|| format!("无法创建 XLSX 内容：{name}"))?;
    zip.write_all(content.as_bytes())
        .with_context(|| format!("无法写入 XLSX 内容：{name}"))?;
    Ok(())
}

fn write_summary_sheet(
    zip: &mut ZipWriter<File>,
    request: &ExportRequest,
    options: FileOptions,
) -> Result<()> {
    zip.start_file("xl/worksheets/sheet1.xml", options)
        .context("无法创建扫描摘要工作表")?;

    let roots = if request.scan.roots.is_empty() {
        vec!["未提供扫描目录".to_string()]
    } else {
        request.scan.roots.clone()
    };
    let note_row = 11 + roots.len();
    let last_row = note_row;

    write!(
        zip,
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><dimension ref="A1:H{last_row}"/><sheetViews><sheetView showGridLines="0" workbookViewId="0"/></sheetViews><sheetFormatPr defaultRowHeight="18"/><cols><col min="1" max="1" width="18" customWidth="1"/><col min="2" max="2" width="14" customWidth="1"/><col min="3" max="3" width="4" customWidth="1"/><col min="4" max="4" width="18" customWidth="1"/><col min="5" max="5" width="14" customWidth="1"/><col min="6" max="6" width="4" customWidth="1"/><col min="7" max="7" width="18" customWidth="1"/><col min="8" max="8" width="14" customWidth="1"/></cols><sheetData>"#
    )?;

    write!(zip, r#"<row r="1" ht="34" customHeight="1">"#)?;
    write_inline_cell(zip, "A1", "BirdIndex2 物种清单", 1)?;
    write!(zip, "</row>")?;

    write!(zip, r#"<row r="3" ht="24" customHeight="1">"#)?;
    write_inline_cell(zip, "A3", "扫描文件", 3)?;
    write_number_cell(zip, "B3", request.scan.stats.total_files, 4)?;
    write_inline_cell(zip, "D3", "命中文件", 3)?;
    write_number_cell(zip, "E3", request.scan.stats.matched_files, 4)?;
    write_inline_cell(zip, "G3", "命中物种", 3)?;
    write_number_cell(zip, "H3", request.scan.stats.matched_species, 4)?;
    write!(zip, "</row>")?;

    write!(zip, r#"<row r="5" ht="24" customHeight="1">"#)?;
    write_inline_cell(zip, "A5", "未命中文件", 3)?;
    write_number_cell(zip, "B5", request.scan.stats.unmatched_files, 4)?;
    write_inline_cell(zip, "D5", "IOC 物种数", 3)?;
    write_number_cell(zip, "E5", request.scan.total_species, 4)?;
    write_inline_cell(zip, "G5", "扫描目录", 3)?;
    write_number_cell(zip, "H5", request.scan.roots.len(), 4)?;
    write!(zip, "</row>")?;

    write!(zip, r#"<row r="7" ht="24" customHeight="1">"#)?;
    write_inline_cell(zip, "A7", "导出时间", 3)?;
    write_inline_cell(zip, "B7", &request.exported_at, 6)?;
    write!(zip, "</row>")?;

    write!(zip, r#"<row r="8" ht="24" customHeight="1">"#)?;
    write_inline_cell(zip, "A8", "IOC 数据源", 3)?;
    write_inline_cell(zip, "B8", &request.scan.ioc_source, 6)?;
    write!(zip, "</row>")?;

    write!(zip, r#"<row r="10" ht="24" customHeight="1">"#)?;
    write_inline_cell(zip, "A10", "扫描目录", 2)?;
    write!(zip, "</row>")?;

    for (index, root) in roots.iter().enumerate() {
        let row = 11 + index;
        write!(zip, r#"<row r="{row}" ht="22" customHeight="1">"#)?;
        write_inline_cell(zip, &format!("A{row}"), root, 6)?;
        write!(zip, "</row>")?;
    }

    write!(zip, r#"<row r="{note_row}" ht="24" customHeight="1">"#)?;
    write_inline_cell(
        zip,
        &format!("A{note_row}"),
        "清单基于本次扫描快照生成；原始照片未被修改。",
        8,
    )?;
    write!(zip, "</row></sheetData>")?;

    let merge_count = roots.len() + 5;
    write!(
        zip,
        r#"<mergeCells count="{merge_count}"><mergeCell ref="A1:H1"/><mergeCell ref="B7:H7"/><mergeCell ref="B8:H8"/><mergeCell ref="A10:H10"/>"#
    )?;
    for index in 0..roots.len() {
        let row = 11 + index;
        write!(zip, r#"<mergeCell ref="A{row}:H{row}"/>"#)?;
    }
    write!(
        zip,
        r#"<mergeCell ref="A{note_row}:H{note_row}"/></mergeCells><pageMargins left="0.35" right="0.35" top="0.5" bottom="0.5" header="0.2" footer="0.2"/></worksheet>"#
    )?;
    Ok(())
}

fn write_species_sheet(
    zip: &mut ZipWriter<File>,
    request: &ExportRequest,
    species_count: usize,
    options: FileOptions,
) -> Result<()> {
    zip.start_file("xl/worksheets/sheet2.xml", options)
        .context("无法创建物种清单工作表")?;

    let last_row = if species_count == 0 {
        2
    } else {
        species_count + 1
    };
    write!(
        zip,
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><dimension ref="A1:G{last_row}"/><sheetViews><sheetView showGridLines="0" workbookViewId="0"><pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/><selection pane="bottomLeft" activeCell="A2" sqref="A2"/></sheetView></sheetViews><sheetFormatPr defaultRowHeight="20"/><cols><col min="1" max="1" width="9" customWidth="1"/><col min="2" max="4" width="22" customWidth="1"/><col min="5" max="5" width="20" customWidth="1"/><col min="6" max="6" width="30" customWidth="1"/><col min="7" max="7" width="12" customWidth="1"/></cols><sheetData><row r="1" ht="26" customHeight="1">"#
    )?;

    for (index, header) in ["序号", "目", "科", "属", "中文名", "拉丁名", "照片数"]
        .iter()
        .enumerate()
    {
        write_inline_cell(zip, &format!("{}1", column_name(index)), header, 5)?;
    }
    write!(zip, "</row>")?;

    if species_count == 0 {
        write!(zip, r#"<row r="2" ht="24" customHeight="1">"#)?;
        write_inline_cell(zip, "A2", "本次扫描没有命中的物种", 8)?;
        write!(zip, "</row>")?;
    } else {
        let mut sequence = 0usize;
        for order in &request.scan.tree.orders {
            for family in &order.families {
                for genus in &family.genera {
                    for species in &genus.species {
                        sequence += 1;
                        let row = sequence + 1;
                        let (text_style, number_style) =
                            if sequence & 1 == 0 { (9, 10) } else { (6, 7) };
                        write!(zip, r#"<row r="{row}" ht="20" customHeight="1">"#)?;
                        write_number_cell(zip, &format!("A{row}"), sequence, number_style)?;
                        write_inline_cell(zip, &format!("B{row}"), &order.name, text_style)?;
                        write_inline_cell(zip, &format!("C{row}"), &family.name, text_style)?;
                        write_inline_cell(zip, &format!("D{row}"), &genus.name, text_style)?;
                        write_inline_cell(zip, &format!("E{row}"), &species.chinese, text_style)?;
                        write_inline_cell(zip, &format!("F{row}"), &species.latin, text_style)?;
                        write_number_cell(
                            zip,
                            &format!("G{row}"),
                            species.photos.len(),
                            number_style,
                        )?;
                        write!(zip, "</row>")?;
                    }
                }
            }
        }
    }

    write!(zip, "</sheetData>")?;
    if species_count == 0 {
        write!(
            zip,
            r#"<mergeCells count="1"><mergeCell ref="A2:G2"/></mergeCells><autoFilter ref="A1:G1"/>"#
        )?;
    } else {
        write!(zip, r#"<autoFilter ref="A1:G{last_row}"/>"#)?;
    }
    write!(
        zip,
        r#"<pageMargins left="0.25" right="0.25" top="0.5" bottom="0.5" header="0.2" footer="0.2"/><pageSetup orientation="landscape" fitToWidth="1" fitToHeight="0"/></worksheet>"#
    )?;
    Ok(())
}

fn write_inline_cell(
    writer: &mut impl Write,
    reference: &str,
    value: &str,
    style: usize,
) -> std::io::Result<()> {
    write!(
        writer,
        r#"<c r="{}" s="{}" t="inlineStr"><is><t xml:space="preserve">{}</t></is></c>"#,
        reference,
        style,
        escape_xml(value)
    )
}

fn write_number_cell(
    writer: &mut impl Write,
    reference: &str,
    value: usize,
    style: usize,
) -> std::io::Result<()> {
    write!(
        writer,
        r#"<c r="{}" s="{}"><v>{}</v></c>"#,
        reference, style, value
    )
}

fn escape_xml(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            '\t' | '\n' | '\r' => escaped.push(character),
            character if character >= ' ' => escaped.push(character),
            _ => {}
        }
    }
    escaped
}

fn column_name(mut index: usize) -> String {
    let mut name = String::new();
    loop {
        name.insert(0, (b'A' + (index % 26) as u8) as char);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    name
}

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/worksheets/sheet2.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>"#;

const PACKAGE_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#;

const WORKBOOK: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><bookViews><workbookView activeTab="1"/></bookViews><sheets><sheet name="扫描摘要" sheetId="1" r:id="rId1"/><sheet name="物种清单" sheetId="2" r:id="rId2"/></sheets><calcPr calcId="191029" fullCalcOnLoad="1"/></workbook>"#;

const WORKBOOK_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet2.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#;

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><fonts count="5"><font><sz val="11"/><color theme="1"/><name val="Calibri"/><family val="2"/></font><font><b/><sz val="18"/><color rgb="FFFFFFFF"/><name val="Calibri"/><family val="2"/></font><font><b/><sz val="11"/><color rgb="FFFFFFFF"/><name val="Calibri"/><family val="2"/></font><font><b/><sz val="11"/><color rgb="FF1E4A34"/><name val="Calibri"/><family val="2"/></font><font><i/><sz val="10"/><color rgb="FF5D6775"/><name val="Calibri"/><family val="2"/></font></fonts><fills count="5"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill><fill><patternFill patternType="solid"><fgColor rgb="FF2F6B4B"/><bgColor indexed="64"/></patternFill></fill><fill><patternFill patternType="solid"><fgColor rgb="FFE8F1EC"/><bgColor indexed="64"/></patternFill></fill><fill><patternFill patternType="solid"><fgColor rgb="FFF7F5F2"/><bgColor indexed="64"/></patternFill></fill></fills><borders count="3"><border><left/><right/><top/><bottom/><diagonal/></border><border><left/><right/><top/><bottom style="thin"><color rgb="FFDDE4E0"/></bottom><diagonal/></border><border><left/><right/><top/><bottom style="medium"><color rgb="FF26553D"/></bottom><diagonal/></border></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="11"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/><xf numFmtId="0" fontId="1" fillId="2" borderId="0" xfId="0" applyFont="1" applyFill="1" applyAlignment="1"><alignment horizontal="left" vertical="center"/></xf><xf numFmtId="0" fontId="3" fillId="3" borderId="1" xfId="0" applyFont="1" applyFill="1" applyBorder="1" applyAlignment="1"><alignment vertical="center"/></xf><xf numFmtId="0" fontId="3" fillId="3" borderId="1" xfId="0" applyFont="1" applyFill="1" applyBorder="1" applyAlignment="1"><alignment vertical="center"/></xf><xf numFmtId="3" fontId="3" fillId="3" borderId="1" xfId="0" applyNumberFormat="1" applyFont="1" applyFill="1" applyBorder="1" applyAlignment="1"><alignment horizontal="center" vertical="center"/></xf><xf numFmtId="0" fontId="2" fillId="2" borderId="2" xfId="0" applyFont="1" applyFill="1" applyBorder="1" applyAlignment="1"><alignment horizontal="center" vertical="center"/></xf><xf numFmtId="0" fontId="0" fillId="0" borderId="1" xfId="0" applyBorder="1" applyAlignment="1"><alignment vertical="center"/></xf><xf numFmtId="3" fontId="0" fillId="0" borderId="1" xfId="0" applyNumberFormat="1" applyBorder="1" applyAlignment="1"><alignment horizontal="center" vertical="center"/></xf><xf numFmtId="0" fontId="4" fillId="0" borderId="0" xfId="0" applyFont="1" applyAlignment="1"><alignment vertical="center"/></xf><xf numFmtId="0" fontId="0" fillId="4" borderId="1" xfId="0" applyFill="1" applyBorder="1" applyAlignment="1"><alignment vertical="center"/></xf><xf numFmtId="3" fontId="0" fillId="4" borderId="1" xfId="0" applyNumberFormat="1" applyFill="1" applyBorder="1" applyAlignment="1"><alignment horizontal="center" vertical="center"/></xf></cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles></styleSheet>"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{
        FamilyNode, GenusNode, OrderNode, PhotoItem, ScanResponse, ScanStats, SpeciesNode,
        TaxonTree,
    };
    use calamine::{open_workbook_auto, DataType, Reader};

    #[test]
    fn exports_readable_xlsx_with_one_row_per_species() {
        let base = std::env::temp_dir().join(format!(
            "birdindex2-export-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&base).unwrap();
        let destination_without_extension = base.join("鸟类清单");
        let request = sample_request(destination_without_extension.to_string_lossy().to_string());

        let response = export_manifest(&request).unwrap();

        assert_eq!(response.species_count, 1);
        assert!(response.path.ends_with("鸟类清单.xlsx"));
        let mut workbook = open_workbook_auto(&response.path).unwrap();
        assert_eq!(workbook.sheet_names(), &["扫描摘要", "物种清单"]);

        let summary = workbook.worksheet_range("扫描摘要").unwrap();
        assert_eq!(
            summary.get_value((0, 0)),
            Some(&DataType::String("BirdIndex2 物种清单".into()))
        );
        assert_eq!(summary.get_value((2, 1)), Some(&DataType::Float(3.0)));
        assert_eq!(
            summary.get_value((6, 1)),
            Some(&DataType::String("2026-08-02 12:34:56".into()))
        );
        assert_eq!(
            summary.get_value((7, 1)),
            Some(&DataType::String(
                "/应用资源/Multiling IOC 15.1_d.xlsx".into()
            ))
        );
        assert_eq!(
            summary.get_value((10, 0)),
            Some(&DataType::String("/照片/<精选>".into()))
        );

        let manifest = workbook.worksheet_range("物种清单").unwrap();
        assert_eq!(
            manifest.get_value((0, 0)),
            Some(&DataType::String("序号".into()))
        );
        assert_eq!(
            manifest.get_value((1, 4)),
            Some(&DataType::String("白头鹎".into()))
        );
        assert_eq!(
            manifest.get_value((1, 5)),
            Some(&DataType::String("Pycnonotus sinensis".into()))
        );
        assert_eq!(manifest.get_value((1, 6)), Some(&DataType::Float(2.0)));
        assert_eq!(manifest.height(), 2);

        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn exports_zero_match_scan_with_headers_only() {
        let base = std::env::temp_dir().join(format!(
            "birdindex2-empty-export-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&base).unwrap();
        let destination = base.join("空物种清单.xlsx");
        let mut request = sample_request(destination.to_string_lossy().to_string());
        request.scan.tree.orders.clear();
        request.scan.stats.matched_files = 0;
        request.scan.stats.matched_species = 0;
        request.scan.stats.unmatched_files = request.scan.stats.total_files;

        let response = export_manifest(&request).unwrap();

        assert_eq!(response.species_count, 0);
        let mut workbook = open_workbook_auto(&response.path).unwrap();
        let manifest = workbook.worksheet_range("物种清单").unwrap();
        assert_eq!(
            manifest.get_value((1, 0)),
            Some(&DataType::String("本次扫描没有命中的物种".into()))
        );

        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn safely_replaces_an_existing_workbook() {
        let base = std::env::temp_dir().join(format!(
            "birdindex2-overwrite-export-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&base).unwrap();
        let destination = base.join("已有清单.xlsx");
        std::fs::write(&destination, b"original workbook placeholder").unwrap();
        let request = sample_request(destination.to_string_lossy().to_string());

        let response = export_manifest(&request).unwrap();

        let workbook = open_workbook_auto(&response.path).unwrap();
        assert_eq!(workbook.sheet_names(), &["扫描摘要", "物种清单"]);
        let leftovers: Vec<_> = std::fs::read_dir(&base)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path() != destination)
            .collect();
        assert!(leftovers.is_empty());

        std::fs::remove_dir_all(base).unwrap();
    }

    fn sample_request(destination: String) -> ExportRequest {
        ExportRequest {
            destination,
            exported_at: "2026-08-02 12:34:56".into(),
            scan: ScanResponse {
                tree: TaxonTree {
                    orders: vec![OrderNode {
                        name: "PASSERIFORMES".into(),
                        count: 2,
                        families: vec![FamilyNode {
                            name: "Pycnonotidae".into(),
                            count: 2,
                            genera: vec![GenusNode {
                                name: "Pycnonotus".into(),
                                count: 2,
                                species: vec![SpeciesNode {
                                    latin: "Pycnonotus sinensis".into(),
                                    chinese: "白头鹎".into(),
                                    count: 2,
                                    photos: vec![
                                        PhotoItem {
                                            path: "/照片/<精选>/白头鹎-1.jpg".into(),
                                            file_name: "白头鹎-1.jpg".into(),
                                        },
                                        PhotoItem {
                                            path: "/照片/<精选>/白头鹎 & 竹.jpg".into(),
                                            file_name: "白头鹎 & 竹.jpg".into(),
                                        },
                                    ],
                                }],
                            }],
                        }],
                    }],
                },
                stats: ScanStats {
                    total_files: 3,
                    matched_files: 2,
                    matched_species: 1,
                    unmatched_files: 1,
                },
                total_species: 11_250,
                roots: vec!["/照片/<精选>".into()],
                ioc_source: "/应用资源/Multiling IOC 15.1_d.xlsx".into(),
            },
        }
    }
}
