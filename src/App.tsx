import { useEffect, useMemo, useRef, useState, type ChangeEvent } from "react";
import { invoke, convertFileSrc, isTauri } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";

const IOC_FILE_NAME = "Multiling IOC 15.1_d.xlsx";

type MediaType = "image" | "video";

interface MediaItem {
  path: string;
  file_name: string;
  media_type: MediaType;
}

interface SpeciesNode {
  latin: string;
  chinese: string;
  media_count: number;
  media_items: MediaItem[];
}

interface GenusNode {
  name: string;
  media_count: number;
  species: SpeciesNode[];
}

interface FamilyNode {
  name: string;
  media_count: number;
  genera: GenusNode[];
}

interface OrderNode {
  name: string;
  media_count: number;
  families: FamilyNode[];
}

interface TaxonTree {
  orders: OrderNode[];
}

interface ScanStats {
  total_media: number;
  total_images: number;
  total_videos: number;
  matched_media: number;
  matched_images: number;
  matched_videos: number;
  matched_species: number;
  unmatched_media: number;
  unmatched_images: number;
  unmatched_videos: number;
}

interface ScanResponse {
  tree: TaxonTree;
  stats: ScanStats;
  total_species: number;
  roots: string[];
  ioc_source: string;
}

interface ExportResponse {
  path: string;
  species_count: number;
}

function exportFileName(date = new Date()): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  const hour = String(date.getHours()).padStart(2, "0");
  const minute = String(date.getMinutes()).padStart(2, "0");
  return `BirdIndex2-物种清单-${year}${month}${day}-${hour}${minute}.xlsx`;
}

function exportTimestamp(date = new Date()): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  const hour = String(date.getHours()).padStart(2, "0");
  const minute = String(date.getMinutes()).padStart(2, "0");
  const second = String(date.getSeconds()).padStart(2, "0");
  return `${year}-${month}-${day} ${hour}:${minute}:${second}`;
}

function toThumbnailSrc(path: string): string {
  if (!path) return "";
  try {
    return isTauri() ? convertFileSrc(path) : path;
  } catch {
    return path;
  }
}

