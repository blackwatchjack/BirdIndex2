# BirdIndex2

BirdIndex2 is a local-only virtual indexing system for bird photos and videos. It builds a strict IOC taxonomic tree from the embedded `Multiling IOC 15.1_d.xlsx` dataset and matches media by filename, without moving or modifying any files.

## Highlights
- IOC-driven classification: `Order > Family > Genus > Species`
- Read-only indexing: no move/copy/rename/delete
- Fast name-based matching for large collections
- Styled Excel species checklist export with image, video, and total-media counts
- Offline by design

## Data Source
- File: `Multiling IOC 15.1_d.xlsx` (bundled with the app)
- Sheet: `List`
- Columns used: `Order`, `Family`, `IOC_15.1`, `Chinese`
- Other language columns are ignored

## Matching Rules
- Case-insensitive matching
- Priority: match `IOC_15.1` first, then `Chinese`
- Single hit classification (no multi-hit conflict handling)
- Genus is derived from the first word of the Latin species name

## Display Rules
- `Order/Family/Genus` are shown in Latin only
- `Order/Family/Genus/Species` counts are total matched media counts
- `Species` shows `Chinese + Latin (total media count)`

## Scan Scope
- Recursively scans user-selected folders
- Image formats: JPG/JPEG/PNG/HEIC
- Video formats: MP4/MOV/M4V/AVI/MKV/WEBM/MTS/M2TS
- Extensions are matched case-insensitively
- RAW formats are not included
- No index persistence; full scan on each start

## User Flow
1. Select one or more media root folders.
2. System parses `List` and scans supported image and video filenames.
3. Taxonomic tree appears with only matched nodes.
4. Select an image or video and use "定位到文件夹" to reveal it in Finder/Explorer. Double-clicking opens the file in the system default application.
5. After a scan, use "导出物种清单" to save an `.xlsx` workbook. The checklist contains one row per matched species with order, family, genus, Chinese and Latin names, plus image, video, and total-media counts.

Videos are indexed by filename only. BirdIndex2 does not inspect video contents, generate video thumbnails, or play video inside the app.

## Non-Functional Requirements
- Target: 100,000 media files indexed within ~1 minute (filename-only scan)
- Strictly read-only access to original files
- Works fully offline
