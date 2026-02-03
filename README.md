# BirdIndex2

BirdIndex2 is a local-only virtual indexing system for bird photos. It builds a strict IOC taxonomic tree from the `Multiling IOC 15.1_d.xlsx` dataset and matches photos by filename, without moving or modifying any files.

## Highlights
- IOC-driven classification: `Order > Family > Genus > Species`
- Read-only indexing: no move/copy/rename/delete
- Fast name-based matching for large collections
- Offline by design

## Data Source
- File: `Multiling IOC 15.1_d.xlsx`
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
- `Species` shows `Chinese + Latin (count)`

## Scan Scope
- Recursively scans user-selected folders
- Image formats: JPG/JPEG/PNG/HEIC
- RAW formats are not included
- No index persistence; full scan on each start

## User Flow
1. Select one or more photo root folders.
2. System parses `List` and scans filenames.
3. Taxonomic tree appears with only matched nodes.
4. Select a photo and use "Locate" to reveal it in Finder/Explorer.

## Non-Functional Requirements
- Target: 100,000 photos indexed within ~1 minute (filename-only scan)
- Strictly read-only access to original files
- Works fully offline