export default function App() {
  const [roots, setRoots] = useState<string[]>([]);
  const [scanResult, setScanResult] = useState<ScanResponse | null>(null);
  const [treeQuery, setTreeQuery] = useState("");
  const [selectedSpecies, setSelectedSpecies] = useState<SpeciesNode | null>(null);
  const [selectedMedia, setSelectedMedia] = useState<MediaItem | null>(null);
  const [thumbnailErrorMap, setThumbnailErrorMap] = useState<
    Record<string, boolean>
  >({});
  const [isScanning, setIsScanning] = useState(false);
  const [isChoosingExportPath, setIsChoosingExportPath] = useState(false);
  const [isExporting, setIsExporting] = useState(false);
  const [exportMessage, setExportMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const rootPickerInputRef = useRef<HTMLInputElement | null>(null);
  const exportBusyRef = useRef(false);
  const isExportBusy = isChoosingExportPath || isExporting;

  useEffect(() => {
    const input = rootPickerInputRef.current;
    if (!input) return;
    input.setAttribute("webkitdirectory", "");
    input.setAttribute("directory", "");
  }, []);

  useEffect(() => {
    setThumbnailErrorMap({});
    setSelectedMedia(null);
  }, [selectedSpecies]);

  const rootsLabel = useMemo(() => {
    if (roots.length === 0) return "0 个目录";
    return `${roots.length} 个目录`;
  }, [roots.length]);

  const invalidateScanResult = () => {
    setScanResult(null);
    setTreeQuery("");
    setSelectedSpecies(null);
    setSelectedMedia(null);
    setExportMessage(null);
    setError(null);
  };

  const handleBrowserRootInputChange = (event: ChangeEvent<HTMLInputElement>) => {
    if (isScanning || exportBusyRef.current) {
      event.target.value = "";
      return;
    }
    const files = event.target.files;
    if (!files) return;
    if (files.length === 0) {
      event.target.value = "";
      return;
    }

    const parsedRoots = new Set<string>();
    for (const file of Array.from(files)) {
      const relativePath = file.webkitRelativePath;
      if (!relativePath) continue;
      const [topLevel] = relativePath.split("/");
      if (topLevel) parsedRoots.add(topLevel);
    }

    if (parsedRoots.size === 0) {
      setError("未能从浏览器选择结果中解析目录，请在 Tauri 桌面应用中运行。");
      event.target.value = "";
      return;
    }

    setError(null);
    invalidateScanResult();
    setRoots((prev) => Array.from(new Set([...prev, ...Array.from(parsedRoots)])));
    event.target.value = "";
  };

  const handlePickRoots = async () => {
    if (isScanning || exportBusyRef.current) return;
    setError(null);

    if (!isTauri()) {
      rootPickerInputRef.current?.click();
      return;
    }

    try {
      const selected = await open({
        directory: true,
        multiple: true,
        title: "选择根目录"
      });
      if (!selected) return;
      const next = Array.isArray(selected) ? selected : [selected];
      invalidateScanResult();
      setRoots((prev) => Array.from(new Set([...prev, ...next])));
    } catch (err) {
      setError(`打开目录选择失败：${String(err)}`);
    }
  };

  const handleRemoveRoot = (path: string) => {
    if (isScanning || exportBusyRef.current) return;
    invalidateScanResult();
    setRoots((prev) => prev.filter((item) => item !== path));
  };

  const handleClearRoots = () => {
    if (isScanning || exportBusyRef.current || roots.length === 0) return;
    invalidateScanResult();
    setRoots([]);
  };

  const handleScan = async () => {
    if (isScanning || exportBusyRef.current || roots.length === 0) return;
    if (!isTauri()) {
      setError("当前为浏览器模式，无法执行本地扫描，请在 Tauri 桌面应用中运行。");
      return;
    }

    setIsScanning(true);
    setError(null);
    setExportMessage(null);
    setScanResult(null);
    setTreeQuery("");
    setSelectedSpecies(null);
    setSelectedMedia(null);
    try {
      const response = await invoke<ScanResponse>("scan", {
        request: {
          roots
        }
      });
      setScanResult(response);
    } catch (err) {
      setError(String(err));
    } finally {
      setIsScanning(false);
    }
  };

  const handleExport = async () => {
    if (!scanResult || exportBusyRef.current) return;
    if (!isTauri()) {
      setError("当前为浏览器模式，无法导出本地文件，请在 Tauri 桌面应用中运行。");
      return;
    }

    exportBusyRef.current = true;
    setIsChoosingExportPath(true);
    setError(null);
    setExportMessage(null);
    try {
      const destination = await save({
        title: "导出物种清单",
        defaultPath: exportFileName(),
        filters: [{ name: "Excel 工作簿", extensions: ["xlsx"] }]
      });
      setIsChoosingExportPath(false);
      if (!destination) return;

      setIsExporting(true);
      const response = await invoke<ExportResponse>("export_manifest", {
        request: {
          destination,
          exported_at: exportTimestamp(),
          scan: scanResult
        }
      });
      setExportMessage(
        `已导出 ${response.species_count} 个物种：${response.path}`
      );
    } catch (err) {
      setError(`导出失败：${String(err)}`);
    } finally {
      exportBusyRef.current = false;
      setIsChoosingExportPath(false);
      setIsExporting(false);
    }
  };

  const handleReveal = async () => {
    if (!selectedMedia) return;
    try {
      await invoke("reveal", { path: selectedMedia.path });
    } catch (err) {
      setError(String(err));
    }
  };

  const handleOpen = async (path: string) => {
    try {
      await invoke("open_file", { path });
    } catch (err) {
      setError(String(err));
    }
  };

  const handleThumbnailError = (path: string) => {
    setThumbnailErrorMap((prev) => {
      if (prev[path]) return prev;
      return { ...prev, [path]: true };
    });
  };

  return (
    <div className="app-shell">
      <header className="toolbar">
        <div>
          <h1>BirdIndex2</h1>
          <p>Local-only IOC indexing for bird photos and videos.</p>
        </div>
        <div className="toolbar-actions">
          <button
            className="primary"
            onClick={handleScan}
            disabled={isScanning || isExportBusy || roots.length === 0}
          >
            {isScanning ? "Scanning..." : "Scan"}
          </button>
        </div>
      </header>

      <section className="panel settings">
        <label>
          IOC 文件路径
          <input
            value={IOC_FILE_NAME}
            readOnly
            aria-readonly="true"
            title="桌面端使用随应用内置的 IOC 文件"
          />
        </label>
        <div className="root-picker">
          <div className="root-header">
            <div>
              <div className="root-title">扫描根目录</div>
              <div className="root-subtitle">{rootsLabel}</div>
            </div>
            <div className="root-actions">
              <button
                className="ghost"
                onClick={handlePickRoots}
                disabled={isScanning || isExportBusy}
              >
                选择目录
              </button>
              <button
                className="ghost"
                onClick={handleClearRoots}
                disabled={isScanning || isExportBusy || roots.length === 0}
              >
                清空
              </button>
            </div>
            <input
              ref={rootPickerInputRef}
              type="file"
              multiple
              style={{ display: "none" }}
              onChange={handleBrowserRootInputChange}
            />
          </div>
          <div className="root-list">
            {roots.length === 0 ? (
              <div className="empty">尚未选择任何目录</div>
            ) : (
              roots.map((root) => (
                <div key={root} className="root-item">
                  <span>{root}</span>
                  <button
                    className="ghost small"
                    onClick={() => handleRemoveRoot(root)}
                    disabled={isScanning || isExportBusy}
                  >
                    移除
                  </button>
                </div>
              ))
            )}
          </div>
        </div>
        <div className="scan-summary">
          {scanResult ? (
            <div className="stats">
              <span>扫描媒体：{scanResult.stats.total_media}</span>
              <span>图片：{scanResult.stats.total_images}</span>
              <span>视频：{scanResult.stats.total_videos}</span>
              <span>命中媒体：{scanResult.stats.matched_media}</span>
              <span>命中图片：{scanResult.stats.matched_images}</span>
              <span>命中视频：{scanResult.stats.matched_videos}</span>
              <span>命中物种：{scanResult.stats.matched_species}</span>
              <span>未命中媒体：{scanResult.stats.unmatched_media}</span>
              <span>IOC 物种数：{scanResult.total_species}</span>
            </div>
          ) : (
            <div className="stats">等待扫描</div>
          )}
          <button
            className="ghost export-button"
            onClick={handleExport}
            disabled={!scanResult || isScanning || isExportBusy}
            title={!scanResult ? "请先完成扫描" : "导出当前扫描中的全部物种"}
          >
            {isExporting ? "正在导出…" : "导出物种清单"}
          </button>
        </div>
        <div className="feedback" aria-live="polite">
          {error ? <div className="error">{error}</div> : null}
          {exportMessage ? (
            <div className="success">{exportMessage}</div>
          ) : null}
        </div>
      </section>

      <main className="main-grid">
        <section className="panel tree">
          <h2>分类树</h2>
          {scanResult ? (
            <>
              <div className="tree-search">
                <input
                  type="search"
                  value={treeQuery}
                  onChange={(event) => setTreeQuery(event.target.value)}
                  placeholder="搜索物种（中文或拉丁名）"
                  aria-label="搜索物种（中文或拉丁名）"
                />
              </div>
              <TreeView
                tree={scanResult.tree}
                query={treeQuery}
                onSelect={setSelectedSpecies}
              />
            </>
          ) : (
            <div className="empty">尚未生成分类树</div>
          )}
        </section>

        <section className="panel gallery">
          <h2>媒体</h2>
          {selectedSpecies ? (
            <div className="grid">
              {selectedSpecies.media_items.map((media) => (
                <button
                  key={media.path}
                  className={
                    selectedMedia?.path === media.path
                      ? "media-card active"
                      : "media-card"
                  }
                  onClick={() => setSelectedMedia(media)}
                  onDoubleClick={() => handleOpen(media.path)}
                  aria-label={`选择${media.media_type === "video" ? "视频" : "图片"}：${media.file_name}`}
                >
                  {media.media_type === "video" ? (
                    <div
                      className="media-preview video-placeholder"
                      role="img"
                      aria-label={`视频：${media.file_name}`}
                    >
                      <span className="video-icon" aria-hidden="true">
                        ▶
                      </span>
                      <span className="video-label">视频</span>
                    </div>
                  ) : thumbnailErrorMap[media.path] ? (
                    <div
                      className="media-preview media-fallback"
                      role="img"
                      aria-label={`无法预览：${media.file_name}`}
                    >
                      无法预览
                    </div>
                  ) : (
                    <img
                      className="media-preview"
                      src={toThumbnailSrc(media.path)}
                      alt={media.file_name}
                      onError={() => handleThumbnailError(media.path)}
                    />
                  )}
                  <span className="media-name">{media.file_name}</span>
                  <span className={`media-type-badge ${media.media_type}`}>
                    {media.media_type === "video" ? "视频" : "图片"}
                  </span>
                </button>
              ))}
            </div>
          ) : (
            <div className="empty">请选择一个物种</div>
          )}
        </section>

        <section className="panel meta">
          <h2>元数据</h2>
          {selectedSpecies ? (
            <div className="meta-block">
              <div className="meta-title">
                {selectedSpecies.chinese
                  ? `${selectedSpecies.chinese} ${selectedSpecies.latin}`
                  : selectedSpecies.latin}
              </div>
              <div className="meta-row">
                媒体总数：{selectedSpecies.media_count}
              </div>
              <div className="meta-row">
                文件名：{selectedMedia ? selectedMedia.file_name : "—"}
              </div>
              <div className="meta-row">
                媒体类型：
                {selectedMedia
                  ? selectedMedia.media_type === "video"
                    ? "视频"
                    : "图片"
                  : "—"}
              </div>
              <div className="meta-row">
                路径：{selectedMedia ? selectedMedia.path : "请选择一个媒体文件"}
              </div>
              <button
                className="ghost"
                onClick={handleReveal}
                disabled={!selectedMedia}
              >
                定位到文件夹
              </button>
            </div>
          ) : (
            <div className="empty">尚未选择物种</div>
          )}
        </section>
      </main>
    </div>
  );
}

function TreeView({
  tree,
  query,
  onSelect
}: {
  tree: TaxonTree;
  query: string;
  onSelect: (species: SpeciesNode) => void;
}) {
  const normalizedQuery = query.trim().toLowerCase();
  const hasQuery = normalizedQuery.length > 0;

  const filteredOrders = useMemo(() => {
    if (!normalizedQuery) return tree.orders;

    return tree.orders
      .map((order) => {
        const families = order.families
          .map((family) => {
            const genera = family.genera
              .map((genus) => {
                const species = genus.species.filter((item) => {
                  return (
                    item.chinese.toLowerCase().includes(normalizedQuery) ||
                    item.latin.toLowerCase().includes(normalizedQuery)
                  );
                });

                if (species.length === 0) return null;
                return { ...genus, species };
              })
              .filter((genus): genus is GenusNode => genus !== null);

            if (genera.length === 0) return null;
            return { ...family, genera };
          })
          .filter((family): family is FamilyNode => family !== null);

        if (families.length === 0) return null;
        return { ...order, families };
      })
      .filter((order): order is OrderNode => order !== null);
  }, [tree, normalizedQuery]);

  if (hasQuery && filteredOrders.length === 0) {
    return <div className="empty">未找到匹配物种</div>;
  }

  return (
    <div className="tree-root">
      {filteredOrders.map((order) => (
        <details key={order.name} open>
          <summary>
            {order.name} ({order.media_count})
          </summary>
          {order.families.map((family) => (
            <details key={family.name} className="level" open={hasQuery || undefined}>
              <summary>
                {family.name} ({family.media_count})
              </summary>
              {family.genera.map((genus) => (
                <details key={genus.name} className="level" open={hasQuery || undefined}>
                  <summary>
                    {genus.name} ({genus.media_count})
                  </summary>
                  <div className="species-list">
                    {genus.species.map((species) => (
                      <button
                        key={species.latin}
                        className="species"
                        onClick={() => onSelect(species)}
                      >
                        {species.chinese
                          ? `${species.chinese} ${species.latin}`
                          : species.latin}{" "}
                        ({species.media_count})
                      </button>
                    ))}
                  </div>
                </details>
              ))}
            </details>
          ))}
        </details>
      ))}
    </div>
  );
}
